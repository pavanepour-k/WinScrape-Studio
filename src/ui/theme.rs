#[cfg(feature = "ui")]
use eframe::egui;

/// UI theme configuration
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct Theme {
    pub is_dark: bool,
    pub primary_color: egui::Color32,
    pub secondary_color: egui::Color32,
    pub background_color: egui::Color32,
    pub text_color: egui::Color32,
}

#[cfg(feature = "ui")]
impl Theme {
    /// Create dark theme
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            primary_color: egui::Color32::from_rgb(100, 149, 237),
            secondary_color: egui::Color32::from_rgb(70, 130, 180),
            background_color: egui::Color32::from_rgb(32, 32, 32),
            text_color: egui::Color32::WHITE,
        }
    }
    
    /// Create light theme
    pub fn light() -> Self {
        Self {
            is_dark: false,
            primary_color: egui::Color32::from_rgb(70, 130, 180),
            secondary_color: egui::Color32::from_rgb(100, 149, 237),
            background_color: egui::Color32::WHITE,
            text_color: egui::Color32::BLACK,
        }
    }
    
    /// Apply theme to context
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        
        // Customize colors
        visuals.widgets.noninteractive.bg_fill = self.background_color;
        visuals.widgets.inactive.bg_fill = self.secondary_color;
        visuals.widgets.hovered.bg_fill = self.primary_color;
        visuals.widgets.active.bg_fill = self.primary_color;
        
        visuals.override_text_color = Some(self.text_color);
        
        ctx.set_visuals(visuals);
    }
    
    /// Check if theme is dark
    pub fn is_dark(&self) -> bool {
        self.is_dark
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct Theme;

#[cfg(not(feature = "ui"))]
impl Theme {
    pub fn dark() -> Self { Self }
    pub fn light() -> Self { Self }
    pub fn is_dark(&self) -> bool { true }
}
