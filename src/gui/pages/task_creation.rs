// Task creation page

use crate::gui::{ApiClient, AppState};
use crate::gui::components::task_wizard::{TaskCreationWizard, TaskTemplate};

pub struct TaskCreationPage {
    wizard: TaskCreationWizard,
    templates: Vec<TaskTemplate>,
    show_template_dialog: bool,
}

impl TaskCreationPage {
    pub fn new() -> Self {
        Self {
            wizard: TaskCreationWizard::new(),
            templates: Self::load_templates(),
            show_template_dialog: false,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, state: &mut AppState, api_client: &ApiClient) {
        ui.heading("创建任务");
        
        ui.horizontal(|ui| {
            if ui.button("📋 从模板创建").clicked() {
                self.show_template_dialog = true;
            }
            
            if ui.button("💾 保存为模板").clicked() {
                self.save_as_template();
            }
        });
        
        ui.separator();
        
        self.wizard.render(ui, state, api_client);
        
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
                if self.templates.is_empty() {
                    ui.label("暂无保存的模板");
                } else {
                    for template in &self.templates {
                        ui.horizontal(|ui| {
                            if ui.button(&template.name).clicked() {
                                self.wizard.load_from_template(template);
                                self.show_template_dialog = false;
                            }
                            ui.label(&template.description);
                        });
                    }
                }
                
                ui.separator();
                if ui.button("取消").clicked() {
                    self.show_template_dialog = false;
                }
            });
    }
    
    fn save_as_template(&mut self) {
        let template = self.wizard.to_template();
        self.templates.push(template);
        self.save_templates_to_file();
    }
    
    fn save_templates_to_file(&self) {
        if let Some(config_dir) = dirs::config_dir() {
            let app_config_dir = config_dir.join("egui_remote_sync");
            if std::fs::create_dir_all(&app_config_dir).is_ok() {
                let template_file = app_config_dir.join("task_templates.json");
                if let Ok(json) = serde_json::to_string_pretty(&self.templates) {
                    let _ = std::fs::write(template_file, json);
                }
            }
        }
    }
    
    fn load_templates() -> Vec<TaskTemplate> {
        if let Some(config_dir) = dirs::config_dir() {
            let template_file = config_dir.join("egui_remote_sync/task_templates.json");
            if let Ok(content) = std::fs::read_to_string(template_file) {
                if let Ok(templates) = serde_json::from_str(&content) {
                    return templates;
                }
            }
        }
        Vec::new()
    }
}

impl Default for TaskCreationPage {
    fn default() -> Self {
        Self::new()
    }
}
