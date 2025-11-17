// Web server management page

use crate::gui::state::ServerStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    pub static_dir: String,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            db_path: "deployment_sites.sqlite".to_string(),
            static_dir: "frontend/v0-aios-database-management".to_string(),
        }
    }
}

pub struct WebServerPage {
    config: WebServerConfig,
    status: ServerStatus,
    logs: Vec<String>,
}

impl WebServerPage {
    pub fn new() -> Self {
        Self {
            config: WebServerConfig::default(),
            status: ServerStatus::Stopped,
            logs: Vec::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Web Server 管理");
        
        // Status display
        ui.horizontal(|ui| {
            ui.label("服务器状态:");
            match &self.status {
                ServerStatus::Stopped => {
                    ui.colored_label(egui::Color32::GRAY, "⚪ 已停止");
                }
                ServerStatus::Starting => {
                    ui.colored_label(egui::Color32::YELLOW, "🟡 启动中...");
                }
                ServerStatus::Running { address } => {
                    ui.colored_label(egui::Color32::GREEN, format!("🟢 运行中 ({})", address));
                }
                ServerStatus::Stopping => {
                    ui.colored_label(egui::Color32::YELLOW, "🟡 停止中...");
                }
                ServerStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("🔴 错误: {}", msg));
                }
            }
        });
        
        ui.separator();
        
        // Configuration form
        egui::Grid::new("server_config")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("监听地址:");
                ui.text_edit_singleline(&mut self.config.host);
                ui.end_row();
                
                ui.label("监听端口:");
                ui.add(egui::DragValue::new(&mut self.config.port).range(1..=65535));
                ui.end_row();
                
                ui.label("数据库路径:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.config.db_path);
                    if ui.button("📁").clicked() {
                        // File dialog would go here
                    }
                });
                ui.end_row();
                
                ui.label("静态文件目录:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.config.static_dir);
                    if ui.button("📁").clicked() {
                        // Folder dialog would go here
                    }
                });
                ui.end_row();
            });
        
        ui.separator();
        
        // Operation buttons
        ui.horizontal(|ui| {
            match &self.status {
                ServerStatus::Stopped | ServerStatus::Error(_) => {
                    if ui.button("▶️ 启动服务器").clicked() {
                        self.start_server();
                    }
                }
                ServerStatus::Running { .. } => {
                    if ui.button("⏹️ 停止服务器").clicked() {
                        self.stop_server();
                    }
                }
                _ => {
                    ui.add_enabled(false, egui::Button::new("处理中..."));
                }
            }
            
            if ui.button("💾 保存配置").clicked() {
                self.save_config();
            }
        });
        
        ui.separator();
        
        // Server logs
        ui.heading("服务器日志");
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for log in &self.logs {
                    ui.label(log);
                }
                
                if self.logs.is_empty() {
                    ui.label("暂无日志");
                }
            });
    }
    
    fn start_server(&mut self) {
        self.status = ServerStatus::Starting;
        self.logs.push(format!("[{}] 正在启动服务器...", chrono::Local::now().format("%H:%M:%S")));
        
        // Server start logic would go here
        let address = format!("http://{}:{}", self.config.host, self.config.port);
        self.status = ServerStatus::Running { address: address.clone() };
        self.logs.push(format!("[{}] 服务器已启动: {}", chrono::Local::now().format("%H:%M:%S"), address));
    }
    
    fn stop_server(&mut self) {
        self.status = ServerStatus::Stopping;
        self.logs.push(format!("[{}] 正在停止服务器...", chrono::Local::now().format("%H:%M:%S")));
        
        // Server stop logic would go here
        self.status = ServerStatus::Stopped;
        self.logs.push(format!("[{}] 服务器已停止", chrono::Local::now().format("%H:%M:%S")));
    }
    
    fn save_config(&self) {
        // Save configuration to DbOption.toml
        if let Ok(toml_content) = toml::to_string(&self.config) {
            if let Err(e) = std::fs::write("DbOption.toml", toml_content) {
                eprintln!("Failed to save config: {}", e);
            }
        }
    }
}

impl Default for WebServerPage {
    fn default() -> Self {
        Self::new()
    }
}
