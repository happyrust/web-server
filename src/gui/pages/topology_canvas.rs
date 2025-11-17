// Topology canvas page for visual configuration

use crate::gui::{ApiClient, AppState};
use crate::gui::state::{RemoteSyncEnv, RemoteSyncSite, TopologyData, TopologyConnection};
use crate::gui::canvas::{TopologyCanvas, CanvasMode};
use crate::gui::components::ToastManager;

pub struct TopologyCanvasPage {
    canvas: TopologyCanvas,
    selected_node: Option<usize>,
    show_node_config: bool,
    mode: CanvasMode,
}

impl TopologyCanvasPage {
    pub fn new() -> Self {
        Self {
            canvas: TopologyCanvas::new(),
            selected_node: None,
            show_node_config: false,
            mode: CanvasMode::Select,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, _state: &mut AppState, _api_client: &ApiClient) {
        ui.heading("拓扑配置");
        
        // Toolbar
        ui.horizontal(|ui| {
            if ui.selectable_label(self.mode == CanvasMode::Select, "🖱️ 选择").clicked() {
                self.mode = CanvasMode::Select;
            }
            if ui.selectable_label(self.mode == CanvasMode::AddEnvironment, "➕ 添加环境").clicked() {
                self.mode = CanvasMode::AddEnvironment;
            }
            if ui.selectable_label(self.mode == CanvasMode::AddSite, "➕ 添加站点").clicked() {
                self.mode = CanvasMode::AddSite;
            }
            if ui.selectable_label(self.mode == CanvasMode::Connect, "🔗 连接").clicked() {
                self.mode = CanvasMode::Connect;
            }
            
            ui.separator();
            
            if ui.button("🔄 自动布局").clicked() {
                self.canvas.auto_layout();
            }
            if ui.button("💾 保存拓扑").clicked() {
                // Save topology to backend
            }
            if ui.button("📥 导入 JSON").clicked() {
                // Import topology from JSON file
            }
            if ui.button("📤 导出 JSON").clicked() {
                // Export topology to JSON file
            }
        });
        
        ui.separator();
        
        // Canvas area
        egui::Frame::canvas(ui.style())
            .show(ui, |ui| {
                self.canvas.render(ui, self.mode);
            });
    }
}

impl Default for TopologyCanvasPage {
    fn default() -> Self {
        Self::new()
    }
}
