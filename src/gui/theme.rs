// Theme and styling configuration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub mode: ThemeMode,
    pub font_size: f32,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            mode: ThemeMode::Dark,
            font_size: 14.0,
        }
    }
    
    pub fn apply(&self, ctx: &egui::Context) {
        match self.mode {
            ThemeMode::Light => {
                ctx.set_visuals(egui::Visuals::light());
            }
            ThemeMode::Dark => {
                ctx.set_visuals(egui::Visuals::dark());
            }
            ThemeMode::Auto => {
                // Use system theme
                ctx.set_visuals(egui::Visuals::dark());
            }
        }
        
        // Apply font size
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(self.font_size),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(self.font_size),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::proportional(self.font_size * 1.5),
        );
        ctx.set_style(style);
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}
