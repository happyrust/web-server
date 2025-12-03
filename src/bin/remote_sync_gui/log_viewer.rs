use super::app::RemoteSyncApp;
use eframe::egui;

#[derive(Default)]
pub struct LogViewerState {}

pub fn render(ui: &mut egui::Ui, _state: &mut LogViewerState, app: &mut RemoteSyncApp) {
    ui.heading("📝 日志查看器");
    ui.label("日志查看功能待实现...");
}
