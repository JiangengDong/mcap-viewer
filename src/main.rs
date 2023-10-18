#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::path::PathBuf;

use clap::Parser;

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    path: Option<PathBuf>,
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let cli = Cli::parse();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        ..Default::default()
    };
    eframe::run_native(
        "mcap viewer",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let viewer = if let Some(path) = cli.path {
                mcap_viewer::McapViewer::from_path(cc, path).unwrap()
            } else {
                mcap_viewer::McapViewer::new(cc)
            };
            Box::new(viewer)
        }),
    )
}
