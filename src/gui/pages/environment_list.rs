// Environment list page

use crate::gui::{ApiClient, AppState};
use crate::gui::components::{EnvironmentForm, ConfirmDialog};

pub struct EnvironmentListPage {
    selected_env: Option<String>,
    show_env_form: bool,
    env_form: EnvironmentForm,
    confirm_delete: ConfirmDialog,
    delete_target: Option<String>,
}

impl EnvironmentListPage {
    pub fn new() -> Self {
        Self {
            selected_env: None,
            show_env_form: false,
            env_form: EnvironmentForm::new(),
            confirm_delete: ConfirmDialog::new(),
            delete_target: None,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, _api_client: &ApiClient) {
        ui.heading("异地协同环境");
        
        ui.horizontal(|ui| {
            if ui.button("➕ 添加环境").clicked() {
                self.show_env_form = true;
                self.env_form = EnvironmentForm::new();
            }
            
            if ui.button("🔄 刷新").clicked() {
                // Refresh logic would go here
            }
        });
        
        ui.separator();
        
        // Environment list table
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("env_list")
                .striped(true)
                .num_columns(6)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    // Header
                    ui.strong("环境名称");
                    ui.strong("MQTT 地址");
                    ui.strong("文件服务器");
                    ui.strong("地区");
                    ui.strong("状态");
                    ui.strong("操作");
                    ui.end_row();
                    
                    // Rows
                    for env in &state.environments {
                        ui.label(&env.name);
                        
                        if let Some(host) = &env.mqtt_host {
                            ui.label(format!("{}:{}", host, env.mqtt_port.unwrap_or(1883)));
                        } else {
                            ui.label("-");
                        }
                        
                        if let Some(host) = &env.file_server_host {
                            ui.label(host);
                        } else {
                            ui.label("-");
                        }
                        
                        ui.label(&env.location);
                        
                        // Status indicator
                        ui.colored_label(egui::Color32::GRAY, "⚪ 未激活");
                        
                        ui.horizontal(|ui| {
                            if ui.small_button("编辑").clicked() {
                                self.env_form = EnvironmentForm::from_env(env);
                                self.show_env_form = true;
                            }
                            if ui.small_button("激活").clicked() {
                                // Activate environment
                            }
                            if ui.small_button("删除").clicked() {
                                self.delete_target = Some(env.id.clone());
                                self.confirm_delete.open("确认删除", "确定要删除这个环境吗？\n⚠️ 关联的站点也将被删除");
                            }
                        });
                        
                        ui.end_row();
                    }
                });
        });
        
        // Environment form dialog
        if self.show_env_form {
            egui::Window::new("环境配置")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    self.env_form.render(ui);
                    
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("保存").clicked() {
                            if self.env_form.validate() {
                                let _env = self.env_form.to_env();
                                // Save environment via API
                                self.show_env_form = false;
                            }
                        }
                        if ui.button("取消").clicked() {
                            self.show_env_form = false;
                        }
                    });
                });
        }
        
        // Delete confirmation dialog
        if self.confirm_delete.render(ui.ctx()) {
            if let Some(_env_id) = &self.delete_target {
                // Delete environment via API
                self.delete_target = None;
            }
        }
    }
}

impl Default for EnvironmentListPage {
    fn default() -> Self {
        Self::new()
    }
}
