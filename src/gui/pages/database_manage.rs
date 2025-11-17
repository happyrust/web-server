// Database management page

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBConfig {
    pub host: String,
    pub port: u16,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub data_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseStatus {
    Stopped,
    Starting,
    Running { address: String, version: String },
    Stopping,
    Error(String),
}

pub struct DatabaseManagePage {
    config: SurrealDBConfig,
    status: DatabaseStatus,
    logs: Vec<String>,
    process_handle: Option<std::process::Child>,
}

impl DatabaseManagePage {
    pub fn new() -> Self {
        Self {
            config: SurrealDBConfig::default(),
            status: DatabaseStatus::Stopped,
            logs: Vec::new(),
            process_handle: None,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("数据库管理");
        
        ui.horizontal(|ui| {
            ui.label("数据库状态:");
            match &self.status {
                DatabaseStatus::Stopped => {
                    ui.colored_label(egui::Color32::GRAY, "⚪ 已停止");
                }
                DatabaseStatus::Starting => {
                    ui.colored_label(egui::Color32::YELLOW, "🟡 启动中...");
                }
                DatabaseStatus::Running { address, version } => {
                    ui.colored_label(egui::Color32::GREEN, format!("🟢 运行中 ({}, {})", address, version));
                }
                DatabaseStatus::Stopping => {
                    ui.colored_label(egui::Color32::YELLOW, "🟡 停止中...");
                }
                DatabaseStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("🔴 错误: {}", msg));
                }
            }
        });
        
        ui.separator();
        
        egui::Grid::new("db_config")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("主机地址:");
                ui.text_edit_singleline(&mut self.config.host);
                ui.end_row();
                
                ui.label("端口:");
                ui.add(egui::DragValue::new(&mut self.config.port).clamp_range(1..=65535));
                ui.end_row();
                
                ui.label("命名空间:");
                ui.text_edit_singleline(&mut self.config.namespace);
                ui.end_row();
                
                ui.label("数据库:");
                ui.text_edit_singleline(&mut self.config.database);
                ui.end_row();
                
                ui.label("用户名:");
                ui.text_edit_singleline(&mut self.config.username);
                ui.end_row();
                
                ui.label("密码:");
                ui.add(egui::TextEdit::singleline(&mut self.config.password).password(true));
                ui.end_row();
                
                ui.label("数据路径:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.config.data_path);
                    if ui.button("📁").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.config.data_path = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.end_row();
            });
        
        ui.separator();
        
        ui.horizontal(|ui| {
            match &self.status {
                DatabaseStatus::Stopped | DatabaseStatus::Error(_) => {
                    if ui.button("▶️ 启动数据库").clicked() {
                        self.start_database();
                    }
                }
                DatabaseStatus::Running { .. } => {
                    if ui.button("⏹️ 停止数据库").clicked() {
                        self.stop_database();
                    }
                }
                _ => {
                    ui.add_enabled(false, egui::Button::new("处理中..."));
                }
            }
            
            if ui.button("🔍 测试连接").clicked() {
                self.test_connection();
            }
            
            if ui.button("💾 保存配置").clicked() {
                self.save_config();
            }
        });
        
        ui.separator();
        
        ui.heading("数据库日志");
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
    
    fn start_database(&mut self) {
        self.status = DatabaseStatus::Starting;
        self.logs.push(format!("[{}] 正在启动 SurrealDB...", chrono::Local::now().format("%H:%M:%S")));
        
        let command = format!(
            "surreal start --bind {}:{} --user {} --pass {} file://{}",
            self.config.host,
            self.config.port,
            self.config.username,
            self.config.password,
            self.config.data_path
        );
        
        self.logs.push(format!("[{}] 命令: {}", chrono::Local::now().format("%H:%M:%S"), command));
        
        match std::process::Command::new("surreal")
            .args(&["start", "--bind", &format!("{}:{}", self.config.host, self.config.port)])
            .args(&["--user", &self.config.username])
            .args(&["--pass", &self.config.password])
            .arg(format!("file://{}", self.config.data_path))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => {
                self.process_handle = Some(child);
                let address = format!("ws://{}:{}", self.config.host, self.config.port);
                self.status = DatabaseStatus::Running {
                    address: address.clone(),
                    version: "1.0.0".to_string(),
                };
                self.logs.push(format!("[{}] SurrealDB 已启动: {}", chrono::Local::now().format("%H:%M:%S"), address));
            }
            Err(e) => {
                self.status = DatabaseStatus::Error(format!("启动失败: {}", e));
                self.logs.push(format!("[{}] 错误: {}", chrono::Local::now().format("%H:%M:%S"), e));
            }
        }
    }
    
    fn stop_database(&mut self) {
        self.status = DatabaseStatus::Stopping;
        self.logs.push(format!("[{}] 正在停止 SurrealDB...", chrono::Local::now().format("%H:%M:%S")));
        
        if let Some(mut child) = self.process_handle.take() {
            match child.kill() {
                Ok(_) => {
                    self.status = DatabaseStatus::Stopped;
                    self.logs.push(format!("[{}] SurrealDB 已停止", chrono::Local::now().format("%H:%M:%S")));
                }
                Err(e) => {
                    self.status = DatabaseStatus::Error(format!("停止失败: {}", e));
                    self.logs.push(format!("[{}] 错误: {}", chrono::Local::now().format("%H:%M:%S"), e));
                }
            }
        }
    }
    
    fn test_connection(&mut self) {
        self.logs.push(format!("[{}] 正在测试连接...", chrono::Local::now().format("%H:%M:%S")));
        // TODO: Implement connection test
    }
    
    fn save_config(&self) {
        if let Ok(toml_content) = toml::to_string(&self.config) {
            if let Err(e) = std::fs::write("DbOption.toml", toml_content) {
                log::error!("保存配置失败: {}", e);
            }
        }
    }
}

impl Default for SurrealDBConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8000,
            namespace: "default".to_string(),
            database: "default".to_string(),
            username: "root".to_string(),
            password: String::new(),
            data_path: "./data/surreal".to_string(),
        }
    }
}

impl Default for DatabaseManagePage {
    fn default() -> Self {
        Self::new()
    }
}
