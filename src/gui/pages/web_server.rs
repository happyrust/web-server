// Web server management page with embedded server integration

use crate::gui::embedded_server::{
    ServerHandle, ServerMetrics, MetricsSnapshot, RequestLogger, SystemMonitor, SystemInfo,
    get_local_ip, format_server_address,
};
use crate::gui::state::ServerStatus;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    pub static_dir: String,
    pub web_server_binary: String,
    pub cba_dir: String,
    pub cba_server_port: u16,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            db_path: "deployment_sites.sqlite".to_string(),
            static_dir: "frontend/v0-aios-database-management".to_string(),
            web_server_binary: "./target/release/web_server".to_string(),
            cba_dir: "./CBA".to_string(),
            cba_server_port: 8080,
        }
    }
}

pub struct WebServerPage {
    config: WebServerConfig,
    status: ServerStatus,
    cba_server_status: ServerStatus,
    logs: Vec<String>,
    cba_logs: Vec<String>,
    
    // New fields for embedded server
    server_handle: Option<Arc<ServerHandle>>,
    metrics: Arc<ServerMetrics>,
    request_logger: Arc<RequestLogger>,
    metrics_history: VecDeque<MetricsSnapshot>,
    system_monitor: SystemMonitor,
    last_metrics_sample: std::time::Instant,
    
    // Config import/export
    show_config_import: bool,
    import_url: String,
    import_preview: Option<String>,
    show_qr_code: bool,
    qr_code_url: String,
    
    // Environment selection for config export
    selected_env_id: Option<String>,
    available_environments: Vec<(String, String)>, // (id, name)
    
    // Local IP address
    local_ip: Option<IpAddr>,
}

impl WebServerPage {
    pub fn new() -> Self {
        // Get local IP address
        let local_ip = get_local_ip();
        
        Self {
            config: WebServerConfig::default(),
            status: ServerStatus::Stopped,
            cba_server_status: ServerStatus::Stopped,
            logs: Vec::new(),
            cba_logs: Vec::new(),
            server_handle: None,
            metrics: Arc::new(ServerMetrics::new()),
            request_logger: Arc::new(RequestLogger::new(100)),
            metrics_history: VecDeque::new(),
            system_monitor: SystemMonitor::new(),
            last_metrics_sample: std::time::Instant::now(),
            show_config_import: false,
            import_url: String::new(),
            import_preview: None,
            show_qr_code: false,
            qr_code_url: String::new(),
            selected_env_id: None,
            available_environments: Vec::new(),
            local_ip,
        }
    }
    
    /// Update available environments (should be called when environments change)
    pub fn update_environments(&mut self, environments: Vec<(String, String)>) {
        self.available_environments = environments;
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Web Server 控制面板");
        
        // Server status and control
        self.render_server_status(ui);
        ui.separator();
        
        // Metrics cards
        self.render_metrics_cards(ui);
        ui.separator();
        
        // Performance charts
        self.render_performance_charts(ui);
        ui.separator();
        
        // Request logs
        self.render_request_logs(ui);
        ui.separator();
        
        // Configuration management
        self.render_config_management(ui);
        
        // Config import dialog
        if self.show_config_import {
            self.render_config_import_dialog(ui);
        }
        
        // QR code dialog
        if self.show_qr_code {
            self.render_qr_code_dialog(ui);
        }
        
        // Auto-sample metrics every 5 seconds
        if self.last_metrics_sample.elapsed() > std::time::Duration::from_secs(5) {
            let snapshot = self.metrics.get_snapshot();
            self.metrics_history.push_back(snapshot);
            if self.metrics_history.len() > 60 {
                self.metrics_history.pop_front();
            }
            self.last_metrics_sample = std::time::Instant::now();
        }
        
        // Request repaint for auto-refresh
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
    }
    
    fn render_server_status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("服务器状态:");
            
