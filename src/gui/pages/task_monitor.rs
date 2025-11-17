// Task monitor page

use crate::gui::ApiClient;
use serde::{Deserialize, Serialize};

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

pub struct TaskMonitorPage {
    tasks: Vec<TaskInfo>,
    selected_task: Option<String>,
    show_task_detail: bool,
    auto_refresh: bool,
    last_refresh: std::time::Instant,
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
        
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(150.0))
            .column(Column::auto().at_least(100.0))
            .column(Column::auto().at_least(80.0))
            .column(Column::auto().at_least(150.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong("任务名称"); });
                header.col(|ui| { ui.strong("任务类型"); });
                header.col(|ui| { ui.strong("状态"); });
                header.col(|ui| { ui.strong("进度"); });
                header.col(|ui| { ui.strong("操作"); });
            })
            .body(|mut body| {
                let tasks = self.tasks.clone();
                for task in &tasks {
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
                
                if tasks.is_empty() {
                    body.row(30.0, |mut row| {
                        row.col(|ui| { ui.label("暂无任务"); });
                    });
                }
            });
        
        if self.show_task_detail {
            self.render_task_detail(ui.ctx());
        }
        
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
    
    fn refresh_tasks(&mut self, _api_client: &ApiClient) {
        // TODO: Call API to get tasks
        self.last_refresh = std::time::Instant::now();
    }
    
    fn cancel_task(&self, _task_id: &str, _api_client: &ApiClient) {
        // TODO: Call API to cancel task
    }
    
    fn delete_task(&mut self, task_id: &str, _api_client: &ApiClient) {
        // TODO: Call API to delete task
        self.tasks.retain(|t| t.id != task_id);
    }
}

impl Default for TaskMonitorPage {
    fn default() -> Self {
        Self::new()
    }
}
