// Main application structure

use crate::gui::{ApiClient, AppState};
use crate::gui::components::ToastManager;
use crate::gui::pages::{EnvironmentListPage, MonitorDashboardPage, WebServerPage, TopologyCanvasPage, LogQueryPage, SiteConfigPage, TaskCreationPage, TaskMonitorPage, DatabaseManagePage, ConfigEditorPage};
use crate::gui::theme::Theme;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Page {
    EnvironmentList,
    SiteConfig,
    TopologyCanvas,
    MonitorDashboard,
    LogQuery,
    WebServer,
    TaskCreation,
    TaskMonitor,
    DatabaseManage,
    ConfigEditor,
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
    site_config_page: SiteConfigPage,
    topology_canvas_page: TopologyCanvasPage,
    monitor_dashboard_page: MonitorDashboardPage,
    log_query_page: LogQueryPage,
    web_server_page: WebServerPage,
    task_creation_page: TaskCreationPage,
    task_monitor_page: TaskMonitorPage,
    database_manage_page: DatabaseManagePage,
    config_editor_page: ConfigEditorPage,
}

impl EguiRemoteSyncApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Setup Chinese font
        Self::setup_chinese_fonts(&cc.egui_ctx);
        
        let mut app = Self {
            state: AppState::new(),
            api_client: ApiClient::new("http://localhost:3000"),
            current_page: Page::EnvironmentList,
            toast_manager: ToastManager::new(),
            theme: Theme::new(),
            environment_list_page: EnvironmentListPage::new(),
            site_config_page: SiteConfigPage::new(),
            topology_canvas_page: TopologyCanvasPage::new(),
            monitor_dashboard_page: MonitorDashboardPage::new(),
            log_query_page: LogQueryPage::new(),
            web_server_page: WebServerPage::new(),
            task_creation_page: TaskCreationPage::new(),
            task_monitor_page: TaskMonitorPage::new(),
            database_manage_page: DatabaseManagePage::new(),
            config_editor_page: ConfigEditorPage::new(),
        };
        
        // Load saved state
        if let Some(storage) = cc.storage {
            if let Some(page) = storage.get_string("current_page") {
                if let Ok(page) = serde_json::from_str(&page) {
                    app.current_page = page;
                }
            }
            if let Some(theme) = storage.get_string("theme") {
                if let Ok(theme) = serde_json::from_str(&theme) {
                    app.theme = theme;
                }
            }
        }
        
        // Apply theme
        app.theme.apply(&cc.egui_ctx);
        
        app
    }
    
    fn setup_chinese_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        
        // Try to load embedded Chinese font
        // If the font file doesn't exist, we'll use system fonts as fallback
        let font_loaded = Self::try_load_embedded_font(&mut fonts);
        
        if !font_loaded {
            // Fallback: Try to load from system fonts
            Self::try_load_system_font(&mut fonts);
        }
        
        ctx.set_fonts(fonts);
    }
    
    fn try_load_embedded_font(fonts: &mut egui::FontDefinitions) -> bool {
        // Try to load embedded font (if available)
        // This requires the font file to be placed in assets/fonts/
        #[cfg(feature = "embed_fonts")]
        {
            fonts.font_data.insert(
                "noto_sans_sc".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../../assets/fonts/NotoSansSC-Regular.ttf"
                ))),
            );
            
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "noto_sans_sc".to_owned());
            
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("noto_sans_sc".to_owned());
            
            return true;
        }
        
        #[cfg(not(feature = "embed_fonts"))]
        {
            // Try to load from file system
            if let Ok(font_data) = std::fs::read("assets/fonts/NotoSansSC-Regular.ttf") {
                fonts.font_data.insert(
                    "noto_sans_sc".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                );
                
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "noto_sans_sc".to_owned());
                
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("noto_sans_sc".to_owned());
                
                return true;
            }
        }
        
        false
    }
    
    fn try_load_system_font(fonts: &mut egui::FontDefinitions) {
        // On different platforms, try to load system Chinese fonts
        let system_font_paths = if cfg!(target_os = "windows") {
            vec![
                "C:\\Windows\\Fonts\\msyh.ttc",      // Microsoft YaHei
                "C:\\Windows\\Fonts\\simhei.ttf",    // SimHei
                "C:\\Windows\\Fonts\\simsun.ttc",    // SimSun
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                "/System/Library/Fonts/PingFang.ttc",           // PingFang SC
                "/System/Library/Fonts/STHeiti Light.ttc",      // STHeiti
                "/Library/Fonts/Arial Unicode.ttf",             // Arial Unicode MS
            ]
        } else {
            // Linux
            vec![
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            ]
        };
        
        for font_path in system_font_paths {
            if let Ok(font_data) = std::fs::read(font_path) {
                fonts.font_data.insert(
                    "system_chinese".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                );
                
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "system_chinese".to_owned());
                
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("system_chinese".to_owned());
                
                log::info!("Loaded system Chinese font from: {}", font_path);
                break;
            }
        }
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
                if ui.button("站点配置").clicked() {
                    self.current_page = Page::SiteConfig;
                }
                if ui.button("拓扑配置").clicked() {
                    self.current_page = Page::TopologyCanvas;
                }
                if ui.button("监控面板").clicked() {
                    self.current_page = Page::MonitorDashboard;
                }
                if ui.button("日志查询").clicked() {
                    self.current_page = Page::LogQuery;
                }
                if ui.button("Web Server").clicked() {
                    self.current_page = Page::WebServer;
                }
                ui.separator();
                if ui.button("任务创建").clicked() {
                    self.current_page = Page::TaskCreation;
                }
                if ui.button("任务监控").clicked() {
                    self.current_page = Page::TaskMonitor;
                }
                if ui.button("数据库管理").clicked() {
                    self.current_page = Page::DatabaseManage;
                }
                if ui.button("配置编辑").clicked() {
                    self.current_page = Page::ConfigEditor;
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
            if ui.selectable_label(self.current_page == Page::SiteConfig, "  站点配置").clicked() {
                self.current_page = Page::SiteConfig;
            }
            if ui.selectable_label(self.current_page == Page::TopologyCanvas, "  拓扑配置").clicked() {
                self.current_page = Page::TopologyCanvas;
            }
            if ui.selectable_label(self.current_page == Page::MonitorDashboard, "  监控面板").clicked() {
                self.current_page = Page::MonitorDashboard;
            }
            if ui.selectable_label(self.current_page == Page::LogQuery, "  日志查询").clicked() {
                self.current_page = Page::LogQuery;
            }
            
            ui.add_space(10.0);
            ui.label("任务管理");
            if ui.selectable_label(self.current_page == Page::TaskCreation, "  任务创建").clicked() {
                self.current_page = Page::TaskCreation;
            }
            if ui.selectable_label(self.current_page == Page::TaskMonitor, "  任务监控").clicked() {
                self.current_page = Page::TaskMonitor;
            }
            
            ui.add_space(10.0);
            ui.label("系统管理");
            if ui.selectable_label(self.current_page == Page::WebServer, "  Web Server").clicked() {
                self.current_page = Page::WebServer;
            }
            if ui.selectable_label(self.current_page == Page::DatabaseManage, "  数据库管理").clicked() {
                self.current_page = Page::DatabaseManage;
            }
            if ui.selectable_label(self.current_page == Page::ConfigEditor, "  配置编辑").clicked() {
                self.current_page = Page::ConfigEditor;
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
                Page::SiteConfig => {
                    self.site_config_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::TopologyCanvas => {
                    self.topology_canvas_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::MonitorDashboard => {
                    self.monitor_dashboard_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::LogQuery => {
                    self.log_query_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::WebServer => {
                    self.web_server_page.render(ui);
                }
                Page::TaskCreation => {
                    self.task_creation_page.render(ui, &mut self.state, &self.api_client);
                }
                Page::TaskMonitor => {
                    self.task_monitor_page.render(ui, &self.api_client);
                }
                Page::DatabaseManage => {
                    self.database_manage_page.render(ui);
                }
                Page::ConfigEditor => {
                    self.config_editor_page.render(ui);
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
        if let Ok(page_json) = serde_json::to_string(&self.current_page) {
            storage.set_string("current_page", page_json);
        }
        if let Ok(theme_json) = serde_json::to_string(&self.theme) {
            storage.set_string("theme", theme_json);
        }
    }
}
