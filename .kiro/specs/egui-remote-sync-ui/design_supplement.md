# 设计文档补充 - 解析和模型生成功能集成

## 概述

本补充文档描述如何在 egui 界面中集成 PDMS 数据解析和模型生成功能，以及如何一键部署和启动 web-server。

## 新增页面和功能

### 1. 解析任务管理页面 (ParseTaskPage)

```rust
pub struct ParseTaskPage {
    tasks: Vec<ParseTask>,
    selected_task: Option<String>,
    show_create_dialog: bool,
    task_form: ParseTaskForm,
    re_ui: re_ui::ReUi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseTask {
    pub id: String,
    pub name: String,
    pub db_path: String,
    pub output_dir: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ParseTaskPage {
    pub fn render(&mut self, ui: &mut egui::Ui, api_client: &ApiClient) {
        ui.heading("PDMS 数据解析");
        
        // 工具栏
        ui.horizontal(|ui| {
            if self.re_ui.large_button(ui, "➕ 新建解析任务").clicked() {
                self.show_create_dialog = true;
                self.task_form = ParseTaskForm::new();
            }
            
            if self.re_ui.large_button(ui, "🔄 刷新").clicked() {
                self.refresh_tasks(api_client);
            }
        });
        
        ui.separator();
        
        // 任务列表
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(150.0)) // 任务名称
            .column(Column::auto().at_least(200.0)) // 数据库路径
            .column(Column::auto().at_least(100.0)) // 状态
            .column(Column::auto().at_least(150.0)) // 进度
            .column(Column::remainder())            // 操作
            .header(20.0, |mut header| {
                header.col(|ui| { self.re_ui.table_header(ui, "任务名称"); });
                header.col(|ui| { self.re_ui.table_header(ui, "数据库路径"); });
                header.col(|ui| { self.re_ui.table_header(ui, "状态"); });
                header.col(|ui| { self.re_ui.table_header(ui, "进度"); });
                header.col(|ui| { self.re_ui.table_header(ui, "操作"); });
            })
            .body(|mut body| {
                for task in &self.tasks {
                    body.row(30.0, |mut row| {
                        row.col(|ui| { ui.label(&task.name); });
                        row.col(|ui| { ui.label(&task.db_path); });
                        row.col(|ui| {
                            self.render_task_status(ui, &task.status);
                        });
                        row.col(|ui| {
                            ui.add(egui::ProgressBar::new(task.progress)
                                .text(format!("{:.1}%", task.progress * 100.0)));
                        });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if task.status == TaskStatus::Pending {
                                    if self.re_ui.small_icon_button(ui, &re_ui::icons::PLAY).clicked() {
                                        self.start_task(api_client, &task.id);
                                    }
                                }
                                if task.status == TaskStatus::Running {
                                    if self.re_ui.small_icon_button(ui, &re_ui::icons::PAUSE).clicked() {
                                        self.cancel_task(api_client, &task.id);
                                    }
                                }
                                if self.re_ui.small_icon_button(ui, &re_ui::icons::REMOVE).clicked() {
                                    self.delete_task(api_client, &task.id);
                                }
                            });
                        });
                    });
                }
            });
        
        // 创建任务对话框
        if self.show_create_dialog {
            self.render_create_dialog(ui, api_client);
        }
    }
    
    fn render_task_status(&self, ui: &mut egui::Ui, status: &TaskStatus) {
        let (color, text) = match status {
            TaskStatus::Pending => (egui::Color32::GRAY, "⏸️ 等待中"),
            TaskStatus::Running => (egui::Color32::BLUE, "🟢 运行中"),
            TaskStatus::Completed => (egui::Color32::GREEN, "✅ 完成"),
            TaskStatus::Failed => (egui::Color32::RED, "❌ 失败"),
            TaskStatus::Cancelled => (egui::Color32::YELLOW, "⚠️ 已取消"),
        };
        ui.label(egui::RichText::new(text).color(color));
    }
    
    fn render_create_dialog(&mut self, ui: &mut egui::Ui, api_client: &ApiClient) {
        egui::Window::new("创建解析任务")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                self.re_ui.panel_content(ui, |ui| {
                    self.task_form.render(ui, &self.re_ui);
                    
                    ui.horizontal(|ui| {
                        if self.re_ui.large_button(ui, "创建").clicked() {
                            if self.task_form.validate() {
                                self.create_task(api_client);
                                self.show_create_dialog = false;
                            }
                        }
                        
                        if self.re_ui.large_button(ui, "取消").clicked() {
                            self.show_create_dialog = false;
                        }
                    });
                });
            });
    }
    
    fn create_task(&mut self, api_client: &ApiClient) {
        let task = self.task_form.to_parse_task();
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            let _ = api_client.create_parse_task(&task).await;
        });
    }
    
    fn start_task(&mut self, api_client: &ApiClient, task_id: &str) {
        let api_client = api_client.clone();
        let task_id = task_id.to_string();
        
        tokio::spawn(async move {
            let _ = api_client.start_parse_task(&task_id).await;
        });
    }
    
    fn cancel_task(&mut self, api_client: &ApiClient, task_id: &str) {
        let api_client = api_client.clone();
        let task_id = task_id.to_string();
        
        tokio::spawn(async move {
            let _ = api_client.cancel_parse_task(&task_id).await;
        });
    }
    
    fn delete_task(&mut self, api_client: &ApiClient, task_id: &str) {
        let api_client = api_client.clone();
        let task_id = task_id.to_string();
        
        tokio::spawn(async move {
            let _ = api_client.delete_parse_task(&task_id).await;
        });
    }
    
    fn refresh_tasks(&mut self, api_client: &ApiClient) {
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            if let Ok(tasks) = api_client.get_parse_tasks().await {
                // 更新任务列表
            }
        });
    }
}

pub struct ParseTaskForm {
    pub name: String,
    pub db_path: String,
    pub output_dir: String,
    pub errors: HashMap<String, String>,
}

impl ParseTaskForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            db_path: String::new(),
            output_dir: String::new(),
            errors: HashMap::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, re_ui: &re_ui::ReUi) {
        egui::Grid::new("parse_task_form")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("任务名称 *");
                ui.text_edit_singleline(&mut self.name);
                ui.end_row();
                
                if let Some(error) = self.errors.get("name") {
                    ui.label("");
                    ui.colored_label(egui::Color32::RED, error);
                    ui.end_row();
                }
                
                ui.label("数据库路径 *");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.db_path);
                    if re_ui.small_icon_button(ui, &re_ui::icons::FOLDER).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Database", &["db", "sqlite"])
                            .pick_file()
                        {
                            self.db_path = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.end_row();
                
                ui.label("输出目录 *");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.output_dir);
                    if re_ui.small_icon_button(ui, &re_ui::icons::FOLDER).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.output_dir = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.end_row();
            });
    }
    
    pub fn validate(&mut self) -> bool {
        self.errors.clear();
        
        if self.name.trim().is_empty() {
            self.errors.insert("name".to_string(), "任务名称不能为空".to_string());
        }
        
        if self.db_path.trim().is_empty() {
            self.errors.insert("db_path".to_string(), "数据库路径不能为空".to_string());
        } else if !std::path::Path::new(&self.db_path).exists() {
            self.errors.insert("db_path".to_string(), "数据库文件不存在".to_string());
        }
        
        if self.output_dir.trim().is_empty() {
            self.errors.insert("output_dir".to_string(), "输出目录不能为空".to_string());
        }
        
        self.errors.is_empty()
    }
    
    pub fn to_parse_task(&self) -> ParseTask {
        ParseTask {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name.clone(),
            db_path: self.db_path.clone(),
            output_dir: self.output_dir.clone(),
            status: TaskStatus::Pending,
            progress: 0.0,
            created_at: chrono::Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            error_message: None,
        }
    }
}
```


