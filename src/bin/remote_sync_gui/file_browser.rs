use super::app::RemoteSyncApp;
use eframe::egui;

#[derive(Default)]
pub struct FileBrowserState {}

pub fn render(ui: &mut egui::Ui, _state: &mut FileBrowserState, app: &mut RemoteSyncApp) {
    ui.heading("📁 文件浏览");
    ui.label("文件浏览功能待实现...");
}
