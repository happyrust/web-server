// Toast notification manager

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: usize,
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

pub struct ToastManager {
    toasts: Vec<Toast>,
    next_id: usize,
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            next_id: 0,
        }
    }
    
    pub fn success(&mut self, message: impl Into<String>) {
        self.add_toast(message.into(), ToastType::Success);
    }
    
    pub fn error(&mut self, message: impl Into<String>) {
        self.add_toast(message.into(), ToastType::Error);
    }
    
    pub fn warning(&mut self, message: impl Into<String>) {
        self.add_toast(message.into(), ToastType::Warning);
    }
    
    pub fn info(&mut self, message: impl Into<String>) {
        self.add_toast(message.into(), ToastType::Info);
    }
    
    fn add_toast(&mut self, message: String, toast_type: ToastType) {
        let id = self.next_id;
        self.next_id += 1;
        self.toasts.push(Toast {
            id,
            message,
            toast_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        });
    }
    
    pub fn render(&mut self, ctx: &egui::Context) {
        let mut to_remove = Vec::new();
        
        egui::Area::new(egui::Id::new("toasts"))
            .fixed_pos(egui::pos2(ctx.screen_rect().width() - 320.0, 10.0))
            .show(ctx, |ui| {
                for toast in &self.toasts {
                    if toast.created_at.elapsed() > toast.duration {
                        to_remove.push(toast.id);
                        continue;
                    }
                    
                    let (bg_color, icon) = match toast.toast_type {
                        ToastType::Success => (egui::Color32::from_rgb(200, 255, 200), "✅"),
                        ToastType::Error => (egui::Color32::from_rgb(255, 200, 200), "❌"),
                        ToastType::Warning => (egui::Color32::from_rgb(255, 255, 200), "⚠️"),
                        ToastType::Info => (egui::Color32::from_rgb(200, 220, 255), "ℹ️"),
                    };
                    
                    egui::Frame::none()
                        .fill(bg_color)
                        .rounding(5.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(icon);
                                ui.label(&toast.message);
                            });
                        });
                    
                    ui.add_space(5.0);
                }
            });
        
        self.toasts.retain(|t| !to_remove.contains(&t.id));
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}
