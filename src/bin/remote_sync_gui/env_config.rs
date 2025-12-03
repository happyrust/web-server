// Environment configuration tab
use super::app::RemoteSyncApp;
use super::types::*;
use eframe::egui;

#[derive(Default)]
pub struct EnvConfigState {}

pub fn render(ui: &mut egui::Ui, state: &mut EnvConfigState, app: &mut RemoteSyncApp) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("⚙️ 环境配置");
        ui.add_space(10.0);

        // Environment basic info
        ui.group(|ui| {
            ui.label(egui::RichText::new("环境基本信息").strong());
            ui.add_space(5.0);

            egui::Grid::new("env_grid").show(ui, |ui| {
                ui.label("环境名称:");
                ui.text_edit_singleline(&mut app.environment.name);
                ui.end_row();

                ui.label("站点标识:");
                ui.text_edit_singleline(&mut app.environment.location);
                ui.end_row();
            });
        });

        ui.add_space(10.0);

        // MQTT configuration
        ui.group(|ui| {
            ui.label(egui::RichText::new("MQTT 配置").strong());
            ui.add_space(5.0);

            egui::Grid::new("mqtt_grid").show(ui, |ui| {
                ui.label("服务器:");
                ui.text_edit_singleline(&mut app.environment.mqtt_host);
                ui.end_row();

                ui.label("端口:");
                let mut port = app.environment.mqtt_port as i32;
                ui.add(egui::DragValue::new(&mut port).clamp_range(1..=65535));
                app.environment.mqtt_port = port as u16;
                ui.end_row();
            });

            if ui.button("测试连接").clicked() {
                // TODO: Test MQTT connection
            }
        });

        ui.add_space(10.0);

        // Save button
        if ui.button("💾 保存配置").clicked() {
            let db = app.db.clone();
            let env = app.environment.clone();
            app.runtime.spawn(async move {
                if let Err(e) = db.lock().await.save_environment(&env).await {
                    eprintln!("Failed to save environment: {}", e);
                }
            });
        }
    });
}
