// Main application structure

use crate::gui::{ApiClient, AppState};
use crate::gui::components::ToastManager;
use crate::gui::pages::{EnvironmentListPage, MonitorDashboardPage, WebServerPage};
use crate::gui::theme::Theme;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Page {
    EnvironmentList,
    MonitorDashboard,
    WebServer,
    Settings,
}

pub struct EguiRemoteSyncApp {
    state: AppState,
    api_client: ApiClient,
    current_page: Page,
    toast_manager: ToastManager,
    theme: Theme,
    
    // Pages
    environment_list_page: EnvironmentListPage,
    monitor_dashboard_page: MonitorDashboardPage,
    web_server_page: WebServerPage,
}

impl EguiRemoteSyncApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            state: AppState::new(),
            api_client: ApiClient::new("http://localhost:3000"),
            current_page: Page::EnvironmentList,
            toast_manager: ToastManager::new(),
            theme: Theme::new(),
            environment_list_page: EnvironmentListPage::new(),
            monitor_dashboard_page: MonitorDashboardPage::new(),
            web_server_page: WebServerPage::new(),
        };
        
        // Load saved state
        if let Some(storage) = cc.storage {
            if let Some(page) = eframe::get_value(storage, "current_page") {
                app.current_page = page;
            }
            if let Some(theme) = eframe::get_value(storage, "theme") {
                app.theme = theme;
            }
        }
        
        // Apply theme
        app.theme.apply(&cc.egui_ctx);
        
        app
    }
    
    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("文件", |ui| {
                if ui.button("退出").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            
            ui.menu_button("视图", |ui| {
                if ui.button("环境列表").clicked() {
                    self.current_page = Page::EnvironmentList;
                }
                if ui.button("监控面板").clicked() {
                    self.current_page = Page::MonitorDashboard;
                }
                if ui.button("Web Server").clicked() {
                    self.current_page = Page::WebServer;
                }
            });
            
            ui.menu_button("帮助", |ui| {
                if ui.button("关于").clicked() {
                    self.toast_manager.info("egui 异地协同运维界面 v0.1.0");
                }
            });
        });
    }
    
    fn render_navigation(&mut self, ui: &mut egui::Ui) {
        ui.heading("导航");
        ui.separator();
        
        ui.vertical(|ui| {
            ui.label("异地协同");
            if ui.selectable_label(self.current_page == Page::EnvironmentList, "  环境列表").clicked() {
                self.current_page = Page::EnvironmentList;
            }
            if ui.selectable_label(self.current_page == Page::MonitorDashboard, "  监控面板").clicked() {
                self.current_page = Page::MonitorDashboard;
            }
            
            ui.add_space(10.0);
            ui.label("系统管理");
            if ui.selectable_label(self.current_page == Page::WebServer, "  Web Server").clicked() {
                self.current_page = Page::WebServer;
            }
            if ui.selectable_label(self.current_page == Page::Settings, "  设置").clicked() {
                self.current_page = Page::Settings;
            }
        });
    }
    
    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("设置");
        ui.separator();
        
        ui.label("主题");
        ui.horizontal(|ui| {
            if ui.selectable_label(matches!(self.theme.mode, crate::gui::theme::ThemeMode::Light), "浅色").clicked() {
                self.theme.mode = crate::gui::theme::ThemeMode::Light;
                self.theme.apply(ui.ctx());
            }
            if ui.selectable_label(matches!(self.theme.mode, crate::gui::theme::ThemeMode::Dark), "深色").clicked() {
                self.theme.mode = crate::gui::theme::ThemeMode::Dark;
                self.theme.apply(ui.ctx());
            }
        });
        
        ui.add_space(10.0);
        ui.label("字体大小");
        ui.add(egui::Slider::new(&mut self.theme.font_size, 10.0..=20.0));
        if ui.button("应用").clicked() {
            self.theme.apply(ui.ctx());
        }
        
        ui.add_space(10.0);
        ui.label("API 地址");
        let mut base_url = self.api_client.clone();
        // Note: ApiClient doesn't expose base_url, so we'll just show a placeholder
        ui.label("http://localhost:3000");
    }
}

impl eframe::App for EguiRemoteSyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu_bar(ui);
        });
        
        // Left navigation panel
        egui::SidePanel::left("navigation")
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_navigation(ui);
            });
        
        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_page {
                Page::EnvironmentList => {
                    self.environment_list_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::MonitorDashboard => {
                    self.monitor_dashboard_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::WebServer => {
                    self.web_server_page.render(ui);
                }
                Page::Settings => {
                    self.render_settings(ui);
                }
            }
        });
        
        // Toast notifications
        self.toast_manager.render(ctx);
        
        // Auto-refresh for monitor dashboard
        if self.current_page == Page::MonitorDashboard {
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        }
    }
    
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "current_page", &self.current_page);
        eframe::set_value(storage, "theme", &self.theme);
    }
}
