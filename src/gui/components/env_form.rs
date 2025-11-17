// Environment configuration form component

use crate::gui::state::RemoteSyncEnv;
use std::collections::HashMap;

pub struct EnvironmentForm {
    pub id: Option<String>,
    pub name: String,
    pub mqtt_host: String,
    pub mqtt_port: String,
    pub file_server_host: String,
    pub location: String,
    pub location_dbs: String,
    pub errors: HashMap<String, String>,
}

impl EnvironmentForm {
    pub fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            mqtt_host: String::new(),
            mqtt_port: "1883".to_string(),
            file_server_host: String::new(),
            location: String::new(),
            location_dbs: String::new(),
            errors: HashMap::new(),
        }
    }
    
    pub fn from_env(env: &RemoteSyncEnv) -> Self {
        Self {
            id: Some(env.id.clone()),
            name: env.name.clone(),
            mqtt_host: env.mqtt_host.clone().unwrap_or_default(),
            mqtt_port: env.mqtt_port.map(|p| p.to_string()).unwrap_or_else(|| "1883".to_string()),
            file_server_host: env.file_server_host.clone().unwrap_or_default(),
            location: env.location.clone(),
            location_dbs: env.location_dbs.clone().unwrap_or_default(),
            errors: HashMap::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("env_form_grid")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("环境名称 *");
                ui.text_edit_singleline(&mut self.name);
                ui.end_row();
                
                if let Some(error) = self.errors.get("name") {
                    ui.label("");
                    ui.colored_label(egui::Color32::RED, error);
                    ui.end_row();
                }
                
                ui.label("MQTT 主机");
                ui.text_edit_singleline(&mut self.mqtt_host);
                ui.end_row();
                
                ui.label("MQTT 端口");
                ui.text_edit_singleline(&mut self.mqtt_port);
                ui.end_row();
                
                if let Some(error) = self.errors.get("mqtt_port") {
                    ui.label("");
                    ui.colored_label(egui::Color32::RED, error);
                    ui.end_row();
                }
                
                ui.label("文件服务器地址");
                ui.text_edit_singleline(&mut self.file_server_host);
                ui.end_row();
                
                ui.label("地区标识");
                ui.text_edit_singleline(&mut self.location);
                ui.end_row();
                
                ui.label("数据库编号");
                ui.text_edit_singleline(&mut self.location_dbs);
                ui.end_row();
                
                ui.label("");
                ui.label("(逗号分隔，如: 7999,8001,8002)");
                ui.end_row();
            });
    }
    
    pub fn validate(&mut self) -> bool {
        self.errors.clear();
        
        if self.name.trim().is_empty() {
            self.errors.insert("name".to_string(), "环境名称不能为空".to_string());
        }
        
        if !self.mqtt_port.is_empty() {
            if let Ok(port) = self.mqtt_port.parse::<u16>() {
                if port == 0 {
                    self.errors.insert("mqtt_port".to_string(), "端口号必须大于 0".to_string());
                }
            } else {
                self.errors.insert("mqtt_port".to_string(), "端口号格式不正确".to_string());
            }
        }
        
        self.errors.is_empty()
    }
    
    pub fn to_env(&self) -> RemoteSyncEnv {
        RemoteSyncEnv {
            id: self.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name.clone(),
            mqtt_host: if self.mqtt_host.is_empty() { None } else { Some(self.mqtt_host.clone()) },
            mqtt_port: self.mqtt_port.parse().ok(),
            file_server_host: if self.file_server_host.is_empty() { None } else { Some(self.file_server_host.clone()) },
            location: self.location.clone(),
            location_dbs: if self.location_dbs.is_empty() { None } else { Some(self.location_dbs.clone()) },
            reconnect_initial_ms: Some(1000),
            reconnect_max_ms: Some(30000),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl Default for EnvironmentForm {
    fn default() -> Self {
        Self::new()
    }
}
