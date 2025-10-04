#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use std::collections::HashMap;

/// Windows-native theme system with Fluent Design elements
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct WindowsTheme {
    pub is_dark: bool,
    pub accent_color: egui::Color32,
    pub background_color: egui::Color32,
    pub surface_color: egui::Color32,
    pub text_color: egui::Color32,
    pub secondary_text_color: egui::Color32,
    pub border_color: egui::Color32,
    pub hover_color: egui::Color32,
    pub active_color: egui::Color32,
    pub error_color: egui::Color32,
    pub warning_color: egui::Color32,
    pub success_color: egui::Color32,
    pub info_color: egui::Color32,
    pub corner_radius: f32,
    pub shadow_size: f32,
    pub animation_duration: f32,
}

#[cfg(feature = "ui")]
impl WindowsTheme {
    /// Create Windows 11 dark theme
    pub fn windows11_dark() -> Self {
        Self {
            is_dark: true,
            accent_color: egui::Color32::from_rgb(0, 120, 212), // Windows 11 blue
            background_color: egui::Color32::from_rgb(32, 32, 32), // Dark gray
            surface_color: egui::Color32::from_rgb(43, 43, 43), // Slightly lighter
            text_color: egui::Color32::from_rgb(255, 255, 255), // White
            secondary_text_color: egui::Color32::from_rgb(200, 200, 200), // Light gray
            border_color: egui::Color32::from_rgb(60, 60, 60), // Medium gray
            hover_color: egui::Color32::from_rgb(70, 70, 70), // Hover gray
            active_color: egui::Color32::from_rgb(0, 120, 212), // Accent blue
            error_color: egui::Color32::from_rgb(232, 17, 35), // Red
            warning_color: egui::Color32::from_rgb(255, 185, 0), // Yellow
            success_color: egui::Color32::from_rgb(16, 124, 16), // Green
            info_color: egui::Color32::from_rgb(0, 120, 212), // Blue
            corner_radius: 8.0,
            shadow_size: 4.0,
            animation_duration: 0.2,
        }
    }
    
    /// Create Windows 11 light theme
    pub fn windows11_light() -> Self {
        Self {
            is_dark: false,
            accent_color: egui::Color32::from_rgb(0, 120, 212), // Windows 11 blue
            background_color: egui::Color32::from_rgb(243, 243, 243), // Light gray
            surface_color: egui::Color32::from_rgb(255, 255, 255), // White
            text_color: egui::Color32::from_rgb(32, 32, 32), // Dark gray
            secondary_text_color: egui::Color32::from_rgb(96, 96, 96), // Medium gray
            border_color: egui::Color32::from_rgb(200, 200, 200), // Light gray
            hover_color: egui::Color32::from_rgb(240, 240, 240), // Very light gray
            active_color: egui::Color32::from_rgb(0, 120, 212), // Accent blue
            error_color: egui::Color32::from_rgb(232, 17, 35), // Red
            warning_color: egui::Color32::from_rgb(255, 185, 0), // Yellow
            success_color: egui::Color32::from_rgb(16, 124, 16), // Green
            info_color: egui::Color32::from_rgb(0, 120, 212), // Blue
            corner_radius: 8.0,
            shadow_size: 4.0,
            animation_duration: 0.2,
        }
    }
    
    /// Apply Windows theme to egui context
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        
        // Apply Windows 11 Fluent Design colors
        visuals.window_fill = self.surface_color;
        visuals.panel_fill = self.background_color;
        visuals.faint_bg_color = self.hover_color;
        visuals.extreme_bg_color = self.border_color;
        
        // Button styling
        visuals.widgets.inactive.bg_fill = self.surface_color;
        visuals.widgets.hovered.bg_fill = self.hover_color;
        visuals.widgets.active.bg_fill = self.active_color;
        visuals.widgets.open.bg_fill = self.active_color;
        
        // Text colors
        visuals.override_text_color = Some(self.text_color);
        // Note: weak_text_color is not directly settable in current egui API
        
        // Selection colors
        visuals.selection.bg_fill = self.accent_color;
        visuals.selection.stroke.color = self.accent_color;
        
        // Hyperlink colors
        visuals.hyperlink_color = self.accent_color;
        
        // Window styling
        visuals.window_rounding = egui::Rounding::same(self.corner_radius);
        visuals.window_shadow = egui::epaint::Shadow {
            color: egui::Color32::from_black_alpha(30),
            extrusion: self.shadow_size,
        };
        
        // Panel styling
        visuals.panel_fill = self.background_color;
        
