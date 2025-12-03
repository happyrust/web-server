// Remote Sync GUI - Main entry point

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

use anyhow::Result;

fn main() -> Result<()> {
    // Set up logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("Starting Remote Sync GUI...");

    // Run the egui application
    app::RemoteSyncApp::run()
}

impl app::RemoteSyncApp {
    pub fn run() -> Result<()> {
        let options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 800.0])
                .with_min_inner_size([1024.0, 600.0])
                .with_title("异地协同管理工具"),
            ..Default::default()
        };

        eframe::run_native(
            "异地协同管理工具",
            options,
            Box::new(|cc| Ok(Box::new(app::RemoteSyncApp::new(cc)))),
        )
        .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))
    }
}
