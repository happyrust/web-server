// Main application state and UI

use super::types::*;
use super::{dashboard, env_config, file_browser, log_viewer, remote_sites, sync_control, test_tools};
use super::db::Database;
use super::service_manager::ServiceManager;
use eframe::egui;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use std::collections::VecDeque;

/// Main application state
pub struct RemoteSyncApp {
    // UI state
    selected_tab: Tab,

    // Configuration
    environment: EnvironmentConfig,
    remote_sites: Vec<RemoteSite>,

    // Runtime state
    services: Arc<RwLock<ServiceManager>>,
    sync_stats: SyncStats,
    recent_syncs: VecDeque<SyncRecord>,
    recent_events: VecDeque<SystemEvent>,
    log_entries: VecDeque<LogEntry>,

    // Database
    db: Arc<Mutex<Database>>,

    // Tab states
    dashboard_state: dashboard::DashboardState,
    env_config_state: env_config::EnvConfigState,
    remote_sites_state: remote_sites::RemoteSitesState,
    sync_control_state: sync_control::SyncControlState,
    file_browser_state: file_browser::FileBrowserState,
    log_viewer_state: log_viewer::LogViewerState,
    test_tools_state: test_tools::TestToolsState,

    // Runtime
    runtime: Arc<tokio::runtime::Runtime>,
}

impl RemoteSyncApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure fonts and style
        Self::configure_fonts(&cc.egui_ctx);
        Self::configure_style(&cc.egui_ctx);

        // Create runtime
        let runtime = Arc::new(
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
        );

        // Initialize database
        let db = Arc::new(Mutex::new(
            runtime.block_on(Database::new("remote-test-dir/test-real"))
                .expect("Failed to initialize database")
        ));

        // Load configuration
        let (environment, remote_sites) = runtime.block_on(async {
            let db_guard = db.lock().await;
            let env = db_guard.load_environment().await
                .unwrap_or_default();
            let sites = db_guard.load_remote_sites().await
                .unwrap_or_default();
            (env, sites)
        });

        // Create service manager
        let services = Arc::new(RwLock::new(ServiceManager::new()));

        Self {
            selected_tab: Tab::Dashboard,
            environment,
            remote_sites,
            services,
            sync_stats: SyncStats::default(),
            recent_syncs: VecDeque::with_capacity(100),
            recent_events: VecDeque::with_capacity(100),
            log_entries: VecDeque::with_capacity(1000),
            db,
            dashboard_state: dashboard::DashboardState::default(),
            env_config_state: env_config::EnvConfigState::default(),
            remote_sites_state: remote_sites::RemoteSitesState::default(),
            sync_control_state: sync_control::SyncControlState::default(),
            file_browser_state: file_browser::FileBrowserState::default(),
            log_viewer_state: log_viewer::LogViewerState::default(),
            test_tools_state: test_tools::TestToolsState::default(),
            runtime,
        }
    }

    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // Add Chinese font support
        // Note: You'll need to add actual font files
        // For now, using default fonts which should support Chinese on Windows

        ctx.set_fonts(fonts);
    }

    fn configure_style(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        // Customize spacing and sizing
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);

        ctx.set_style(style);
    }

    fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("异地协同管理工具");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Service status indicators
                    let services_running = self.runtime.block_on(async {
                        self.services.read().await.running_count()
                    });

                    if services_running > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 200, 0),
                            format!("🟢 {} 个服务运行中", services_running)
                        );
                    } else {
                        ui.colored_label(egui::Color32::GRAY, "⚫ 服务已停止");
                    }

                    ui.separator();

                    // Last sync time
                    if let Some(last_sync) = self.sync_stats.last_sync_time {
                        let elapsed = chrono::Utc::now() - last_sync;
                        let minutes = elapsed.num_minutes();
                        ui.label(format!("最后同步: {}分钟前", minutes));
                    } else {
                        ui.label("最后同步: 无");
                    }
                });
            });
        });
    }

    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("环境: {}", self.environment.location));
                ui.separator();
                ui.label(format!("站点: {} 个", self.remote_sites.len()));
                ui.separator();
                ui.label(format!("同步: {} 成功 / {} 失败",
                    self.sync_stats.total_synced,
                    self.sync_stats.total_failed
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("日志: {} 条", self.log_entries.len()));
                });
            });
        });
    }

    fn render_tab_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for tab in Tab::all() {
                    if ui.selectable_label(self.selected_tab == tab, tab.label()).clicked() {
                        self.selected_tab = tab;
                    }
                }
            });
        });
    }

    fn render_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => {
                    dashboard::render(ui, &mut self.dashboard_state, self);
                }
                Tab::EnvConfig => {
                    env_config::render(ui, &mut self.env_config_state, self);
                }
                Tab::RemoteSites => {
                    remote_sites::render(ui, &mut self.remote_sites_state, self);
                }
                Tab::SyncControl => {
                    sync_control::render(ui, &mut self.sync_control_state, self);
                }
                Tab::FileBrowser => {
                    file_browser::render(ui, &mut self.file_browser_state, self);
                }
                Tab::LogViewer => {
                    log_viewer::render(ui, &mut self.log_viewer_state, self);
                }
                Tab::TestTools => {
                    test_tools::render(ui, &mut self.test_tools_state, self);
                }
            }
        });
    }

    pub fn add_log(&mut self, entry: LogEntry) {
        self.log_entries.push_back(entry);
        if self.log_entries.len() > 1000 {
            self.log_entries.pop_front();
        }
    }

    pub fn add_event(&mut self, event: SystemEvent) {
        self.recent_events.push_back(event);
        if self.recent_events.len() > 100 {
            self.recent_events.pop_front();
        }
    }

    pub fn add_sync_record(&mut self, record: SyncRecord) {
        if record.status == SyncStatus::Completed {
            self.sync_stats.total_synced += 1;
        } else if record.status == SyncStatus::Failed {
            self.sync_stats.total_failed += 1;
        }

        self.sync_stats.last_sync_time = Some(record.timestamp);

        self.recent_syncs.push_back(record);
        if self.recent_syncs.len() > 100 {
            self.recent_syncs.pop_front();
        }
    }
}

impl eframe::App for RemoteSyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaint for real-time updates
        ctx.request_repaint();

        self.render_top_panel(ctx);
        self.render_bottom_panel(ctx);
        self.render_tab_bar(ctx);
        self.render_content(ctx);
    }
}
