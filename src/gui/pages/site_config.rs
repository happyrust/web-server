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
        
        // Site cards grid
        egui::ScrollArea::vertical().show(ui, |ui| {
            if state.sites.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("暂无站点");
                    ui.label("点击上方 ➕ 添加站点 按钮创建新站点");
                });
            } else {
                // Calculate cards per row based on available width
                let available_width = ui.available_width();
                let card_width = 350.0;
                let spacing = 10.0;
                let cards_per_row = ((available_width + spacing) / (card_width + spacing)).floor().max(1.0) as usize;
                
                // Render cards in grid
                let mut current_row = 0;
                ui.horizontal_wrapped(|ui| {
                    for (idx, site) in state.sites.iter().enumerate() {
                        if idx > 0 && idx % cards_per_row == 0 {
                            current_row += 1;
                        }
                        
                        self.render_site_card(ui, site, state);
                        
                        // Add spacing between cards
                        if (idx + 1) % cards_per_row != 0 && idx < state.sites.len() - 1 {
                            ui.add_space(spacing);
                        }
                    }
                });
            }
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
    
    fn render_site_card(&mut self, ui: &mut egui::Ui, site: &RemoteSyncSite, state: &AppState) {
        let card_width = 350.0;
        
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(248, 250, 252))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(226, 232, 240)))
            .rounding(egui::Rounding::same(8))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(card_width);
                
                // Header: Site name and status
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(&site.name);
                        
                        // Environment badge
                        let env_name = state.environments
                            .iter()
                            .find(|e| e.id == site.env_id)
                            .map(|e| e.name.as_str())
                            .unwrap_or("未知");
                        
                        ui.horizontal(|ui| {
                            ui.label("🏷️");
                            ui.label(env_name);
                        });
                    });
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        // Status indicator
                        ui.colored_label(egui::Color32::from_rgb(156, 163, 175), "⚪ 未部署");
                    });
                });
                
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Site details
                egui::Grid::new(format!("site_card_{}", site.id))
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        // Location
                        ui.label("📍");
                        ui.label(&site.location);
                        ui.end_row();
                        
                        // HTTP Host
                        if let Some(host) = &site.http_host {
                            ui.label("🌐");
                            ui.label(host);
                            ui.end_row();
                        }
                        
                        // Database numbers
                        if let Some(dbnums) = &site.dbnums {
                            ui.label("💾");
                            ui.label(format!("DB: {}", dbnums));
                            ui.end_row();
                        }
                        
                        // Created time
                        ui.label("🕐");
                        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&site.created_at) {
                            ui.label(created.format("%Y-%m-%d %H:%M").to_string());
                        } else {
                            ui.label(&site.created_at);
                        }
                        ui.end_row();
                    });
                
                // Notes preview (if exists)
                if let Some(notes) = &site.notes {
                    if !notes.trim().is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        
                        // Show first line of notes
                        let first_line = notes.lines().next().unwrap_or("");
                        if !first_line.is_empty() {
                            ui.label(egui::RichText::new(format!("📝 {}", first_line))
                                .color(egui::Color32::from_rgb(100, 116, 139))
                                .size(12.0));
                        }
                    }
                }
                
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("✏️ 编辑").clicked() {
                        self.site_form = SiteConfigForm::from_site(site);
                        self.show_site_form = true;
                    }
                    
                    if ui.button("🔌 测试").clicked() {
                        // Test connection
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑️ 删除").clicked() {
                            self.delete_target = Some(site.id.clone());
                            self.confirm_delete.open("确认删除", "确定要删除这个站点吗？");
                        }
                    });
                });
            });
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