### 2. 模型生成配置页面 (ModelGenPage)

```rust
pub struct ModelGenPage {
    configs: Vec<ModelGenConfig>,
    selected_config: Option<String>,
    show_config_dialog: bool,
    config_form: ModelGenConfigForm,
    generation_tasks: Vec<GenerationTask>,
    re_ui: re_ui::ReUi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGenConfig {
    pub id: String,
    pub name: String,
    pub db_path: String,
    pub output_format: OutputFormat,
    pub lod_levels: Vec<String>,
    pub include_materials: bool,
    pub compress_output: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputFormat {
    XKT,
    GLB,
    OBJ,
    IFC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTask {
    pub id: String,
    pub config_id: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub output_files: Vec<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl ModelGenPage {
    pub fn render(&mut self, ui: &mut egui::Ui, api_client: &ApiClient) {
        ui.heading("模型生成配置");
        
        // 左侧：配置列表
        egui::SidePanel::left("config_list")
            .default_width(250.0)
            .show_inside(ui, |ui| {
                ui.heading("配置列表");
                
                if self.re_ui.large_button(ui, "➕ 新建配置").clicked() {
                    self.show_config_dialog = true;
                    self.config_form = ModelGenConfigForm::new();
                }
                
                ui.separator();
                
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for config in &self.configs {
                        self.re_ui.list_item()
                            .selected(self.selected_config == Some(config.id.clone()))
                            .show_flat(ui, |ui| {
                                if ui.selectable_label(false, &config.name).clicked() {
                                    self.selected_config = Some(config.id.clone());
                                }
                            });
                    }
                });
            });
        
        // 右侧：配置详情和生成任务
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(config_id) = &self.selected_config {
                if let Some(config) = self.configs.iter().find(|c| &c.id == config_id) {
                    self.render_config_detail(ui, config, api_client);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("请选择一个配置");
                });
            }
        });
        
        // 配置对话框
        if self.show_config_dialog {
            self.render_config_dialog(ui, api_client);
        }
    }
    
    fn render_config_detail(&mut self, ui: &mut egui::Ui, config: &ModelGenConfig, api_client: &ApiClient) {
        ui.heading(&config.name);
        
        ui.horizontal(|ui| {
            if self.re_ui.large_button(ui, "▶️ 开始生成").clicked() {
                self.start_generation(api_client, &config.id);
            }
            
            if self.re_ui.large_button(ui, "✏️ 编辑配置").clicked() {
                self.config_form = ModelGenConfigForm::from_config(config);
                self.show_config_dialog = true;
            }
            
            if self.re_ui.large_button(ui, "🗑️ 删除配置").clicked() {
                self.delete_config(api_client, &config.id);
            }
        });
        
        ui.separator();
        
        // 配置详情
        self.re_ui.collapsing_header(ui, "配置详情", true, |ui| {
            egui::Grid::new("config_detail")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("数据库路径:");
                    ui.label(&config.db_path);
                    ui.end_row();
                    
                    ui.label("输出格式:");
                    ui.label(format!("{:?}", config.output_format));
                    ui.end_row();
                    
                    ui.label("LOD 级别:");
                    ui.label(config.lod_levels.join(", "));
                    ui.end_row();
                    
                    ui.label("包含材质:");
                    ui.label(if config.include_materials { "是" } else { "否" });
                    ui.end_row();
                    
                    ui.label("压缩输出:");
                    ui.label(if config.compress_output { "是" } else { "否" });
                    ui.end_row();
                });
        });
        
        ui.separator();
        
        // 生成任务列表
        ui.heading("生成任务");
        
        let tasks: Vec<_> = self.generation_tasks
            .iter()
            .filter(|t| t.config_id == config.id)
            .collect();
        
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(100.0)) // 状态
            .column(Column::auto().at_least(150.0)) // 进度
            .column(Column::auto().at_least(150.0)) // 开始时间
            .column(Column::remainder())            // 操作
            .header(20.0, |mut header| {
                header.col(|ui| { self.re_ui.table_header(ui, "状态"); });
                header.col(|ui| { self.re_ui.table_header(ui, "进度"); });
                header.col(|ui| { self.re_ui.table_header(ui, "开始时间"); });
                header.col(|ui| { self.re_ui.table_header(ui, "操作"); });
            })
            .body(|mut body| {
                for task in tasks {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            self.render_task_status(ui, &task.status);
                        });
                        row.col(|ui| {
                            ui.add(egui::ProgressBar::new(task.progress)
                                .text(format!("{:.1}%", task.progress * 100.0)));
                        });
                        row.col(|ui| {
                            if let Some(started_at) = &task.started_at {
                                ui.label(started_at);
                            }
                        });
                        row.col(|ui| {
                            if task.status == TaskStatus::Completed {
                                if self.re_ui.small_icon_button(ui, &re_ui::icons::FOLDER).clicked() {
                                    // 打开输出目录
                                }
                            }
                        });
                    });
                }
            });
    }
    
    fn render_config_dialog(&mut self, ui: &mut egui::Ui, api_client: &ApiClient) {
        egui::Window::new("模型生成配置")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                self.re_ui.panel_content(ui, |ui| {
                    self.config_form.render(ui, &self.re_ui);
                    
                    ui.horizontal(|ui| {
                        if self.re_ui.large_button(ui, "保存").clicked() {
                            if self.config_form.validate() {
                                self.save_config(api_client);
                                self.show_config_dialog = false;
                            }
                        }
                        
                        if self.re_ui.large_button(ui, "取消").clicked() {
                            self.show_config_dialog = false;
                        }
                    });
                });
            });
    }
    
    fn start_generation(&mut self, api_client: &ApiClient, config_id: &str) {
        let api_client = api_client.clone();
        let config_id = config_id.to_string();
        
        tokio::spawn(async move {
            let _ = api_client.start_model_generation(&config_id).await;
        });
    }
    
    fn save_config(&mut self, api_client: &ApiClient) {
        let config = self.config_form.to_model_gen_config();
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            let _ = api_client.save_model_gen_config(&config).await;
        });
    }
    
    fn delete_config(&mut self, api_client: &ApiClient, config_id: &str) {
        let api_client = api_client.clone();
        let config_id = config_id.to_string();
        
        tokio::spawn(async move {
            let _ = api_client.delete_model_gen_config(&config_id).await;
        });
    }
    
    fn render_task_status(&self, ui: &mut egui::Ui, status: &TaskStatus) {
        let (color, text) = match status {
            TaskStatus::Pending => (egui::Color32::GRAY, "⏸️ 等待中"),
            TaskStatus::Running => (egui::Color32::BLUE, "🟢 运行中"),
            TaskStatus::Completed => (egui::Color32::GREEN, "✅ 完成"),
            TaskStatus::Failed => (egui::Color32::RED, "❌ 失败"),
            TaskStatus::Cancelled => (egui::Color32::YELLOW, "⚠️ 已取消"),
        };
        ui.label(egui::RichText::new(text).color(color));
    }
}

pub struct ModelGenConfigForm {
    pub id: Option<String>,
    pub name: String,
    pub db_path: String,
    pub output_format: OutputFormat,
    pub lod_levels: Vec<bool>, // L0, L1, L2, L3
    pub include_materials: bool,
    pub compress_output: bool,
    pub errors: HashMap<String, String>,
}

impl ModelGenConfigForm {
    pub fn new() -> Self {
        Self {
            id: None,
            name: String::new(),
            db_path: String::new(),
            output_format: OutputFormat::XKT,
            lod_levels: vec![true, true, false, false],
            include_materials: true,
            compress_output: false,
            errors: HashMap::new(),
        }
    }
    
    pub fn from_config(config: &ModelGenConfig) -> Self {
        let lod_levels = vec![
            config.lod_levels.contains(&"L0".to_string()),
            config.lod_levels.contains(&"L1".to_string()),
            config.lod_levels.contains(&"L2".to_string()),
            config.lod_levels.contains(&"L3".to_string()),
        ];
        
        Self {
            id: Some(config.id.clone()),
            name: config.name.clone(),
            db_path: config.db_path.clone(),
            output_format: config.output_format.clone(),
            lod_levels,
            include_materials: config.include_materials,
            compress_output: config.compress_output,
            errors: HashMap::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, re_ui: &re_ui::ReUi) {
        egui::Grid::new("model_gen_config_form")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.label("配置名称 *");
                ui.text_edit_singleline(&mut self.name);
                ui.end_row();
                
                ui.label("数据库路径 *");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.db_path);
                    if re_ui.small_icon_button(ui, &re_ui::icons::FOLDER).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Database", &["db", "sqlite"])
                            .pick_file()
                        {
                            self.db_path = path.to_string_lossy().to_string();
                        }
                    }
                });
                ui.end_row();
                
                ui.label("输出格式");
                egui::ComboBox::from_id_source("output_format")
                    .selected_text(format!("{:?}", self.output_format))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.output_format, OutputFormat::XKT, "XKT");
                        ui.selectable_value(&mut self.output_format, OutputFormat::GLB, "GLB");
                        ui.selectable_value(&mut self.output_format, OutputFormat::OBJ, "OBJ");
                        ui.selectable_value(&mut self.output_format, OutputFormat::IFC, "IFC");
                    });
                ui.end_row();
                
                ui.label("LOD 级别");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.lod_levels[0], "L0");
                    ui.checkbox(&mut self.lod_levels[1], "L1");
                    ui.checkbox(&mut self.lod_levels[2], "L2");
                    ui.checkbox(&mut self.lod_levels[3], "L3");
                });
                ui.end_row();
                
                ui.label("包含材质");
                ui.checkbox(&mut self.include_materials, "");
                ui.end_row();
                
                ui.label("压缩输出");
                ui.checkbox(&mut self.compress_output, "");
                ui.end_row();
            });
    }
    
    pub fn validate(&mut self) -> bool {
        self.errors.clear();
        
        if self.name.trim().is_empty() {
            self.errors.insert("name".to_string(), "配置名称不能为空".to_string());
        }
        
        if self.db_path.trim().is_empty() {
            self.errors.insert("db_path".to_string(), "数据库路径不能为空".to_string());
        }
        
        if !self.lod_levels.iter().any(|&x| x) {
            self.errors.insert("lod_levels".to_string(), "至少选择一个 LOD 级别".to_string());
        }
        
        self.errors.is_empty()
    }
    
    pub fn to_model_gen_config(&self) -> ModelGenConfig {
        let lod_levels: Vec<String> = self.lod_levels
            .iter()
            .enumerate()
            .filter_map(|(i, &enabled)| {
                if enabled {
                    Some(format!("L{}", i))
                } else {
                    None
                }
            })
            .collect();
        
        ModelGenConfig {
            id: self.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: self.name.clone(),
            db_path: self.db_path.clone(),
            output_format: self.output_format.clone(),
            lod_levels,
            include_materials: self.include_materials,
            compress_output: self.compress_output,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
```


