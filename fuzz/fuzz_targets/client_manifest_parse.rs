#![no_main]

use libfuzzer_sys::fuzz_target;
extern crate copper;
extern crate serde_json;

use copper::assets::client::Manifest;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<Manifest>(s);
    }
});
