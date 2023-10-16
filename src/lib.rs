#![warn(clippy::all, clippy::pedantic, rust_2018_idioms)]
#![allow(clippy::missing_errors_doc)]

mod app;
mod loader;
mod tab;
pub use app::McapViewer;
