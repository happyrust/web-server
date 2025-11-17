// Site configuration page

use crate::gui::{ApiClient, AppState};
use crate::gui::state::RemoteSyncSite;
use crate::gui::components::ConfirmDialog;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfigForm {
    pub id: Option<String>,
    pub env_id: String,
    pub name: String,
    pub location: String,
    pub http_host: String,
    pub dbnums: String,
    pub notes: String,
    pub e3d_dir: String,
    pub surreal_host: String,
    pub surreal_port: String,
    pub surreal_namespace: String,
    pub surreal_database: String,
    pub surreal_username: String,
    pub surreal_password: String,
}

impl SiteConfigForm {
    pub fn new() -> Self {
        Self {
            id: None,
            env_id: String::new(),
            name: String::new(),
            location: String::new(),
            http_host: String::new(),
            dbnums: String::new(),
            notes: String::new(),
            e3d_dir: String::new(),
            surreal_host: "localhost".to_string(),
            surreal_port: "8000".to_string(),
            surreal_namespace: "default".to_string(),
            surreal_database: "default".to_string(),
            surreal_username: "root".to_string(),
            surreal_password: String::new(),
        }
    }
    
    pub fn from_site(site: &RemoteSyncSite) -> Self {
        Self {
            id: Some(site.id.clone()),
            env_id: site.env_id.clone(),
            name: site.name.clone(),
            location: site.location.clone(),
            http_host: site.http_host.clone().unwrap_or_default(),
            dbnums: site.dbnums.clone().unwrap_or_default(),
            notes: site.notes.clone().unwrap_or_default(),
            e3d_dir: String::new(), // TODO: Parse from notes or separate field
            surreal_host: "localhost".to_string(),
            surreal_port: "8000".to_string(),
            surreal_namespace: "default".to_string(),
            surreal_database: "default".to_string(),
            surreal_username: "root".to_string(),
            surreal_password: String::new(),
        }
    }
    
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("站点名称不能为空".to_string());
        }
        
        if self.env_id.trim().is_empty() {
            return Err("必须选择环境".to_string());
        }
        
        if !self.surreal_port.is_empty() {
            if self.surreal_port.parse::<u16>().is_err() {
                return Err("SurrealDB 端口格式不正确".to_string());
            }
        }
        
        Ok(())
    }
    
    pub fn to_site(&self) -> RemoteSyncSite {
        // Encode e3d_dir and surreal config in notes field
        let config_json = serde_json::json!({
            "e3d_dir": self.e3d_dir,
            "surreal": {
                "host": self.surreal_host,
                "port": self.surreal_port,
                "namespace": self.surreal_namespace,
                "database": self.surreal_database,
                "username": self.surreal_username,
                "password": self.surreal_password,
            }
        });
        
        let notes = format!("{}\n\nConfig: {}", self.notes, config_json);
        
        RemoteSyncSite {
            id: self.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            env_id: self.env_id.clone(),
            name: self.name.clone(),
            location: self.location.clone(),
            http_host: if self.http_host.is_empty() { None } else { Some(self.http_host.clone()) },
            dbnums: if self.dbnums.is_empty() { None } else { Some(self.dbnums.clone()) },
            notes: Some(notes),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub struct SiteConfigPage {
    selected_site: Option<String>,
    show_site_form: bool,
    site_form: SiteConfigForm,
    confirm_delete: ConfirmDialog,
    delete_target: Option<String>,
}

impl SiteConfigPage {
    pub fn new() -> Self {
        Self {
            selected_site: None,
            show_site_form: false,
            site_form: SiteConfigForm::new(),
            confirm_delete: ConfirmDialog::new(),
            delete_target: None,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, _api_client: &ApiClient) {
        ui.heading("站点配置");
        
        ui.horizontal(|ui| {
            if ui.button("➕ 添加站点").clicked() {
                self.show_site_form = true;
                self.site_form = SiteConfigForm::new();
            }
            
            if ui.button("🔄 刷新").clicked() {
                // Refresh logic would go here
            }
        });
        
        ui.separator();
        
        // Site list table
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("site_list")
                .striped(true)
                .num_columns(7)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    // Header
                    ui.strong("站点名称");
                    ui.strong("环境");
                    ui.strong("HTTP 地址");
                    ui.strong("地区");
                    ui.strong("数据库编号");
                    ui.strong("状态");
                    ui.strong("操作");
                    ui.end_row();
                    
                    // Rows
                    for site in &state.sites {
                        ui.label(&site.name);
                        
                        // Find environment name
                        let env_name = state.environments
                            .iter()
                            .find(|e| e.id == site.env_id)
                            .map(|e| e.name.as_str())
                            .unwrap_or("未知");
                        ui.label(env_name);
                        
                        if let Some(host) = &site.http_host {
                            ui.label(host);
                        } else {
                            ui.label("-");
                        }
                        
                        ui.label(&site.location);
                        
                        if let Some(dbnums) = &site.dbnums {
                            ui.label(dbnums);
                        } else {
                            ui.label("-");
                        }
                        
                        // Status indicator
                        ui.colored_label(egui::Color32::GRAY, "⚪ 未部署");
                        
                        ui.horizontal(|ui| {
                            if ui.small_button("编辑").clicked() {
                                self.site_form = SiteConfigForm::from_site(site);
                                self.show_site_form = true;
                            }
                            if ui.small_button("测试").clicked() {
                                // Test connection
                            }
                            if ui.small_button("删除").clicked() {
                                self.delete_target = Some(site.id.clone());
                                self.confirm_delete.open("确认删除", "确定要删除这个站点吗？");
                            }
                        });
                        
                        ui.end_row();
                    }
                });
        });
        
        // Site form dialog
        if self.show_site_form {
            egui::Window::new("站点配置")
                .collapsible(false)
                .resizable(true)
                .default_width(600.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(500.0)
                        .show(ui, |ui| {
                            self.render_site_form(ui, state);
                        });
                    
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("💾 保存").clicked() {
                            match self.site_form.validate() {
                                Ok(_) => {
                                    let _site = self.site_form.to_site();
                                    // Save site via API
                                    self.show_site_form = false;
                                }
                                Err(err) => {
                                    // Show error toast
                                    eprintln!("验证失败: {}", err);
                                }
                            }
                        }
                        if ui.button("❌ 取消").clicked() {
                            self.show_site_form = false;
                        }
                    });
                });
        }
        
        // Delete confirmation dialog
        if self.confirm_delete.render(ui.ctx()) {
            if let Some(_site_id) = &self.delete_target {
                // Delete site via API
                self.delete_target = None;
            }
        }
    }
    
    fn render_site_form(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.heading("基本信息");
        
        egui::Grid::new("site_form_basic")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("站点名称 *:");
                ui.text_edit_singleline(&mut self.site_form.name);
                ui.end_row();
                
                ui.label("所属环境 *:");
                egui::ComboBox::from_id_source("site_env")
                    .selected_text(
                        state.environments
                            .iter()
                            .find(|e| e.id == self.site_form.env_id)
                            .map(|e| e.name.as_str())
                            .unwrap_or("请选择环境")
                    )
                    .show_ui(ui, |ui| {
                        for env in &state.environments {
                            ui.selectable_value(&mut self.site_form.env_id, env.id.clone(), &env.name);
                        }
                    });
                ui.end_row();
                
                ui.label("HTTP 地址:");
                ui.text_edit_singleline(&mut self.site_form.http_host);
                ui.end_row();
                
                ui.label("地区:");
                ui.text_edit_singleline(&mut self.site_form.location);
                ui.end_row();
                
                ui.label("数据库编号:");
                ui.text_edit_singleline(&mut self.site_form.dbnums);
                ui.end_row();
                
                ui.label("备注:");
                ui.text_edit_multiline(&mut self.site_form.notes);
                ui.end_row();
            });
        
        ui.add_space(10.0);
        ui.separator();
        ui.heading("E3D 配置");
        
        egui::Grid::new("site_form_e3d")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("E3D 监听目录:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.site_form.e3d_dir);
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.site_form.e3d_dir = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.end_row();
            });
        
        ui.add_space(10.0);
        ui.separator();
        ui.heading("SurrealDB 配置");
        
        egui::Grid::new("site_form_surreal")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("SurrealDB 主机:");
                ui.text_edit_singleline(&mut self.site_form.surreal_host);
                ui.end_row();
                
                ui.label("SurrealDB 端口:");
                ui.text_edit_singleline(&mut self.site_form.surreal_port);
                ui.end_row();
                
                ui.label("命名空间:");
                ui.text_edit_singleline(&mut self.site_form.surreal_namespace);
                ui.end_row();
                
                ui.label("数据库:");
                ui.text_edit_singleline(&mut self.site_form.surreal_database);
                ui.end_row();
                
                ui.label("用户名:");
                ui.text_edit_singleline(&mut self.site_form.surreal_username);
                ui.end_row();
                
                ui.label("密码:");
                ui.add(egui::TextEdit::singleline(&mut self.site_form.surreal_password).password(true));
                ui.end_row();
            });
    }
}

impl Default for SiteConfigPage {
    fn default() -> Self {
        Self::new()
    }
}
