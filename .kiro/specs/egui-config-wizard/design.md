# Design Document

## Overview

本设计文档描述了为 egui 原生界面添加配置向导功能的技术架构、组件设计和实现方案。该功能将参考 frontend/v0-aios-database-management 的任务创建向导，提供数据解析、模型生成、空间树生成等任务的配置界面，并支持数据库启动管理。

### 设计目标

1. **向导式交互** - 提供分步骤的任务创建流程，降低配置复杂度
2. **参数验证** - 实时验证用户输入，提供清晰的错误提示
3. **模板复用** - 支持保存和加载任务配置模板，提高效率
4. **批量操作** - 支持批量创建多个相似任务
5. **数据库集成** - 提供 SurrealDB 启动和管理界面
6. **配置管理** - 支持解析和生成 DbOption.toml 配置文件

### 技术栈

**GUI 框架**
- egui 0.33 (即时模式 GUI)
- eframe (egui 的应用框架)
- egui_extras (表格、图表等扩展组件)

**网络和数据**
- reqwest (HTTP 客户端，调用 REST API)
- tokio (异步运行时)
- serde_json (JSON 序列化)
- toml (TOML 配置文件解析)

**其他依赖**
- chrono (时间处理)
- anyhow (错误处理)
- uuid (生成唯一 ID)
- rfd (原生文件对话框)

## Architecture

### 应用架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      EguiRemoteSyncApp                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 新增页面                                                  │  │
│  │ - TaskCreationPage (任务创建向导)                         │  │
│  │ - TaskMonitorPage (任务监控)                              │  │
│  │ - DatabaseManagePage (数据库管理)                         │  │
│  │ - ConfigEditorPage (配置编辑)                             │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 新增组件                                                  │  │
│  │ - TaskCreationWizard (任务创建向导)                       │  │
│  │ - TaskParametersForm (任务参数表单)                       │  │
│  │ - TaskPreview (任务预览)                                  │  │
│  │ - DatabaseControl (数据库控制)                            │  │
│  │ - ConfigEditor (配置编辑器)                               │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP REST API
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Axum Web Server                            │
│  /api/tasks (POST, GET, DELETE)                                 │
│  /api/tasks/{id}/cancel (POST)                                  │
│  /api/deployment-sites (GET)                                    │
│  /api/task-templates (GET, POST, DELETE)                        │
└─────────────────────────────────────────────────────────────────┘
```

### 目录结构

```
src/gui/
├── pages/
│   ├── task_creation.rs      # 任务创建页面
│   ├── task_monitor.rs       # 任务监控页面
│   ├── database_manage.rs    # 数据库管理页面
│   └── config_editor.rs      # 配置编辑页面
├── components/
│   ├── task_wizard.rs        # 任务创建向导组件
│   ├── task_params.rs        # 任务参数表单组件
│   ├── task_preview.rs       # 任务预览组件
│   ├── db_control.rs         # 数据库控制组件
│   └── config_editor.rs      # 配置编辑器组件
└── state.rs                  # 扩展全局状态
```


## Components and Interfaces

### 1. 任务创建页面 (TaskCreationPage)

```rust
pub struct TaskCreationPage {
    wizard: TaskCreationWizard,
    templates: Vec<TaskTemplate>,
    show_template_dialog: bool,
}

