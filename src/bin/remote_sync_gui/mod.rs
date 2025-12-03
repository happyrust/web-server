// Remote Sync GUI - Main Module
// egui-based graphical interface for managing remote synchronization test environment

mod app;
mod config;
mod dashboard;
mod db;
mod env_config;
mod file_browser;
mod log_viewer;
mod remote_sites;
mod service_manager;
mod sync_control;
mod test_tools;
mod types;
mod web_server;

pub use app::RemoteSyncApp;
pub use types::*;

use anyhow::Result;
use eframe::egui;

/// Run the GUI application
pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1024.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "异地协同管理工具",
        options,
        Box::new(|cc| Box::new(RemoteSyncApp::new(cc))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))
}

/// Load application icon
fn load_icon() -> eframe::IconData {
    // TODO: Load actual icon from assets
    // For now, create a simple default icon
    eframe::IconData {
        rgba: vec![255; 32 * 32 * 4],
        width: 32,
        height: 32,
    }
}
