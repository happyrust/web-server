// Task creation wizard component

use crate::gui::{ApiClient, AppState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum WizardStep {
    BasicInfo,
    SelectSite,
    Parameters,
    Preview,
}

#[derive(Debug, Clone)]
pub struct TaskCreationFormData {
    pub task_name: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub description: String,
    pub parameters: TaskParameters,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    DataParsing,
    ModelGeneration,
    SpatialTreeGeneration,
    FullSync,
    IncrementalSync,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParseMode {
    All,
    SpecificDbNums,
    SpecificRefno,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpatialIndexType {
    RTree,
    QuadTree,
    OctTree,
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

pub struct TaskCreationWizard {
    current_step: WizardStep,
    form_data: TaskCreationFormData,
    validation_errors: HashMap<String, String>,
    selected_site: Option<String>,
    batch_mode: bool,
    selected_sites: Vec<String>,
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
        self.render_step_indicator(ui);
        ui.separator();
        
        match self.current_step {
            WizardStep::BasicInfo => self.render_basic_info(ui),
            WizardStep::SelectSite => self.render_site_selection(ui, state),
            WizardStep::Parameters => self.render_parameters(ui),
            WizardStep::Preview => self.render_preview(ui, state),
        }
        
        ui.separator();
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
                ui.label("任务名称 *:");
                ui.text_edit_singleline(&mut self.form_data.task_name);
                ui.end_row();
                
                if let Some(error) = self.validation_errors.get("task_name") {
                    ui.label("");
                    ui.colored_label(egui::Color32::RED, error);
                    ui.end_row();
                }
                
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
                
                ui.label("任务描述:");
                ui.text_edit_multiline(&mut self.form_data.description);
                ui.end_row();
                
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
        
        ui.checkbox(&mut self.batch_mode, "批量模式（选择多个站点）");
        ui.separator();
        
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(40.0))
            .column(Column::auto().at_least(150.0))
            .column(Column::auto().at_least(100.0))
            .column(Column::auto().at_least(100.0))
            .column(Column::auto().at_least(80.0))
            .column(Column::remainder())
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
                                // TODO: Test connection
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
        
        ui.strong("资源需求预估:");
        ui.label("预计内存使用: ~2GB");
        ui.label("预计磁盘空间: ~500MB");
        ui.label("预计耗时: 10-30 分钟");
        
        ui.separator();
        
        ui.strong("⚠️ 注意事项:");
        ui.label("• 大型数据库解析可能需要较长时间");
        ui.label("• 请确保目标站点有足够的磁盘空间");
        ui.label("• 任务执行期间请勿关闭应用");
    }
    
    fn render_navigation_buttons(&mut self, ui: &mut egui::Ui, _api_client: &ApiClient) {
        ui.horizontal(|ui| {
            if self.current_step != WizardStep::BasicInfo {
                if ui.button("⬅️ 上一步").clicked() {
                    self.go_to_previous_step();
                }
            }
            
            if self.current_step == WizardStep::Preview {
                if ui.button("✅ 创建任务").clicked() {
                    self.create_task();
                }
            } else {
                let can_proceed = self.validate_current_step();
                if ui.add_enabled(can_proceed, egui::Button::new("下一步 ➡️")).clicked() {
                    self.go_to_next_step();
                }
            }
            
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
    
    fn create_task(&self) {
        // TODO: Implement task creation with API call
        log::info!("Creating task: {}", self.form_data.task_name);
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

impl Default for TaskCreationWizard {
    fn default() -> Self {
        Self::new()
    }
}