impl TaskCreationPage {
    pub fn new() -> Self {
        Self {
            wizard: TaskCreationWizard::new(),
            templates: Vec::new(),
            show_template_dialog: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, api_client: &ApiClient) {
        ui.heading("创建任务");
        
        // 模板选择
        ui.horizontal(|ui| {
            if ui.button("📋 从模板创建").clicked() {
                self.show_template_dialog = true;
            }
            
            if ui.button("💾 保存为模板").clicked() {
                self.save_as_template();
            }
        });
        
        ui.separator();
        
        // 任务创建向导
        self.wizard.render(ui, state, api_client);
        
        // 模板选择对话框
        if self.show_template_dialog {
            self.render_template_dialog(ui.ctx());
        }
    }
    
    fn render_template_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("选择模板")
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                for template in &self.templates {
                    ui.horizontal(|ui| {
                        if ui.button(&template.name).clicked() {
                            self.wizard.load_from_template(template);
                            self.show_template_dialog = false;
                        }
                        ui.label(&template.description);
                    });
                }
                
                if ui.button("取消").clicked() {
                    self.show_template_dialog = false;
                }
            });
    }
    
    fn save_as_template(&mut self) {
        // 保存当前配置为模板
        let template = self.wizard.to_template();
        self.templates.push(template);
        // 持久化到配置文件
        self.save_templates_to_file();
    }
    
    fn save_templates_to_file(&self) {
        let config_dir = dirs::config_dir()
            .unwrap()
            .join("egui_remote_sync");
        std::fs::create_dir_all(&config_dir).ok();
        
        let template_file = config_dir.join("task_templates.json");
        let json = serde_json::to_string_pretty(&self.templates).unwrap();
        std::fs::write(template_file, json).ok();
    }
}
```

### 2. 任务创建向导 (TaskCreationWizard)

```rust
pub struct TaskCreationWizard {
    current_step: WizardStep,
    form_data: TaskCreationFormData,
    validation_errors: HashMap<String, String>,
    selected_site: Option<String>,
    batch_mode: bool,
    selected_sites: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WizardStep {
    BasicInfo,      // 步骤 1: 基础信息
    SelectSite,     // 步骤 2: 选择站点
    Parameters,     // 步骤 3: 任务参数
    Preview,        // 步骤 4: 预览确认
}

#[derive(Debug, Clone)]
pub struct TaskCreationFormData {
    pub task_name: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub description: String,
    pub parameters: TaskParameters,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    DataParsing,
    ModelGeneration,
    SpatialTreeGeneration,
    FullSync,
    IncrementalSync,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone)]
pub enum TaskParameters {
    DataParsing {
        parse_mode: ParseMode,
        db_nums: String,
        refno: String,
    },
    ModelGeneration {
        generate_3d: bool,
        generate_mesh: bool,
        generate_spatial_tree: bool,
        generate_boolean: bool,
        mesh_tolerance: f32,
        max_concurrency: u32,
        parallel_processing: bool,
    },
    SpatialTreeGeneration {
        tree_depth: u32,
        node_capacity: u32,
        index_type: SpatialIndexType,
    },
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseMode {
    All,
    SpecificDbNums,
    SpecificRefno,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpatialIndexType {
    RTree,
    QuadTree,
    OctTree,
}

impl TaskCreationWizard {
    pub fn new() -> Self {
        Self {
            current_step: WizardStep::BasicInfo,
            form_data: TaskCreationFormData {
                task_name: String::new(),
                task_type: TaskType::DataParsing,
                priority: TaskPriority::Normal,
                description: String::new(),
                parameters: TaskParameters::None,
            },
            validation_errors: HashMap::new(),
            selected_site: None,
            batch_mode: false,
            selected_sites: Vec::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, api_client: &ApiClient) {
        // 步骤指示器
        self.render_step_indicator(ui);
        
        ui.separator();
        
        // 当前步骤内容
        match self.current_step {
            WizardStep::BasicInfo => self.render_basic_info(ui),
            WizardStep::SelectSite => self.render_site_selection(ui, state),
            WizardStep::Parameters => self.render_parameters(ui),
            WizardStep::Preview => self.render_preview(ui, state),
        }
        
        ui.separator();
        
        // 导航按钮
        self.render_navigation_buttons(ui, api_client);
    }
    
    fn render_step_indicator(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let steps = [
                ("1. 基础信息", WizardStep::BasicInfo),
                ("2. 选择站点", WizardStep::SelectSite),
                ("3. 任务参数", WizardStep::Parameters),
                ("4. 预览确认", WizardStep::Preview),
            ];
            
            for (i, (label, step)) in steps.iter().enumerate() {
                if i > 0 {
                    ui.label("→");
                }
                
                let is_current = &self.current_step == step;
                let is_completed = self.is_step_completed(step);
                
                let color = if is_current {
                    egui::Color32::BLUE
                } else if is_completed {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::GRAY
                };
                
                ui.colored_label(color, *label);
            }
        });
    }
    
    fn render_basic_info(&mut self, ui: &mut egui::Ui) {
        ui.heading("基础信息");
        
        egui::Grid::new("basic_info_grid")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                // 任务名称
                ui.label("任务名称 *:");
                ui.text_edit_singleline(&mut self.form_data.task_name);
                ui.end_row();
                
                if let Some(error) = self.validation_errors.get("task_name") {
                    ui.label("");
                    ui.colored_label(egui::Color32::RED, error);
                    ui.end_row();
                }
                
                // 任务类型
                ui.label("任务类型 *:");
                egui::ComboBox::from_id_source("task_type")
                    .selected_text(self.get_task_type_label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.form_data.task_type, TaskType::DataParsing, "📊 数据解析任务");
                        ui.selectable_value(&mut self.form_data.task_type, TaskType::ModelGeneration, "🎨 模型生成任务");
                        ui.selectable_value(&mut self.form_data.task_type, TaskType::SpatialTreeGeneration, "🌳 空间树生成任务");
                        ui.selectable_value(&mut self.form_data.task_type, TaskType::FullSync, "🔄 全量同步任务");
                        ui.selectable_value(&mut self.form_data.task_type, TaskType::IncrementalSync, "⚡ 增量同步任务");
                    });
                ui.end_row();
                
