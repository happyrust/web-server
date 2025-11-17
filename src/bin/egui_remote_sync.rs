// egui Remote Sync UI - Main entry point

use aios_database::gui::EguiRemoteSyncApp;

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::init();
    
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("egui 异地协同运维界面"),
        ..Default::default()
    };
    
    eframe::run_native(
        "egui_remote_sync",
        native_options,
        Box::new(|cc| Ok(Box::new(EguiRemoteSyncApp::new(cc)))),
    )
}
