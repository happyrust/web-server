// Dashboard tab implementation

use super::app::RemoteSyncApp;
use super::types::*;
use eframe::egui;

#[derive(Default)]
pub struct DashboardState {
    // UI state for dashboard
}

pub fn render(ui: &mut egui::Ui, state: &mut DashboardState, app: &mut RemoteSyncApp) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("📊 仪表盘");
        ui.add_space(10.0);

        // Quick action buttons
        render_quick_actions(ui, app);
        ui.add_space(10.0);

        // Service status cards
        render_service_cards(ui, app);
        ui.add_space(10.0);

        // Sync statistics
        render_sync_stats(ui, app);
        ui.add_space(10.0);

        // Recent activity
        render_recent_activity(ui, app);
    });
}

fn render_quick_actions(ui: &mut egui::Ui, app: &mut RemoteSyncApp) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("快速操作").strong());
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            let services = app.runtime.block_on(async {
                app.services.read().await.running_count()
            });

            let all_running = services > 0;

            if ui.add_sized([150.0, 30.0], egui::Button::new("▶ 启动所有服务"))
                .on_hover_text("启动 MQTT、SurrealDB 和 Web 服务器")
                .clicked()
            {
                let services = app.services.clone();
                app.runtime.spawn(async move {
                    if let Err(e) = services.write().await.start_all().await {
                        eprintln!("Failed to start services: {}", e);
                    }
                });
            }

            if ui.add_sized([150.0, 30.0], egui::Button::new("⏸ 停止所有服务"))
                .on_hover_text("停止所有运行中的服务")
                .clicked()
            {
                let services = app.services.clone();
                app.runtime.spawn(async move {
                    if let Err(e) = services.write().await.stop_all().await {
                        eprintln!("Failed to stop services: {}", e);
                    }
                });
            }

            if ui.add_sized([150.0, 30.0], egui::Button::new("🔄 触发同步"))
                .on_hover_text("手动触发一次同步操作")
                .clicked()
            {
                // TODO: Implement manual sync trigger
            }

            if ui.add_sized([150.0, 30.0], egui::Button::new("🧪 运行测试"))
                .on_hover_text("启动自动化测试")
                .clicked()
            {
                // TODO: Implement test runner
            }
        });
    });
}

fn render_service_cards(ui: &mut egui::Ui, app: &mut RemoteSyncApp) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("服务状态").strong());
        ui.add_space(5.0);

        let services = app.runtime.block_on(async {
            app.services.read().await.get_all_services()
                .into_iter()
                .map(|s| s.clone())
                .collect::<Vec<_>>()
        });

        egui::Grid::new("service_grid")
            .num_columns(3)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                for service in services {
                    render_service_card(ui, &service);

                    if ui.small_button("重启").clicked() {
                        let services = app.services.clone();
                        let name = service.name.clone();
                        app.runtime.spawn(async move {
                            if let Err(e) = services.write().await.restart_service(&name).await {
                                eprintln!("Failed to restart {}: {}", name, e);
                            }
                        });
                    }
                }
            });
    });
}

fn render_service_card(ui: &mut egui::Ui, service: &ServiceInfo) {
    egui::Frame::none()
        .fill(egui::Color32::from_gray(40))
        .inner_margin(egui::Margin::same(10.0))
        .rounding(egui::Rounding::same(5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(service.status.color(), service.status.icon());
                ui.label(&service.display_name);
            });

            ui.label(format!("状态: {:?}", service.status));

            if let Some(port) = service.port {
                ui.label(format!("端口: {}", port));
            }

            if let Some(uptime) = service.uptime() {
                let minutes = uptime.as_secs() / 60;
                ui.label(format!("运行时间: {}分钟", minutes));
            }
        });

    ui.end_row();
}

fn render_sync_stats(ui: &mut egui::Ui, app: &RemoteSyncApp) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("同步统计").strong());
        ui.add_space(5.0);

        egui::Grid::new("sync_stats_grid")
            .num_columns(4)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("总同步次数:");
                ui.label(format!("{}", app.sync_stats.total_synced));

                ui.label("失败次数:");
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    format!("{}", app.sync_stats.total_failed)
                );
                ui.end_row();

                ui.label("同步速率:");
                ui.label(format!("{:.2} MB/s", app.sync_stats.sync_rate_mbps));

                ui.label("平均耗时:");
                ui.label(format!("{} ms", app.sync_stats.avg_sync_time_ms));
                ui.end_row();
            });

        ui.add_space(10.0);

        // Recent syncs table
        if !app.recent_syncs.is_empty() {
            ui.label(egui::RichText::new("最近同步记录").size(14.0));

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("recent_syncs_grid")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("时间");
                            ui.label("方向");
                            ui.label("文件");
                            ui.label("状态");
                            ui.label("耗时");
                            ui.end_row();

                            for sync in app.recent_syncs.iter().rev().take(10) {
                                ui.label(sync.timestamp.format("%H:%M:%S").to_string());
                                ui.label(format!("{} → {}", sync.source, sync.target));
                                ui.label(&sync.file_path);
                                ui.colored_label(sync.status.color(), sync.status.icon());
                                ui.label(format!("{}ms", sync.duration_ms));
                                ui.end_row();
                            }
                        });
                });
        }
    });
}

fn render_recent_activity(ui: &mut egui::Ui, app: &RemoteSyncApp) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("近期活动").strong());
        ui.add_space(5.0);

        if app.recent_events.is_empty() {
            ui.label("暂无活动记录");
        } else {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for event in app.recent_events.iter().rev().take(20) {
                        ui.horizontal(|ui| {
                            ui.label(event.icon());
                            ui.label(event.timestamp().format("%H:%M:%S").to_string());
                            ui.label(event.description());
                        });
                    }
                });
        }
    });
}
