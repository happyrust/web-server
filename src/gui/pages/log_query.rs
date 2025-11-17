// Log query page

use crate::gui::{ApiClient, AppState};
use crate::gui::api_client::LogFilters;
use crate::gui::state::SyncLog;

pub struct LogQueryPage {
    filters: LogFilters,
    logs: Vec<SyncLog>,
    selected_log: Option<String>,
    show_log_detail: bool,
    page: usize,
    page_size: usize,
    total: usize,
}

impl LogQueryPage {
    pub fn new() -> Self {
        Self {
            filters: LogFilters {
                env_id: None,
                site_id: None,
                status: None,
                start_time: None,
                end_time: None,
                limit: Some(100),
            },
            logs: Vec::new(),
            selected_log: None,
            show_log_detail: false,
            page: 0,
            page_size: 100,
            total: 0,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, _api_client: &ApiClient) {
        ui.heading("日志查询");
        
        // Filter form
        egui::Grid::new("log_filters")
            .num_columns(4)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("环境");
                egui::ComboBox::from_id_source("env_filter")
                    .selected_text(self.filters.env_id.as_deref().unwrap_or("全部"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filters.env_id, None, "全部");
                        for env in &state.environments {
                            ui.selectable_value(&mut self.filters.env_id, Some(env.id.clone()), &env.name);
                        }
                    });
                
                ui.label("站点");
                egui::ComboBox::from_id_source("site_filter")
                    .selected_text(self.filters.site_id.as_deref().unwrap_or("全部"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filters.site_id, None, "全部");
                        for site in &state.sites {
                            ui.selectable_value(&mut self.filters.site_id, Some(site.id.clone()), &site.name);
                        }
                    });
                
                ui.label("状态");
                egui::ComboBox::from_id_source("status_filter")
                    .selected_text(self.filters.status.as_deref().unwrap_or("全部"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filters.status, None, "全部");
                        ui.selectable_value(&mut self.filters.status, Some("pending".to_string()), "待处理");
                        ui.selectable_value(&mut self.filters.status, Some("running".to_string()), "运行中");
                        ui.selectable_value(&mut self.filters.status, Some("completed".to_string()), "完成");
                        ui.selectable_value(&mut self.filters.status, Some("failed".to_string()), "失败");
                    });
                
                if ui.button("🔍 查询").clicked() {
                    // Query logs
                }
                
                ui.end_row();
            });
        
        ui.separator();
        
        // Log list
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("log_list")
                .striped(true)
                .num_columns(6)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    // Header
                    ui.strong("时间");
                    ui.strong("文件路径");
                    ui.strong("源环境");
                    ui.strong("目标站点");
                    ui.strong("状态");
                    ui.strong("操作");
                    ui.end_row();
                    
                    // Rows
                    for log in &self.logs {
                        ui.label(&log.created_at);
                        ui.label(&log.file_path);
                        ui.label(&log.source_env);
                        ui.label(&log.target_site);
                        
                        let (color, text) = match log.status.as_str() {
                            "completed" => (egui::Color32::GREEN, "✅ 完成"),
                            "failed" => (egui::Color32::RED, "❌ 失败"),
                            _ => (egui::Color32::GRAY, "⏸️ 其他"),
                        };
                        ui.colored_label(color, text);
                        
                        if ui.small_button("详情").clicked() {
                            self.selected_log = Some(log.id.clone());
                            self.show_log_detail = true;
                        }
                        
                        ui.end_row();
                    }
                });
        });
        
        // Pagination controls
        ui.horizontal(|ui| {
            if ui.button("◀ 上一页").clicked() && self.page > 0 {
                self.page -= 1;
            }
            
            ui.label(format!("第 {} 页 / 共 {} 条", self.page + 1, self.total));
            
            if ui.button("下一页 ▶").clicked() && (self.page + 1) * self.page_size < self.total {
                self.page += 1;
            }
            
            #[cfg(feature = "gui")]
            if ui.button("📤 导出 CSV").clicked() {
                self.export_csv();
            }
        });
        
        // Log detail dialog
        if self.show_log_detail {
            egui::Window::new("日志详情")
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    if let Some(log_id) = &self.selected_log {
                        if let Some(log) = self.logs.iter().find(|l| &l.id == log_id) {
                            egui::Grid::new("log_detail")
                                .num_columns(2)
                                .spacing([10.0, 10.0])
                                .show(ui, |ui| {
                                    ui.label("任务 ID:");
                                    ui.label(&log.task_id);
                                    ui.end_row();
                                    
                                    ui.label("文件路径:");
                                    ui.label(&log.file_path);
                                    ui.end_row();
                                    
                                    ui.label("文件大小:");
                                    ui.label(format!("{} bytes", log.file_size));
                                    ui.end_row();
                                    
                                    ui.label("记录数:");
                                    ui.label(format!("{}", log.record_count));
                                    ui.end_row();
                                    
                                    ui.label("开始时间:");
                                    ui.label(&log.started_at);
                                    ui.end_row();
                                    
                                    if let Some(completed_at) = &log.completed_at {
                                        ui.label("完成时间:");
                                        ui.label(completed_at);
                                        ui.end_row();
                                    }
                                    
                                    if let Some(error) = &log.error_message {
                                        ui.label("错误信息:");
                                        ui.colored_label(egui::Color32::RED, error);
                                        ui.end_row();
                                    }
                                });
                        }
                    }
                    
                    if ui.button("关闭").clicked() {
                        self.show_log_detail = false;
                    }
                });
        }
    }
    
    #[cfg(feature = "gui")]
    fn export_csv(&self) {
        use std::io::Write;
        
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .save_file()
        {
            if let Ok(mut file) = std::fs::File::create(path) {
                let _ = writeln!(file, "时间,文件路径,源环境,目标站点,状态");
                
                for log in &self.logs {
                    let _ = writeln!(
                        file,
                        "{},{},{},{},{}",
                        log.created_at,
                        log.file_path,
                        log.source_env,
                        log.target_site,
                        log.status
                    );
                }
            }
        }
    }
}

impl Default for LogQueryPage {
    fn default() -> Self {
        Self::new()
    }
}
