use super::app::RemoteSyncApp;
use eframe::egui;

#[derive(Default)]
pub struct TestToolsState {}

pub fn render(ui: &mut egui::Ui, _state: &mut TestToolsState, app: &mut RemoteSyncApp) {
    ui.heading("🧪 测试工具");
    ui.label("测试工具功能待实现...");
}