### 3. 一键部署和启动 Web Server

```rust
pub struct QuickDeployPage {
    deploy_config: DeployConfig,
    deploy_status: DeployStatus,
    server_status: ServerStatus,
    logs: Vec<String>,
    re_ui: re_ui::ReUi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    pub server_host: String,
    pub server_port: u16,
    pub db_path: String,
    pub static_dir: String,
    pub enable_cors: bool,
    pub log_level: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeployStatus {
    NotStarted,
    Deploying,
    Deployed,
    Failed(String),
}

impl QuickDeployPage {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("一键部署和启动");
        
        // 状态卡片
        ui.horizontal(|ui| {
            self.render_status_card(ui, "部署状态", &self.deploy_status);
            self.render_status_card(ui, "服务器状态", &self.server_status);
        });
        
        ui.separator();
        
        // 配置表单
        self.re_ui.collapsing_header(ui, "服务器配置", true, |ui| {
            egui::Grid::new("deploy_config")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("监听地址:");
                    ui.text_edit_singleline(&mut self.deploy_config.server_host);
                    ui.end_row();
                    
                    ui.label("监听端口:");
                    ui.add(egui::DragValue::new(&mut self.deploy_config.server_port)
                        .clamp_range(1..=65535));
                    ui.end_row();
                    
                    ui.label("数据库路径:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.deploy_config.db_path);
                        if self.re_ui.small_icon_button(ui, &re_ui::icons::FOLDER).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("SQLite", &["db", "sqlite"])
                                .pick_file()
                            {
                                self.deploy_config.db_path = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.end_row();
                    
                    ui.label("静态文件目录:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.deploy_config.static_dir);
                        if self.re_ui.small_icon_button(ui, &re_ui::icons::FOLDER).clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.deploy_config.static_dir = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.end_row();
                    
                    ui.label("启用 CORS:");
                    ui.checkbox(&mut self.deploy_config.enable_cors, "");
                    ui.end_row();
                    
                    ui.label("日志级别:");
                    egui::ComboBox::from_id_source("log_level")
                        .selected_text(&self.deploy_config.log_level)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.deploy_config.log_level, "trace".to_string(), "Trace");
                            ui.selectable_value(&mut self.deploy_config.log_level, "debug".to_string(), "Debug");
                            ui.selectable_value(&mut self.deploy_config.log_level, "info".to_string(), "Info");
                            ui.selectable_value(&mut self.deploy_config.log_level, "warn".to_string(), "Warn");
                            ui.selectable_value(&mut self.deploy_config.log_level, "error".to_string(), "Error");
                        });
                    ui.end_row();
                });
        });
        
        ui.separator();
        
        // 操作按钮
        ui.horizontal(|ui| {
            match &self.server_status {
                ServerStatus::Stopped => {
                    if self.re_ui.large_button(ui, "🚀 一键部署并启动").clicked() {
                        self.deploy_and_start();
                    }
                }
                ServerStatus::Running { address } => {
                    if self.re_ui.large_button(ui, "⏹️ 停止服务器").clicked() {
                        self.stop_server();
                    }
                    
                    if self.re_ui.large_button(ui, "🔄 重启服务器").clicked() {
                        self.restart_server();
                    }
                    
                    if self.re_ui.large_button(ui, "🌐 打开浏览器").clicked() {
                        let _ = open::that(address);
                    }
                }
                ServerStatus::Starting | ServerStatus::Stopping => {
                    ui.add_enabled(false, egui::Button::new("处理中..."));
                }
                ServerStatus::Error(msg) => {
                    ui.colored_label(egui::Color32::RED, format!("错误: {}", msg));
                    
                    if self.re_ui.large_button(ui, "🔄 重试").clicked() {
                        self.deploy_and_start();
                    }
                }
            }
            
            if self.re_ui.large_button(ui, "💾 保存配置").clicked() {
                self.save_config();
            }
        });
        
        ui.separator();
        
        // 日志输出
        self.re_ui.collapsing_header(ui, "服务器日志", true, |ui| {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &self.logs {
                        ui.label(log);
                    }
                });
        });
    }
    
    fn render_status_card(&self, ui: &mut egui::Ui, title: &str, status: &impl std::fmt::Display) {
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(title);
                    ui.heading(status.to_string());
                });
            });
    }
    
    fn deploy_and_start(&mut self) {
        self.deploy_status = DeployStatus::Deploying;
        self.server_status = ServerStatus::Starting;
        
        let config = self.deploy_config.clone();
        
        tokio::spawn(async move {
            // 1. 检查配置
            if !std::path::Path::new(&config.db_path).exists() {
                // 创建数据库
            }
            
            // 2. 写入 DbOption.toml
            let toml_content = format!(
                r#"
[server]
host = "{}"
port = {}
db_path = "{}"
static_dir = "{}"
enable_cors = {}
log_level = "{}"
"#,
                config.server_host,
                config.server_port,
                config.db_path,
                config.static_dir,
                config.enable_cors,
                config.log_level
            );
            
            std::fs::write("DbOption.toml", toml_content).unwrap();
            
            // 3. 启动 Web Server
            match start_web_server(config).await {
                Ok(address) => {
                    // 更新状态为 Running
                }
                Err(e) => {
                    // 更新状态为 Error
                }
            }
        });
    }
    
    fn stop_server(&mut self) {
        self.server_status = ServerStatus::Stopping;
        
        tokio::spawn(async move {
            stop_web_server().await;
        });
    }
    
    fn restart_server(&mut self) {
        self.stop_server();
        
        // 等待停止完成后重新启动
        tokio::time::sleep(std::time::Duration::from_secs(2));
        self.deploy_and_start();
    }
    
    fn save_config(&self) {
        let toml_content = toml::to_string(&self.deploy_config).unwrap();
        std::fs::write("DbOption.toml", toml_content).unwrap();
    }
}

impl std::fmt::Display for DeployStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeployStatus::NotStarted => write!(f, "⚪ 未开始"),
            DeployStatus::Deploying => write!(f, "🟡 部署中..."),
            DeployStatus::Deployed => write!(f, "🟢 已部署"),
            DeployStatus::Failed(msg) => write!(f, "🔴 失败: {}", msg),
        }
    }
}

// Web Server 启动函数（集成现有代码）
async fn start_web_server(config: DeployConfig) -> anyhow::Result<String> {
    use crate::web_server;
    
    // 创建 Axum 应用
    let app = web_server::create_app(&config.db_path, &config.static_dir, config.enable_cors)?;
    
    // 绑定地址
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    // 启动服务器（在后台线程）
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    
    Ok(format!("http://{}", addr))
}

async fn stop_web_server() {
    // 发送停止信号
    // 这里需要与现有的 web_server 模块集成
}
```

