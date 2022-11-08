{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    rust-overlay = {
       url = "github:oxalica/rust-overlay";
       inputs = {
         nixpkgs.follows = "nixpkgs";
         flake-utils.follows = "flake-utils";
       };
     };  
   };

  outputs = { self, nixpkgs, crane, flake-utils, advisory-db, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;


        rustTC = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rustfmt" "rust-analyzer" "rust-src"];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustTC;
        src = craneLib.cleanCargoSource ./.;

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
        };

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        copper = craneLib.buildPackage {
          inherit cargoArtifacts src;
        };
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit copper;

          copper-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          };

          copper-doc = craneLib.cargoDoc {
            inherit cargoArtifacts src;
          };

          # Check formatting
          copper-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          copper-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `copper` if you do not want
          # the tests to run twice
          copper-nextest = craneLib.cargoNextest {
            inherit cargoArtifacts src;
            partitions = 1;
            partitionType = "count";
          };
        } // lib.optionalAttrs (system == "x86_64-linux") {
          copper-coverage = craneLib.cargoTarpaulin {
            inherit cargoArtifacts src;
          };
        };

        packages.default = copper;

        apps.default = flake-utils.lib.mkApp {
          drv = copper;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks;

          # Extra inputs can be added here
          nativeBuildInputs = with pkgs; [
            rustTC
            rnix-lsp
          ];
        };
      });
}
