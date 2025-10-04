#[cfg(feature = "ui")]
use eframe::egui;

/// Reusable UI components
#[cfg(feature = "ui")]
pub struct Components;

#[cfg(feature = "ui")]
impl Components {
    /// Render a loading spinner
    pub fn loading_spinner(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(text);
        });
    }
    
    /// Render a status badge
    pub fn status_badge(ui: &mut egui::Ui, status: &str, color: egui::Color32) {
        ui.colored_label(color, status);
    }
    
    /// Render a collapsible section
    pub fn collapsible_section<R>(
        ui: &mut egui::Ui,
        title: &str,
        default_open: bool,
        mut content: impl FnMut(&mut egui::Ui) -> R,
    ) -> R {
        egui::CollapsingHeader::new(title)
            .default_open(default_open)
            .show(ui, |ui| content(ui))
            .body_returned
            .unwrap_or_else(|| content(ui))
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct Components;
