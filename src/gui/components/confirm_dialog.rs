// Confirmation dialog component

pub struct ConfirmDialog {
    pub show: bool,
    pub title: String,
    pub message: String,
    pub confirmed: bool,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            show: false,
            title: String::new(),
            message: String::new(),
            confirmed: false,
        }
    }
    
    pub fn open(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.show = true;
        self.title = title.into();
        self.message = message.into();
        self.confirmed = false;
    }
    
    pub fn render(&mut self, ctx: &egui::Context) -> bool {
        if !self.show {
            return false;
        }
        
        let mut confirmed = false;
        
        egui::Window::new(&self.title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(&self.message);
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.button("确认").clicked() {
                        confirmed = true;
                        self.show = false;
                    }
                    if ui.button("取消").clicked() {
                        self.show = false;
                    }
                });
            });
        
        confirmed
    }
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self::new()
    }
}