## API 客户端扩展

```rust
impl ApiClient {
    // 解析任务 API
    pub async fn get_parse_tasks(&self) -> anyhow::Result<Vec<ParseTask>> {
        let url = format!("{}/api/parse/tasks", self.base_url);
        let response = self.client.get(&url).send().await?;
        let tasks: Vec<ParseTask> = response.json().await?;
        Ok(tasks)
    }
    
    pub async fn create_parse_task(&self, task: &ParseTask) -> anyhow::Result<String> {
        let url = format!("{}/api/parse/tasks", self.base_url);
        let response = self.client.post(&url).json(task).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["id"].as_str().unwrap().to_string())
    }
    
    pub async fn start_parse_task(&self, task_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/parse/tasks/{}/start", self.base_url, task_id);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn cancel_parse_task(&self, task_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/parse/tasks/{}/cancel", self.base_url, task_id);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn delete_parse_task(&self, task_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/parse/tasks/{}", self.base_url, task_id);
        self.client.delete(&url).send().await?;
        Ok(())
    }
    
    // 模型生成 API
    pub async fn get_model_gen_configs(&self) -> anyhow::Result<Vec<ModelGenConfig>> {
        let url = format!("{}/api/model-gen/configs", self.base_url);
        let response = self.client.get(&url).send().await?;
        let configs: Vec<ModelGenConfig> = response.json().await?;
        Ok(configs)
    }
    
    pub async fn save_model_gen_config(&self, config: &ModelGenConfig) -> anyhow::Result<String> {
        let url = format!("{}/api/model-gen/configs", self.base_url);
        let response = self.client.post(&url).json(config).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["id"].as_str().unwrap().to_string())
    }
    
    pub async fn delete_model_gen_config(&self, config_id: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/model-gen/configs/{}", self.base_url, config_id);
        self.client.delete(&url).send().await?;
        Ok(())
    }
    
    pub async fn start_model_generation(&self, config_id: &str) -> anyhow::Result<String> {
        let url = format!("{}/api/model-gen/generate/{}", self.base_url, config_id);
        let response = self.client.post(&url).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["task_id"].as_str().unwrap().to_string())
    }
    
    pub async fn get_generation_tasks(&self, config_id: &str) -> anyhow::Result<Vec<GenerationTask>> {
        let url = format!("{}/api/model-gen/tasks?config_id={}", self.base_url, config_id);
        let response = self.client.get(&url).send().await?;
        let tasks: Vec<GenerationTask> = response.json().await?;
        Ok(tasks)
    }
}
```