        // Button styling
        visuals.widgets.noninteractive.bg_fill = self.surface_color;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, self.border_color);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(self.corner_radius);
        
        // Interactive widget styling
        visuals.widgets.inactive.rounding = egui::Rounding::same(self.corner_radius);
        visuals.widgets.hovered.rounding = egui::Rounding::same(self.corner_radius);
        visuals.widgets.active.rounding = egui::Rounding::same(self.corner_radius);
        
        // Slider styling
        visuals.widgets.inactive.weak_bg_fill = self.border_color;
        visuals.widgets.hovered.weak_bg_fill = self.hover_color;
        visuals.widgets.active.weak_bg_fill = self.active_color;
        
        ctx.set_visuals(visuals);
        
        // Set animation settings
        let mut style = (*ctx.style()).clone();
        style.animation_time = self.animation_duration;
        ctx.set_style(style);
    }
    
    /// Get status color for different states
    pub fn get_status_color(&self, status: &str) -> egui::Color32 {
        match status.to_lowercase().as_str() {
            "success" | "completed" | "ok" => self.success_color,
            "error" | "failed" | "critical" => self.error_color,
            "warning" | "warn" => self.warning_color,
            "info" | "information" => self.info_color,
            "running" | "processing" => self.accent_color,
            _ => self.secondary_text_color,
        }
    }
    
    /// Create a Windows-style button
    pub fn create_button_style(&self) -> egui::Button {
        egui::Button::new("")
    }
    
    /// Get Windows-style spacing
    pub fn get_spacing(&self) -> egui::style::Spacing {
        egui::style::Spacing {
            item_spacing: egui::Vec2::new(8.0, 8.0),
            window_margin: egui::Margin::same(16.0),
            button_padding: egui::Vec2::new(12.0, 6.0),
            indent: 20.0,
            interact_size: egui::Vec2::new(32.0, 32.0),
            slider_width: 20.0,
            text_edit_width: 280.0,
            icon_width: 16.0,
            icon_width_inner: 8.0,
            icon_spacing: 4.0,
            tooltip_width: 600.0,
            indent_ends_with_horizontal_line: false,
            combo_width: 100.0,
            menu_margin: egui::Margin::same(8.0),
            combo_height: 20.0,
            scroll: egui::style::ScrollStyle::default(),
        }
    }
    
    /// Create Windows-style frame
    pub fn create_frame(&self, title: &str) -> egui::Frame {
        egui::Frame::group(&egui::Style::default())
            .fill(self.surface_color)
            .stroke(egui::Stroke::new(1.0, self.border_color))
            .rounding(egui::Rounding::same(self.corner_radius))
            .shadow(egui::epaint::Shadow {
                color: egui::Color32::from_black_alpha(10),
                extrusion: 2.0,
            })
    }
    
    /// Create Windows-style card
    pub fn create_card(&self) -> egui::Frame {
        egui::Frame::group(&egui::Style::default())
            .fill(self.surface_color)
            .stroke(egui::Stroke::new(1.0, self.border_color))
            .rounding(egui::Rounding::same(self.corner_radius))
            .shadow(egui::epaint::Shadow {
                color: egui::Color32::from_black_alpha(15),
                extrusion: 4.0,
            })
            .inner_margin(egui::Margin::same(16.0))
    }
    
    /// Get Windows-style input field width (placeholder - not used)
    pub fn get_input_width(&self) -> f32 {
        // This method is not used in the current implementation
        f32::INFINITY
    }
    
    /// Create Windows-style progress bar
    pub fn create_progress_bar(&self, progress: f32) -> egui::ProgressBar {
        egui::ProgressBar::new(progress)
            .fill(self.accent_color)
            .show_percentage()
    }
    
    /// Create Windows-style tooltip (placeholder - not used)
    pub fn get_tooltip_width(&self) -> f32 {
        // This would be used in the UI rendering
        600.0
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct WindowsTheme;

#[cfg(not(feature = "ui"))]
impl WindowsTheme {
    pub fn windows11_dark() -> Self { Self }
    pub fn windows11_light() -> Self { Self }
    pub fn apply(&self, _ctx: &eframe::egui::Context) {}
    pub fn get_status_color(&self, _status: &str) -> eframe::egui::Color32 { eframe::egui::Color32::GRAY }
    pub fn create_button_style(&self) -> eframe::egui::Button { eframe::egui::Button::new("") }
    pub fn get_spacing(&self) -> eframe::egui::style::Spacing { eframe::egui::style::Spacing::default() }
    pub fn create_frame(&self, _title: &str) -> eframe::egui::Frame { eframe::egui::Frame::group(&eframe::egui::Style::default()) }
    pub fn create_card(&self) -> eframe::egui::Frame { eframe::egui::Frame::group(&eframe::egui::Style::default()) }
    pub fn get_input_width(&self) -> f32 { f32::INFINITY }
    pub fn create_progress_bar(&self, _progress: f32) -> eframe::egui::ProgressBar { eframe::egui::ProgressBar::new(0.0) }
    pub fn get_tooltip_width(&self) -> f32 { 600.0 }
}