            if let Some(handle) = &self.server_handle {
                if handle.is_running() {
                    ui.colored_label(egui::Color32::GREEN, "🟢 运行中");
                    
                    // Display local IP address instead of 0.0.0.0
                    let display_address = if let Some(ip) = &self.local_ip {
                        format_server_address(ip, self.config.port)
                    } else {
                        handle.address().to_string()
                    };
                    ui.label(format!("地址: {}", display_address));
                    ui.label(format!("运行时长: {:?}", handle.uptime()));
                    
                    if ui.button("⏹️ 停止").clicked() {
                        self.stop_server();
                    }
                    if ui.button("🔄 重启").clicked() {
                        self.restart_server();
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, "⚪ 已停止");
                    if ui.button("▶️ 启动").clicked() {
                        self.start_server();
                    }
                }
            } else {
                ui.colored_label(egui::Color32::GRAY, "⚪ 已停止");
                
                // Show local IP address
                if let Some(ip) = &self.local_ip {
                    ui.label(format!("本机 IP: {}", ip));
                }
                
                if ui.button("▶️ 启动").clicked() {
                    self.start_server();
                }
            }
        });
        
        // Configuration
        egui::CollapsingHeader::new("配置")
            .default_open(false)
            .show(ui, |ui| {
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
                        
                        ui.label("本机 IP:");
                        if let Some(ip) = &self.local_ip {
                            ui.label(ip.to_string());
                        } else {
                            ui.label("未检测到");
                        }
                        ui.end_row();
                    });
            });
    }
    
    fn render_metrics_cards(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.metrics.get_snapshot();
        let sys_info = self.system_monitor.get_info();
        
        ui.horizontal(|ui| {
            // Request statistics card
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(240, 248, 255))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("请求统计");
                        ui.label(format!("总请求数: {}", snapshot.total_requests));
                        ui.label(format!("成功: {}", snapshot.success_requests));
                        ui.label(format!("失败: {}", snapshot.error_requests));
                        let success_rate = if snapshot.total_requests > 0 {
                            (snapshot.success_requests as f64 / snapshot.total_requests as f64) * 100.0
                        } else {
                            0.0
                        };
                        ui.label(format!("成功率: {:.2}%", success_rate));
                    });
                });
            
            // Connection statistics card
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(240, 255, 240))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("连接统计");
                        ui.label(format!("活跃连接: {}", snapshot.active_connections));
                        ui.label(format!("峰值连接: {}", snapshot.peak_connections));
                    });
                });
            
            // Performance statistics card
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(255, 248, 240))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("性能统计");
                        ui.label(format!("平均响应时间: {} ms", snapshot.avg_response_time_ms));
                        ui.label(format!("CPU 使用率: {:.1}%", sys_info.cpu_usage));
                        ui.label(format!("内存使用: {} MB", sys_info.memory_used_mb));
                    });
                });
        });
    }
    
    fn render_performance_charts(&mut self, ui: &mut egui::Ui) {
        ui.heading("性能趋势");
        
        if self.metrics_history.is_empty() {
            ui.label("暂无数据");
            return;
        }
        
        // Simple text-based metrics display
        ui.label("请求数趋势 (最近 60 个采样点):");
        let request_values: Vec<String> = self.metrics_history
            .iter()
            .map(|s| s.total_requests.to_string())
            .collect();
        ui.label(format!("最新: {}", request_values.last().unwrap_or(&"0".to_string())));
        
        ui.add_space(10.0);
        
        ui.label("响应时间趋势 (ms):");
        let response_values: Vec<String> = self.metrics_history
            .iter()
            .map(|s| s.avg_response_time_ms.to_string())
            .collect();
        ui.label(format!("最新: {} ms", response_values.last().unwrap_or(&"0".to_string())));
        
        // TODO: Implement proper chart visualization when egui_plot version is compatible
        ui.label("(图表功能将在后续版本中添加)");
    }
    
    fn render_request_logs(&mut self, ui: &mut egui::Ui) {
        ui.heading("请求日志");
        
        ui.horizontal(|ui| {
            if ui.button("🔄 刷新").clicked() {
                // Logs are automatically updated
            }
            if ui.button("🗑️ 清空").clicked() {
                self.request_logger.clear();
            }
        });
        
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(150.0)) // Time
            .column(Column::auto().at_least(60.0))  // Method
            .column(Column::auto().at_least(200.0)) // Path
            .column(Column::auto().at_least(60.0))  // Status
            .column(Column::auto().at_least(80.0))  // Duration
            .column(Column::remainder())            // Client IP
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong("时间"); });
                header.col(|ui| { ui.strong("方法"); });
                header.col(|ui| { ui.strong("路径"); });
                header.col(|ui| { ui.strong("状态码"); });
                header.col(|ui| { ui.strong("响应时间"); });
                header.col(|ui| { ui.strong("客户端"); });
            })
            .body(|mut body| {
                let logs = self.request_logger.get_logs();
                for log in logs.iter().rev().take(100) {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(log.timestamp.format("%H:%M:%S").to_string());
                        });
                        row.col(|ui| {
                            ui.label(&log.method);
                        });
                        row.col(|ui| {
                            ui.label(&log.path);
                        });
                        row.col(|ui| {
                            let color = if log.status_code < 400 {
                                egui::Color32::GREEN
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(color, log.status_code.to_string());
                        });
                        row.col(|ui| {
                            ui.label(format!("{} ms", log.duration_ms));
                        });
                        row.col(|ui| {
                            ui.label(&log.client_ip);
                        });
                    });
                }
            });
    }
    
    fn render_config_management(&mut self, ui: &mut egui::Ui) {
        ui.heading("配置管理");
        
        // Environment selection for export
        if !self.available_environments.is_empty() {
            ui.horizontal(|ui| {
                ui.label("选择环境:");
                egui::ComboBox::from_label("")
                    .selected_text(
                        self.selected_env_id
                            .as_ref()
                            .and_then(|id| {
                                self.available_environments
                                    .iter()
                                    .find(|(env_id, _)| env_id == id)
                                    .map(|(_, name)| name.as_str())
                            })
                            .unwrap_or("全部环境")
                    )
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.selected_env_id.is_none(), "全部环境").clicked() {
                            self.selected_env_id = None;
                        }
                        for (env_id, env_name) in &self.available_environments {
                            let is_selected = self.selected_env_id.as_ref() == Some(env_id);
                            if ui.selectable_label(is_selected, env_name).clicked() {
                                self.selected_env_id = Some(env_id.clone());
                            }
                        }
                    });
                
                if ui.button("ℹ️").on_hover_text("选择要导出的环境。\n选择'全部环境'将导出所有配置。\n选择特定环境只导出该环境的配置。").clicked() {
                    // Info button clicked
                }
            });
        }
        
        ui.horizontal(|ui| {
            if ui.button("📤 导出配置").clicked() {
                self.export_config();
            }
            if ui.button("📥 导入配置").clicked() {
                self.show_config_import = true;
            }
            if ui.button("📱 生成二维码").clicked() {
                self.show_qr_code = true;
                self.generate_qr_code_url();
            }
        });
    }
    
    fn render_config_import_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("导入配置")
            .collapsible(false)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.label("输入主站点配置 URL:");
                ui.text_edit_singleline(&mut self.import_url);
                
                ui.horizontal(|ui| {
                    if ui.button("获取配置").clicked() {
                        self.fetch_config_from_url();
                    }
                    if ui.button("取消").clicked() {
                        self.show_config_import = false;
                    }
                });
                
                if let Some(preview) = &self.import_preview {
                    ui.separator();
                    ui.heading("配置预览");
                    ui.label(preview);
                    
                    if ui.button("确认导入").clicked() {
                        self.import_config();
                    }
                }
            });
    }
    
    fn render_qr_code_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("配置导出二维码")
            .collapsible(false)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.label("扫描二维码获取配置:");
                ui.code(&self.qr_code_url);
                
                // TODO: Render actual QR code image
                ui.label("(二维码图片将在此显示)");
                
                ui.horizontal(|ui| {
                    if ui.button("复制 URL").clicked() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&self.qr_code_url);
                            self.logs.push(format!("[{}] URL 已复制到剪贴板", chrono::Local::now().format("%H:%M:%S")));
                        }
                    }
                    if ui.button("关闭").clicked() {
                        self.show_qr_code = false;
                    }
                });
            });
    }
    
    fn start_server(&mut self) {
        self.status = ServerStatus::Starting;
        self.logs.push(format!("[{}] 正在启动嵌入式 Web Server...", chrono::Local::now().format("%H:%M:%S")));
        
        let metrics = self.metrics.clone();
        let logger = self.request_logger.clone();
        let host = self.config.host.clone();
        let port = self.config.port;
        
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        
        let task_handle = tokio::spawn(async move {
            crate::gui::embedded_server::start_embedded_server(
                metrics,
                logger,
                shutdown_rx,
                host,
                port,
            ).await
        });
        
        let address = format!("http://{}:{}", self.config.host, self.config.port);
        self.server_handle = Some(Arc::new(ServerHandle::new(
            task_handle,
            shutdown_tx,
            address.clone(),
        )));
        
        self.status = ServerStatus::Running { address: address.clone() };
        self.logs.push(format!("[{}] Web Server 已启动: {}", chrono::Local::now().format("%H:%M:%S"), address));
    }
    
    fn stop_server(&mut self) {
        self.status = ServerStatus::Stopping;
        self.logs.push(format!("[{}] 正在停止 Web Server...", chrono::Local::now().format("%H:%M:%S")));
        
        if let Some(handle) = self.server_handle.take() {
            tokio::spawn(async move {
                if let Ok(handle) = Arc::try_unwrap(handle) {
                    let _ = handle.shutdown().await;
                }
            });
        }
        
        self.status = ServerStatus::Stopped;
        self.logs.push(format!("[{}] Web Server 已停止", chrono::Local::now().format("%H:%M:%S")));
    }
    
    fn restart_server(&mut self) {
        self.stop_server();
        // Wait a bit before restarting
        std::thread::sleep(std::time::Duration::from_millis(500));
        self.start_server();
    }
    
    fn export_config(&mut self) {
        // Use local IP if available
        let host = if let Some(ip) = &self.local_ip {
            ip.to_string()
        } else {
            self.config.host.clone()
        };
        
        // Build URL with optional env_id parameter
        let mut url = format!("http://{}:{}/api/config/export", host, self.config.port);
        if let Some(env_id) = &self.selected_env_id {
            url.push_str(&format!("?env_id={}", env_id));
            self.logs.push(format!("[{}] 导出环境: {}", 
                chrono::Local::now().format("%H:%M:%S"), 
                self.available_environments
                    .iter()
                    .find(|(id, _)| id == env_id)
                    .map(|(_, name)| name.as_str())
                    .unwrap_or("未知")
            ));
        } else {
            self.logs.push(format!("[{}] 导出所有环境", chrono::Local::now().format("%H:%M:%S")));
        }
        
        self.logs.push(format!("[{}] 配置导出 URL: {}", chrono::Local::now().format("%H:%M:%S"), url));
        
        // Copy to clipboard
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(&url);
            self.logs.push(format!("[{}] URL 已复制到剪贴板", chrono::Local::now().format("%H:%M:%S")));
        }
    }
    
    fn generate_qr_code_url(&mut self) {
        // Use local IP if available
        let host = if let Some(ip) = &self.local_ip {
            ip.to_string()
        } else {
            self.config.host.clone()
        };
        
        // Build URL with optional env_id parameter
        let mut url = format!("http://{}:{}/api/config/export", host, self.config.port);
        if let Some(env_id) = &self.selected_env_id {
            url.push_str(&format!("?env_id={}", env_id));
        }
        
        self.qr_code_url = url;
    }
    
    fn fetch_config_from_url(&mut self) {
        self.logs.push(format!("[{}] 正在获取配置: {}", chrono::Local::now().format("%H:%M:%S"), self.import_url));
        
        // TODO: Implement actual HTTP request
        self.import_preview = Some("配置预览将在此显示".to_string());
    }
    
    fn import_config(&mut self) {
        self.logs.push(format!("[{}] 正在导入配置...", chrono::Local::now().format("%H:%M:%S")));
        
        // TODO: Implement actual config import
        self.show_config_import = false;
        self.logs.push(format!("[{}] 配置导入完成", chrono::Local::now().format("%H:%M:%S")));
    }
}

impl Default for WebServerPage {
    fn default() -> Self {
        Self::new()
    }
}
