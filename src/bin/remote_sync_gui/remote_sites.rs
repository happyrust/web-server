use super::app::RemoteSyncApp;
use eframe::egui;

#[derive(Default)]
pub struct RemoteSitesState {}

pub fn render(ui: &mut egui::Ui, _state: &mut RemoteSitesState, app: &mut RemoteSyncApp) {
    ui.heading("🌐 远程站点管理");
    ui.label("站点管理功能待实现...");
}
