// Configuration editor page

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum EditMode {
    Form,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOptionConfig {
    pub db_path: String,
    pub surreal_host: String,
    pub surreal_port: u16,
    pub web_host: String,
    pub web_port: u16,
    pub log_level: String,
    pub max_connections: u32,
}

pub struct ConfigEditorPage {
    config: DbOptionConfig,
    config_text: String,
    edit_mode: EditMode,
    validation_errors: Vec<String>,
}

impl ConfigEditorPage {
    pub fn new() -> Self {
        let config = Self::load_config();
        let config_text = toml::to_string_pretty(&config).unwrap_or_default();
        
        Self {
            config,
            config_text,
            edit_mode: EditMode::Form,
            validation_errors: Vec::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("配置编辑");
        
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.edit_mode, EditMode::Form, "📝 表单模式");
            ui.selectable_value(&mut self.edit_mode, EditMode::Text, "📄 文本模式");
            
            ui.separator();
            
            if ui.button("💾 保存配置").clicked() {
                self.save_config();
            }
            
            if ui.button("🔄 重新加载").clicked() {
                self.reload_config();
            }
            
            if ui.button("⚠️ 恢复默认").clicked() {
                self.reset_to_default();
            }
        });
        
        ui.separator();
        
        if !self.validation_errors.is_empty() {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(255, 200, 200))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.strong("⚠️ 配置验证错误:");
                    for error in &self.validation_errors {
                        ui.label(format!("• {}", error));
                    }
                });
            ui.separator();
        }
        
        match self.edit_mode {
            EditMode::Form => self.render_form_editor(ui),
            EditMode::Text => self.render_text_editor(ui),
        }
    }
    
    fn render_form_editor(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("数据库配置");
            egui::Grid::new("db_config_grid")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("数据库路径:");
                    ui.text_edit_singleline(&mut self.config.db_path);
                    ui.end_row();
                    
                    ui.label("SurrealDB 主机:");
                    ui.text_edit_singleline(&mut self.config.surreal_host);
                    ui.end_row();
                    
                    ui.label("SurrealDB 端口:");
                    ui.add(egui::DragValue::new(&mut self.config.surreal_port).clamp_range(1..=65535));
                    ui.end_row();
                });
            
            ui.add_space(10.0);
            ui.heading("Web Server 配置");
            egui::Grid::new("web_config_grid")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("监听地址:");
                    ui.text_edit_singleline(&mut self.config.web_host);
                    ui.end_row();
                    
                    ui.label("监听端口:");
                    ui.add(egui::DragValue::new(&mut self.config.web_port).clamp_range(1..=65535));
                    ui.end_row();
                });
            
            ui.add_space(10.0);
            ui.heading("其他配置");
            egui::Grid::new("other_config_grid")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("日志级别:");
                    egui::ComboBox::from_id_source("log_level")
                        .selected_text(&self.config.log_level)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.config.log_level, "trace".to_string(), "Trace");
                            ui.selectable_value(&mut self.config.log_level, "debug".to_string(), "Debug");
                            ui.selectable_value(&mut self.config.log_level, "info".to_string(), "Info");
                            ui.selectable_value(&mut self.config.log_level, "warn".to_string(), "Warn");
                            ui.selectable_value(&mut self.config.log_level, "error".to_string(), "Error");
                        });
                    ui.end_row();
                    
                    ui.label("最大连接数:");
                    ui.add(egui::DragValue::new(&mut self.config.max_connections).clamp_range(1..=1000));
                    ui.end_row();
                });
        });
    }
    
    fn render_text_editor(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.config_text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
            );
        });
    }
    
    fn save_config(&mut self) {
        self.validation_errors.clear();
        
        if self.edit_mode == EditMode::Text {
            match toml::from_str::<DbOptionConfig>(&self.config_text) {
                Ok(config) => {
                    self.config = config;
                }
                Err(e) => {
                    self.validation_errors.push(format!("TOML 解析错误: {}", e));
                    return;
                }
            }
        }
        
        if self.config.surreal_port == 0 {
            self.validation_errors.push("SurrealDB 端口不能为 0".to_string());
        }
        
        if self.config.web_port == 0 {
            self.validation_errors.push("Web Server 端口不能为 0".to_string());
        }
        
        if !self.validation_errors.is_empty() {
            return;
        }
        
        let toml_content = toml::to_string_pretty(&self.config).unwrap();
        match std::fs::write("DbOption.toml", toml_content) {
            Ok(_) => {
                log::info!("配置已保存");
            }
            Err(e) => {
                self.validation_errors.push(format!("保存失败: {}", e));
            }
        }
    }
    
    fn reload_config(&mut self) {
        self.config = Self::load_config();
        self.config_text = toml::to_string_pretty(&self.config).unwrap_or_default();
        self.validation_errors.clear();
    }
    
    fn reset_to_default(&mut self) {
        self.config = DbOptionConfig::default();
        self.config_text = toml::to_string_pretty(&self.config).unwrap_or_default();
        self.validation_errors.clear();
    }
    
    fn load_config() -> DbOptionConfig {
        if let Ok(content) = std::fs::read_to_string("DbOption.toml") {
            toml::from_str(&content).unwrap_or_default()
        } else {
            DbOptionConfig::default()
        }
    }
}

impl Default for DbOptionConfig {
    fn default() -> Self {
        Self {
            db_path: "deployment_sites.sqlite".to_string(),
            surreal_host: "localhost".to_string(),
            surreal_port: 8000,
            web_host: "0.0.0.0".to_string(),
            web_port: 3000,
            log_level: "info".to_string(),
            max_connections: 100,
        }
    }
}

impl Default for ConfigEditorPage {
    fn default() -> Self {
        Self::new()
    }
}
