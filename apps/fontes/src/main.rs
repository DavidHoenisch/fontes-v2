#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
#[command(about = "Fontes — an offline Bible study desk")]
struct Args {
    /// Explicitly trusted local reading/study SQLite snapshot. No automatic network access.
    #[arg(long)]
    library: Option<PathBuf>,
    /// Separate writable state directory. Defaults to this platform's local app-data folder.
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Start with the compact touch layout for desktop mobile-preview testing.
    #[arg(long)]
    mobile: bool,
}
fn main() -> eframe::Result {
    let args = Args::parse();
    let data_dir = args
        .data_dir
        .or_else(|| {
            directories::ProjectDirs::from("org", "Fontes", "Fontes")
                .map(|d| d.data_local_dir().to_path_buf())
        })
        .expect("Supply --data-dir when no platform data directory is available");
    let size = if args.mobile {
        [390.0, 844.0]
    } else {
        [1440.0, 960.0]
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Fontes · Study desk")
            .with_inner_size(size)
            .with_min_inner_size([320.0, 480.0])
            .with_app_id("org.fontes.app"),
        ..Default::default()
    };
    eframe::run_native(
        "Fontes",
        options,
        Box::new(move |cc| {
            Ok(Box::new(fontes_ui_egui::FontesApp::new(
                cc,
                args.library,
                data_dir,
            )))
        }),
    )
}