                // 任务描述
                ui.label("任务描述:");
                ui.text_edit_multiline(&mut self.form_data.description);
                ui.end_row();
                
                // 优先级
                ui.label("优先级:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.form_data.priority, TaskPriority::Low, "低");
                    ui.selectable_value(&mut self.form_data.priority, TaskPriority::Normal, "普通");
                    ui.selectable_value(&mut self.form_data.priority, TaskPriority::High, "高");
                    ui.selectable_value(&mut self.form_data.priority, TaskPriority::Urgent, "紧急");
                });
                ui.end_row();
            });
    }

    fn render_site_selection(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.heading("选择站点");
        
        // 批量模式开关
        ui.checkbox(&mut self.batch_mode, "批量模式（选择多个站点）");
        
        ui.separator();
        
        // 站点列表
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(40.0))  // 选择框
            .column(Column::auto().at_least(150.0)) // 站点名称
            .column(Column::auto().at_least(100.0)) // 环境
            .column(Column::auto().at_least(100.0)) // 地区
            .column(Column::auto().at_least(80.0))  // 状态
            .column(Column::remainder())            // 操作
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong(""); });
                header.col(|ui| { ui.strong("站点名称"); });
                header.col(|ui| { ui.strong("环境"); });
                header.col(|ui| { ui.strong("地区"); });
                header.col(|ui| { ui.strong("状态"); });
                header.col(|ui| { ui.strong("操作"); });
            })
            .body(|mut body| {
                for site in &state.sites {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            if self.batch_mode {
                                let mut is_selected = self.selected_sites.contains(&site.id);
                                if ui.checkbox(&mut is_selected, "").changed() {
                                    if is_selected {
                                        self.selected_sites.push(site.id.clone());
                                    } else {
                                        self.selected_sites.retain(|id| id != &site.id);
                                    }
                                }
                            } else {
                                let is_selected = self.selected_site.as_ref() == Some(&site.id);
                                if ui.radio(is_selected, "").clicked() {
                                    self.selected_site = Some(site.id.clone());
                                }
                            }
                        });
                        
                        row.col(|ui| { ui.label(&site.name); });
                        
                        row.col(|ui| {
                            let env_name = state.environments
                                .iter()
                                .find(|e| e.id == site.env_id)
                                .map(|e| e.name.as_str())
                                .unwrap_or("未知");
                            ui.label(env_name);
                        });
                        
                        row.col(|ui| { ui.label(&site.location); });
                        
                        row.col(|ui| {
                            ui.colored_label(egui::Color32::GREEN, "🟢 在线");
                        });
                        
                        row.col(|ui| {
                            if ui.small_button("测试连接").clicked() {
                                // 测试站点连接
                            }
                        });
                    });
                }
            });
    }
    
    fn render_parameters(&mut self, ui: &mut egui::Ui) {
        ui.heading("任务参数");
        
        match self.form_data.task_type {
            TaskType::DataParsing => self.render_data_parsing_params(ui),
            TaskType::ModelGeneration => self.render_model_generation_params(ui),
            TaskType::SpatialTreeGeneration => self.render_spatial_tree_params(ui),
            _ => {
                ui.label("此任务类型暂无额外参数");
            }
        }
    }
    
    fn render_data_parsing_params(&mut self, ui: &mut egui::Ui) {
        if let TaskParameters::DataParsing { parse_mode, db_nums, refno } = &mut self.form_data.parameters {
            egui::Grid::new("data_parsing_params")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("解析模式:");
                    ui.vertical(|ui| {
                        ui.radio_value(parse_mode, ParseMode::All, "全部解析");
                        ui.radio_value(parse_mode, ParseMode::SpecificDbNums, "指定数据库编号");
                        ui.radio_value(parse_mode, ParseMode::SpecificRefno, "指定参考号");
                    });
                    ui.end_row();
                    
                    if *parse_mode == ParseMode::SpecificDbNums {
                        ui.label("数据库编号:");
                        ui.text_edit_singleline(db_nums);
                        ui.end_row();
                        
                        ui.label("");
                        ui.label("(逗号分隔，如: 7999,8001,8002)");
                        ui.end_row();
                    }
                    
                    if *parse_mode == ParseMode::SpecificRefno {
                        ui.label("参考号:");
                        ui.text_edit_singleline(refno);
                        ui.end_row();
                    }
                });
        } else {
            self.form_data.parameters = TaskParameters::DataParsing {
                parse_mode: ParseMode::All,
                db_nums: String::new(),
                refno: String::new(),
            };
        }
    }
    
    fn render_model_generation_params(&mut self, ui: &mut egui::Ui) {
        if let TaskParameters::ModelGeneration {
            generate_3d,
            generate_mesh,
            generate_spatial_tree,
            generate_boolean,
            mesh_tolerance,
            max_concurrency,
            parallel_processing,
        } = &mut self.form_data.parameters {
            egui::Grid::new("model_generation_params")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("生成选项:");
                    ui.vertical(|ui| {
                        ui.checkbox(generate_3d, "生成 3D 模型");
                        ui.checkbox(generate_mesh, "生成网格");
                        ui.checkbox(generate_spatial_tree, "生成空间树");
                        ui.checkbox(generate_boolean, "生成布尔运算");
                    });
                    ui.end_row();
                    
                    ui.label("网格容差比例:");
                    ui.add(egui::Slider::new(mesh_tolerance, 0.001..=1.0).text(""));
                    ui.end_row();
                    
                    ui.label("最大并发数:");
                    ui.add(egui::DragValue::new(max_concurrency).clamp_range(1..=32));
                    ui.end_row();
                    
                    ui.label("并行处理:");
                    ui.checkbox(parallel_processing, "启用");
                    ui.end_row();
                });
        } else {
            self.form_data.parameters = TaskParameters::ModelGeneration {
                generate_3d: true,
                generate_mesh: true,
                generate_spatial_tree: false,
                generate_boolean: false,
                mesh_tolerance: 0.01,
                max_concurrency: 4,
                parallel_processing: false,
            };
        }
    }
    
    fn render_spatial_tree_params(&mut self, ui: &mut egui::Ui) {
        if let TaskParameters::SpatialTreeGeneration {
            tree_depth,
            node_capacity,
            index_type,
        } = &mut self.form_data.parameters {
            egui::Grid::new("spatial_tree_params")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("树深度:");
                    ui.add(egui::Slider::new(tree_depth, 1..=10));
                    ui.end_row();
                    
                    ui.label("节点容量:");
                    ui.add(egui::DragValue::new(node_capacity).clamp_range(10..=1000));
                    ui.end_row();
                    
                    ui.label("索引类型:");
                    egui::ComboBox::from_id_source("index_type")
                        .selected_text(format!("{:?}", index_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(index_type, SpatialIndexType::RTree, "R-Tree");
                            ui.selectable_value(index_type, SpatialIndexType::QuadTree, "Quad-Tree");
                            ui.selectable_value(index_type, SpatialIndexType::OctTree, "Oct-Tree");
                        });
                    ui.end_row();
                });
        } else {
            self.form_data.parameters = TaskParameters::SpatialTreeGeneration {
                tree_depth: 5,
                node_capacity: 100,
                index_type: SpatialIndexType::RTree,
            };
        }
    }
    
    fn render_preview(&self, ui: &mut egui::Ui, state: &AppState) {
        ui.heading("任务预览");
        
        egui::Grid::new("task_preview")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                ui.strong("任务名称:");
                ui.label(&self.form_data.task_name);
                ui.end_row();
                
                ui.strong("任务类型:");
                ui.label(self.get_task_type_label());
                ui.end_row();
                
                ui.strong("优先级:");
                ui.label(format!("{:?}", self.form_data.priority));
                ui.end_row();
                
                if !self.form_data.description.is_empty() {
                    ui.strong("描述:");
                    ui.label(&self.form_data.description);
                    ui.end_row();
                }
                
                ui.strong("目标站点:");
                if self.batch_mode {
                    ui.label(format!("{} 个站点", self.selected_sites.len()));
                } else if let Some(site_id) = &self.selected_site {
                    let site_name = state.sites
                        .iter()
                        .find(|s| &s.id == site_id)
                        .map(|s| s.name.as_str())
                        .unwrap_or("未知");
                    ui.label(site_name);
                } else {
                    ui.label("未选择");
                }
                ui.end_row();
            });
        
        ui.separator();
        
        // 参数摘要
        ui.strong("任务参数:");
        match &self.form_data.parameters {
            TaskParameters::DataParsing { parse_mode, db_nums, refno } => {
                ui.label(format!("解析模式: {:?}", parse_mode));
                if *parse_mode == ParseMode::SpecificDbNums {
                    ui.label(format!("数据库编号: {}", db_nums));
                }
                if *parse_mode == ParseMode::SpecificRefno {
                    ui.label(format!("参考号: {}", refno));
                }
            }
            TaskParameters::ModelGeneration { generate_3d, generate_mesh, mesh_tolerance, max_concurrency, .. } => {
                ui.label(format!("生成 3D: {}, 生成网格: {}", generate_3d, generate_mesh));
                ui.label(format!("网格容差: {}, 最大并发: {}", mesh_tolerance, max_concurrency));
            }
            TaskParameters::SpatialTreeGeneration { tree_depth, node_capacity, index_type } => {
                ui.label(format!("树深度: {}, 节点容量: {}", tree_depth, node_capacity));
                ui.label(format!("索引类型: {:?}", index_type));
            }
            _ => {}
        }
        
        ui.separator();
        
        // 资源需求预估
        ui.strong("资源需求预估:");
        ui.label("预计内存使用: ~2GB");
        ui.label("预计磁盘空间: ~500MB");
        ui.label("预计耗时: 10-30 分钟");
        
        ui.separator();
        
        // 注意事项
        ui.strong("⚠️ 注意事项:");
        ui.label("• 大型数据库解析可能需要较长时间");
        ui.label("• 请确保目标站点有足够的磁盘空间");
        ui.label("• 任务执行期间请勿关闭应用");
    }
    
    fn render_navigation_buttons(&mut self, ui: &mut egui::Ui, api_client: &ApiClient) {
        ui.horizontal(|ui| {
            // 上一步按钮
            if self.current_step != WizardStep::BasicInfo {
                if ui.button("⬅️ 上一步").clicked() {
                    self.go_to_previous_step();
                }
            }
            
            // 下一步/创建按钮
            if self.current_step == WizardStep::Preview {
                if ui.button("✅ 创建任务").clicked() {
                    self.create_task(api_client);
                }
            } else {
                let can_proceed = self.validate_current_step();
                if ui.add_enabled(can_proceed, egui::Button::new("下一步 ➡️")).clicked() {
                    self.go_to_next_step();
                }
            }
            
            // 取消按钮
            if ui.button("❌ 取消").clicked() {
                self.reset();
            }
        });
    }
    
    fn validate_current_step(&mut self) -> bool {
        self.validation_errors.clear();
        
        match self.current_step {
            WizardStep::BasicInfo => {
                if self.form_data.task_name.trim().is_empty() {
                    self.validation_errors.insert("task_name".to_string(), "任务名称不能为空".to_string());
                    return false;
                }
                true
            }
            WizardStep::SelectSite => {
                if self.batch_mode {
                    !self.selected_sites.is_empty()
                } else {
                    self.selected_site.is_some()
                }
            }
            WizardStep::Parameters => true,
            WizardStep::Preview => true,
        }
    }
    
    fn go_to_next_step(&mut self) {
        self.current_step = match self.current_step {
            WizardStep::BasicInfo => WizardStep::SelectSite,
            WizardStep::SelectSite => WizardStep::Parameters,
            WizardStep::Parameters => WizardStep::Preview,
            WizardStep::Preview => WizardStep::Preview,
        };
    }
    
    fn go_to_previous_step(&mut self) {
        self.current_step = match self.current_step {
            WizardStep::BasicInfo => WizardStep::BasicInfo,
            WizardStep::SelectSite => WizardStep::BasicInfo,
            WizardStep::Parameters => WizardStep::SelectSite,
            WizardStep::Preview => WizardStep::Parameters,
        };
    }
    
    fn is_step_completed(&self, step: &WizardStep) -> bool {
        match step {
            WizardStep::BasicInfo => !self.form_data.task_name.is_empty(),
            WizardStep::SelectSite => self.selected_site.is_some() || !self.selected_sites.is_empty(),
            WizardStep::Parameters => true,
            WizardStep::Preview => false,
        }
    }
    
    fn get_task_type_label(&self) -> &str {
        match self.form_data.task_type {
            TaskType::DataParsing => "📊 数据解析任务",
            TaskType::ModelGeneration => "🎨 模型生成任务",
            TaskType::SpatialTreeGeneration => "🌳 空间树生成任务",
            TaskType::FullSync => "🔄 全量同步任务",
            TaskType::IncrementalSync => "⚡ 增量同步任务",
        }
    }
    
    fn create_task(&self, api_client: &ApiClient) {
        let task_request = self.to_task_request();
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            match api_client.create_task(&task_request).await {
                Ok(task_id) => {
                    // 显示成功提示
                    println!("任务创建成功: {}", task_id);
                }
                Err(e) => {
                    // 显示错误提示
                    eprintln!("任务创建失败: {}", e);
                }
            }
        });
    }
    
    fn to_task_request(&self) -> TaskRequest {
        TaskRequest {
            task_name: self.form_data.task_name.clone(),
            task_type: format!("{:?}", self.form_data.task_type),
            priority: format!("{:?}", self.form_data.priority),
            description: self.form_data.description.clone(),
            site_ids: if self.batch_mode {
                self.selected_sites.clone()
            } else {
                vec![self.selected_site.clone().unwrap_or_default()]
            },
            parameters: serde_json::to_value(&self.form_data.parameters).unwrap(),
        }
    }
    
    fn reset(&mut self) {
        *self = Self::new();
    }
    
    pub fn load_from_template(&mut self, template: &TaskTemplate) {
        self.form_data.task_type = template.task_type.clone();
        self.form_data.priority = template.priority.clone();
        self.form_data.parameters = template.parameters.clone();
    }
    
    pub fn to_template(&self) -> TaskTemplate {
        TaskTemplate {
            name: format!("{} 模板", self.form_data.task_name),
            description: self.form_data.description.clone(),
            task_type: self.form_data.task_type.clone(),
            priority: self.form_data.priority.clone(),
            parameters: self.form_data.parameters.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_name: String,
    pub task_type: String,
    pub priority: String,
    pub description: String,
    pub site_ids: Vec<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub parameters: TaskParameters,
}
```

### 3. 数据库管理页面 (DatabaseManagePage)

```rust
pub struct DatabaseManagePage {
    config: SurrealDBConfig,
    status: DatabaseStatus,
    logs: Vec<String>,
    process_handle: Option<std::process::Child>,
}

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
        
        // 状态显示
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
        
        // 配置表单
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
        
        // 操作按钮
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
        
        // 日志输出
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
        
        // 构建启动命令
        let command = format!(
            "surreal start --bind {}:{} --user {} --pass {} file://{}",
            self.config.host,
            self.config.port,
            self.config.username,
            self.config.password,
            self.config.data_path
        );
        
        self.logs.push(format!("[{}] 命令: {}", chrono::Local::now().format("%H:%M:%S"), command));
        
        // 启动进程
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
                    version: "1.0.0".to_string(), // TODO: 获取实际版本
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
        
        let config = self.config.clone();
        
        tokio::spawn(async move {
            // TODO: 实现 SurrealDB 连接测试
            // 使用 surrealdb crate 连接数据库
        });
    }
    
    fn save_config(&self) {
        // 保存配置到 DbOption.toml
        if let Ok(toml_content) = toml::to_string(&self.config) {
            if let Err(e) = std::fs::write("DbOption.toml", toml_content) {
                eprintln!("保存配置失败: {}", e);
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
```

### 4. 任务监控页面 (TaskMonitorPage)

```rust
pub struct TaskMonitorPage {
    tasks: Vec<TaskInfo>,
    selected_task: Option<String>,
    show_task_detail: bool,
    auto_refresh: bool,
    last_refresh: std::time::Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub progress: u8,
    pub current_step: String,
    pub started_at: String,
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

impl TaskMonitorPage {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_task: None,
            show_task_detail: false,
            auto_refresh: true,
            last_refresh: std::time::Instant::now(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, api_client: &ApiClient) {
        ui.heading("任务监控");
        
        ui.horizontal(|ui| {
            if ui.button("🔄 刷新").clicked() {
                self.refresh_tasks(api_client);
            }
            
            ui.checkbox(&mut self.auto_refresh, "自动刷新 (5秒)");
            
            ui.label(format!(
                "上次刷新: {} 秒前",
                self.last_refresh.elapsed().as_secs()
            ));
        });
        
        ui.separator();
        
        // 任务列表
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(150.0)) // 任务名称
            .column(Column::auto().at_least(100.0)) // 任务类型
            .column(Column::auto().at_least(80.0))  // 状态
            .column(Column::auto().at_least(150.0)) // 进度
            .column(Column::remainder())            // 操作
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong("任务名称"); });
                header.col(|ui| { ui.strong("任务类型"); });
                header.col(|ui| { ui.strong("状态"); });
                header.col(|ui| { ui.strong("进度"); });
                header.col(|ui| { ui.strong("操作"); });
            })
            .body(|mut body| {
                for task in &self.tasks {
                    body.row(30.0, |mut row| {
                        row.col(|ui| { ui.label(&task.name); });
                        row.col(|ui| { ui.label(&task.task_type); });
                        row.col(|ui| {
                            let (color, text) = match task.status {
                                TaskStatus::Pending => (egui::Color32::GRAY, "⏸️ 等待中"),
                                TaskStatus::Running => (egui::Color32::BLUE, "🟢 运行中"),
                                TaskStatus::Completed => (egui::Color32::GREEN, "✅ 完成"),
                                TaskStatus::Failed => (egui::Color32::RED, "❌ 失败"),
                                TaskStatus::Cancelled => (egui::Color32::GRAY, "⚪ 已取消"),
                            };
                            ui.colored_label(color, text);
                        });
                        row.col(|ui| {
                            let progress = task.progress as f32 / 100.0;
                            ui.add(egui::ProgressBar::new(progress).text(format!("{}% - {}", task.progress, task.current_step)));
                        });
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if ui.small_button("详情").clicked() {
                                    self.selected_task = Some(task.id.clone());
                                    self.show_task_detail = true;
                                }
                                
                                if task.status == TaskStatus::Running {
                                    if ui.small_button("取消").clicked() {
                                        self.cancel_task(&task.id, api_client);
                                    }
                                }
                                
                                if task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
                                    if ui.small_button("删除").clicked() {
                                        self.delete_task(&task.id, api_client);
                                    }
                                }
                            });
                        });
                    });
                }
            });
        
        // 任务详情对话框
        if self.show_task_detail {
            self.render_task_detail(ui.ctx());
        }
        
        // 自动刷新
        if self.auto_refresh && self.last_refresh.elapsed().as_secs() >= 5 {
            self.refresh_tasks(api_client);
        }
    }
    
    fn render_task_detail(&mut self, ctx: &egui::Context) {
        egui::Window::new("任务详情")
            .collapsible(false)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                if let Some(task_id) = &self.selected_task {
                    if let Some(task) = self.tasks.iter().find(|t| &t.id == task_id) {
                        egui::Grid::new("task_detail")
                            .num_columns(2)
                            .spacing([10.0, 10.0])
                            .show(ui, |ui| {
                                ui.label("任务 ID:");
                                ui.label(&task.id);
                                ui.end_row();
                                
                                ui.label("任务名称:");
                                ui.label(&task.name);
                                ui.end_row();
                                
                                ui.label("任务类型:");
                                ui.label(&task.task_type);
                                ui.end_row();
                                
                                ui.label("状态:");
                                ui.label(format!("{:?}", task.status));
                                ui.end_row();
                                
                                ui.label("进度:");
                                ui.label(format!("{}%", task.progress));
                                ui.end_row();
                                
                                ui.label("当前步骤:");
                                ui.label(&task.current_step);
                                ui.end_row();
                                
                                ui.label("开始时间:");
                                ui.label(&task.started_at);
                                ui.end_row();
                                
                                if let Some(completed_at) = &task.completed_at {
                                    ui.label("完成时间:");
                                    ui.label(completed_at);
                                    ui.end_row();
                                }
                                
                                if let Some(error) = &task.error_message {
                                    ui.label("错误信息:");
                                    ui.colored_label(egui::Color32::RED, error);
                                    ui.end_row();
                                }
                            });
                    }
                }
                
                if ui.button("关闭").clicked() {
                    self.show_task_detail = false;
                }
            });
    }
    
    fn refresh_tasks(&mut self, api_client: &ApiClient) {
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            // TODO: 调用 API 获取任务列表
            // let tasks = api_client.get_tasks().await?;
        });
        
        self.last_refresh = std::time::Instant::now();
    }
    
    fn cancel_task(&self, task_id: &str, api_client: &ApiClient) {
        let task_id = task_id.to_string();
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            // TODO: 调用 API 取消任务
            // api_client.cancel_task(&task_id).await?;
        });
    }
    
    fn delete_task(&mut self, task_id: &str, api_client: &ApiClient) {
        let task_id = task_id.to_string();
        let api_client = api_client.clone();
        
        tokio::spawn(async move {
            // TODO: 调用 API 删除任务
            // api_client.delete_task(&task_id).await?;
        });
        
        self.tasks.retain(|t| t.id != task_id);
    }
}
```

### 5. 配置编辑页面 (ConfigEditorPage)

```rust
pub struct ConfigEditorPage {
    config: DbOptionConfig,
    config_text: String,
    edit_mode: EditMode,
    validation_errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditMode {
    Form,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOptionConfig {
    // 数据库配置
    pub db_path: String,
    pub surreal_host: String,
    pub surreal_port: u16,
    
    // Web Server 配置
    pub web_host: String,
    pub web_port: u16,
    
    // 其他配置
    pub log_level: String,
    pub max_connections: u32,
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
        
        // 编辑模式切换
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
        
        // 验证错误显示
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
        
        // 编辑区域
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
        
        // 验证配置
        if self.edit_mode == EditMode::Text {
            // 从文本解析配置
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
        
        // 验证配置值
        if self.config.surreal_port == 0 {
            self.validation_errors.push("SurrealDB 端口不能为 0".to_string());
        }
        
        if self.config.web_port == 0 {
            self.validation_errors.push("Web Server 端口不能为 0".to_string());
        }
        
        if !self.validation_errors.is_empty() {
            return;
        }
        
        // 保存到文件
        let toml_content = toml::to_string_pretty(&self.config).unwrap();
        match std::fs::write("DbOption.toml", toml_content) {
            Ok(_) => {
                // 显示成功提示
                println!("配置已保存");
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
```

## Data Models

### TaskRequest (任务请求)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_name: String,
    pub task_type: String,
    pub priority: String,
    pub description: String,
    pub site_ids: Vec<String>,
    pub parameters: serde_json::Value,
}
```

### TaskInfo (任务信息)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub progress: u8,
    pub current_step: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}
```

### TaskTemplate (任务模板)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub parameters: TaskParameters,
}
```

## Error Handling

### 错误类型定义

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigWizardError {
    #[error("任务创建失败: {0}")]
    TaskCreationError(String),
    
    #[error("数据库启动失败: {0}")]
    DatabaseStartError(String),
    
    #[error("配置文件错误: {0}")]
    ConfigFileError(String),
    
    #[error("验证失败: {0}")]
    ValidationError(String),
    
    #[error("API 调用失败: {0}")]
    ApiError(#[from] reqwest::Error),
}
```

## Testing Strategy

### 单元测试

- 任务参数验证逻辑测试
- 配置文件解析和序列化测试
- 任务模板保存和加载测试

### 集成测试

- 任务创建流程端到端测试
- 数据库启动和停止测试
- API 调用集成测试

### UI 测试

- 向导步骤导航测试
- 表单验证和错误提示测试
- 批量任务创建测试
