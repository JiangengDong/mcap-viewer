#![warn(clippy::all, clippy::pedantic, rust_2018_idioms)]
#![allow(clippy::missing_errors_doc)]

mod app;
mod cache;
mod loader;
mod tab;
mod widgets;

pub use app::McapViewer;