## 更新主应用结构

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    EnvironmentList,
    TopologyCanvas,
    MonitorDashboard,
    LogQuery,
    WebServer,
    Deployment,
    ParseTask,        // 新增
    ModelGen,         // 新增
    QuickDeploy,      // 新增
    Settings,
}

impl EguiRemoteSyncApp {
    fn render_navigation_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("导航");
        ui.separator();
        
        // 异地协同分组
        self.re_ui.collapsing_header(ui, "异地协同", true, |ui| {
            self.render_nav_item(ui, "🌐 环境列表", Page::EnvironmentList);
            self.render_nav_item(ui, "🗺️ 拓扑配置", Page::TopologyCanvas);
            self.render_nav_item(ui, "📊 实时监控", Page::MonitorDashboard);
            self.render_nav_item(ui, "📝 日志查询", Page::LogQuery);
        });
        
        ui.separator();
        
        // 数据处理分组
        self.re_ui.collapsing_header(ui, "数据处理", true, |ui| {
            self.render_nav_item(ui, "📋 解析任务", Page::ParseTask);
            self.render_nav_item(ui, "🎨 模型生成", Page::ModelGen);
        });
        
        ui.separator();
        
        // 系统管理分组
        self.re_ui.collapsing_header(ui, "系统管理", true, |ui| {
            self.render_nav_item(ui, "🚀 一键部署", Page::QuickDeploy);
            self.render_nav_item(ui, "🖥️ 服务器管理", Page::WebServer);
            self.render_nav_item(ui, "⚙️ 设置", Page::Settings);
        });
    }
    
    fn render_nav_item(&mut self, ui: &mut egui::Ui, label: &str, page: Page) {
        self.re_ui.list_item()
            .selected(self.current_page == page)
            .show_flat(ui, |ui| {
                if ui.selectable_label(false, label).clicked() {
                    self.current_page = page;
                }
            });
    }
}
```

## 后端 API 端点（需要实现）

### 解析任务 API

- `GET /api/parse/tasks` - 获取所有解析任务
- `POST /api/parse/tasks` - 创建解析任务
- `POST /api/parse/tasks/{id}/start` - 启动解析任务
- `POST /api/parse/tasks/{id}/cancel` - 取消解析任务
- `DELETE /api/parse/tasks/{id}` - 删除解析任务
- `GET /api/parse/tasks/{id}/status` - 获取任务状态

### 模型生成 API

- `GET /api/model-gen/configs` - 获取所有配置
- `POST /api/model-gen/configs` - 创建/更新配置
- `DELETE /api/model-gen/configs/{id}` - 删除配置
- `POST /api/model-gen/generate/{config_id}` - 开始生成
- `GET /api/model-gen/tasks?config_id={id}` - 获取生成任务列表
- `GET /api/model-gen/tasks/{id}/status` - 获取任务状态

### 服务器管理 API

- `POST /api/server/start` - 启动服务器
- `POST /api/server/stop` - 停止服务器
- `POST /api/server/restart` - 重启服务器
- `GET /api/server/status` - 获取服务器状态
- `GET /api/server/logs` - 获取服务器日志
