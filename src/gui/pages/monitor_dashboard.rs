// Monitor dashboard page

use crate::gui::{ApiClient, AppState};
use crate::gui::api_client::SyncStatus;
use std::time::Instant;

pub struct MonitorDashboardPage {
    sync_status: Option<SyncStatus>,
    last_refresh: Instant,
    auto_refresh: bool,
}

impl MonitorDashboardPage {
    pub fn new() -> Self {
        Self {
            sync_status: None,
            last_refresh: Instant::now(),
            auto_refresh: true,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, _api_client: &ApiClient) {
        ui.heading("实时监控");
        
        ui.horizontal(|ui| {
            if ui.button("🔄 刷新").clicked() {
                // Refresh status
                self.last_refresh = Instant::now();
            }
            
            ui.checkbox(&mut self.auto_refresh, "自动刷新 (5秒)");
            
            ui.label(format!(
                "上次刷新: {} 秒前",
                self.last_refresh.elapsed().as_secs()
            ));
        });
        
        ui.separator();
        
        // Status cards
        ui.horizontal(|ui| {
            if let Some(status) = &self.sync_status {
                self.render_status_card(ui, "运行状态", if status.running {
                    if status.paused { "⏸️ 暂停中" } else { "🟢 运行中" }
                } else {
                    "⚪ 已停止"
                });
                
                self.render_status_card(ui, "MQTT 连接", if status.mqtt_connected {
                    "🟢 已连接"
                } else {
                    "🔴 断开"
                });
                
                self.render_status_card(ui, "队列大小", &status.queue_size.to_string());
                self.render_status_card(ui, "活跃任务", &status.active_tasks.to_string());
            } else {
                self.render_status_card(ui, "运行状态", "⚪ 未知");
                self.render_status_card(ui, "MQTT 连接", "⚪ 未知");
                self.render_status_card(ui, "队列大小", "0");
                self.render_status_card(ui, "活跃任务", "0");
            }
        });
        
        ui.separator();
        
        // Task list
        ui.heading("同步任务");
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("task_list")
                .striped(true)
                .num_columns(5)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    // Header
                    ui.strong("文件名");
                    ui.strong("源环境");
                    ui.strong("目标站点");
                    ui.strong("状态");
                    ui.strong("进度");
                    ui.end_row();
                    
                    // Rows
                    for task in &state.sync_tasks {
                        ui.label(&task.file_name);
                        ui.label(&task.source_env);
                        ui.label(&task.target_site);
                        
                        let (color, text) = match task.status.as_str() {
                            "pending" => (egui::Color32::GRAY, "⏸️ 等待中"),
                            "running" => (egui::Color32::BLUE, "🟢 运行中"),
                            "completed" => (egui::Color32::GREEN, "✅ 完成"),
                            "failed" => (egui::Color32::RED, "❌ 失败"),
                            _ => (egui::Color32::GRAY, "未知"),
                        };
                        ui.colored_label(color, text);
                        
                        let progress = task.progress as f32 / 100.0;
                        ui.add(egui::ProgressBar::new(progress).text(format!("{}%", task.progress)));
                        
                        ui.end_row();
                    }
                });
        });
        
        // Auto refresh
        if self.auto_refresh && self.last_refresh.elapsed().as_secs() >= 5 {
            ui.ctx().request_repaint();
        }
    }
    
    fn render_status_card(&self, ui: &mut egui::Ui, title: &str, value: &str) {
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(title);
                    ui.heading(value);
                });
            });
    }
}

impl Default for MonitorDashboardPage {
    fn default() -> Self {
        Self::new()
    }
}
