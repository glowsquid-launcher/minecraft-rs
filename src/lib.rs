#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]
#![feature(async_fn_in_trait)]

pub mod assets;
pub mod auth;
pub mod downloader;
pub mod launcher;
pub mod merger;
pub mod parser;
