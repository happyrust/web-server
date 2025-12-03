use super::app::RemoteSyncApp;
use eframe::egui;

#[derive(Default)]
pub struct SyncControlState {}

pub fn render(ui: &mut egui::Ui, _state: &mut SyncControlState, app: &mut RemoteSyncApp) {
    ui.heading("🔄 同步控制");
    ui.label("同步控制功能待实现...");
}
