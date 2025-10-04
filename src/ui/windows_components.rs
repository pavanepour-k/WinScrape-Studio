#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use std::collections::HashMap;

/// Windows-native UI components with Fluent Design
#[cfg(feature = "ui")]
pub struct WindowsComponents;

#[cfg(feature = "ui")]
impl WindowsComponents {
    /// Create a Windows-style navigation bar
    pub fn navigation_bar(ui: &mut egui::Ui, current_view: &str, views: &[(&str, &str, &str)]) -> Option<String> {
        let mut selected_view = None;
        
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            
            for (id, label, icon) in views {
                let is_selected = current_view == *id;
                let button_text = format!("{} {}", icon, label);
                
                let button = egui::Button::new(button_text)
                    .min_size(egui::Vec2::new(120.0, 40.0));
                
                if ui.add(button).clicked() {
                    selected_view = Some(id.to_string());
                }
                
                ui.add_space(8.0);
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle button
                if ui.button("🌙").clicked() {
                    // Theme toggle would be handled by parent
                }
                
                ui.add_space(8.0);
                
                // Settings button
                if ui.button("⚙️").clicked() {
                    selected_view = Some("settings".to_string());
                }
            });
        });
        
        selected_view
    }
    
    /// Create a Windows-style status bar
    pub fn status_bar(ui: &mut egui::Ui, status_items: &[(&str, egui::Color32)]) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            
            for (text, color) in status_items {
                ui.colored_label(*color, *text);
                ui.add_space(16.0);
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
            });
        });
    }
    
    /// Create a Windows-style card with header
    pub fn card_with_header(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::group(&egui::Style::default())
            .fill(ui.style().visuals.panel_fill)
            .stroke(egui::Stroke::new(1.0, ui.style().visuals.window_stroke.color))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.heading(title);
                ui.separator();
                content(ui);
            });
    }
    
    /// Create a Windows-style data table
    pub fn data_table(ui: &mut egui::Ui, headers: &[&str], rows: &[Vec<String>]) {
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("data_table")
                    .num_columns(headers.len())
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        // Header row
                        for header in headers {
                            ui.strong(*header);
                        }
                        ui.end_row();
                        
                        // Data rows
                        for row in rows {
                            for cell in row {
                                ui.label(cell);
                            }
                            ui.end_row();
                        }
                    });
            });
    }
    
    /// Create a Windows-style progress indicator
    pub fn progress_indicator(ui: &mut egui::Ui, progress: f32, text: &str) {
        ui.horizontal(|ui| {
            ui.label(text);
            ui.add_space(8.0);
            ui.add(egui::ProgressBar::new(progress).show_percentage());
        });
    }
    
    /// Create a Windows-style notification
    pub fn notification(ui: &mut egui::Ui, level: NotificationLevel, title: &str, message: &str) {
        let (color, icon) = match level {
            NotificationLevel::Info => (egui::Color32::BLUE, "ℹ️"),
            NotificationLevel::Success => (egui::Color32::GREEN, "✅"),
            NotificationLevel::Warning => (egui::Color32::YELLOW, "⚠️"),
            NotificationLevel::Error => (egui::Color32::RED, "❌"),
        };
        
        egui::Frame::group(&egui::Style::default())
            .fill(color.linear_multiply(0.1))
            .stroke(egui::Stroke::new(1.0, color))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(icon);
                    ui.vertical(|ui| {
                        ui.strong(title);
                        ui.label(message);
                    });
                });
            });
    }
    
    /// Create a Windows-style input field with label
    pub fn labeled_input(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) -> egui::Response {
        ui.vertical(|ui| {
            ui.label(label);
            ui.add(egui::TextEdit::singleline(value).hint_text(hint))
        }).inner
    }
    
    /// Create a Windows-style number input
    pub fn number_input(ui: &mut egui::Ui, label: &str, value: &mut f64, min: f64, max: f64) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).clamp_range(min..=max))
        }).inner
    }
    
    /// Create a Windows-style slider
    pub fn slider(ui: &mut egui::Ui, label: &str, value: &mut f32, min: f32, max: f32) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::Slider::new(value, min..=max))
        }).inner
    }
    
    /// Create a Windows-style checkbox
    pub fn checkbox(ui: &mut egui::Ui, label: &str, checked: &mut bool) -> egui::Response {
        ui.checkbox(checked, label)
    }
    
    /// Create a Windows-style dropdown
    pub fn dropdown(ui: &mut egui::Ui, label: &str, selected: &mut String, options: &[String]) -> egui::Response {
        let response = ui.horizontal(|ui| {
            ui.label(label);
            egui::ComboBox::from_id_source(label)
                .selected_text(selected.as_str())
                .show_ui(ui, |ui| {
                    for option in options {
                        ui.selectable_value(selected, option.clone(), option);
                    }
                });
        });
        response.response
    }
    
    /// Create a Windows-style button with icon
    pub fn icon_button(ui: &mut egui::Ui, icon: &str, text: &str) -> egui::Response {
        ui.button(format!("{} {}", icon, text))
    }
    
    /// Create a Windows-style action button
    pub fn action_button(ui: &mut egui::Ui, text: &str, primary: bool) -> egui::Response {
        let button = egui::Button::new(text).min_size(egui::Vec2::new(100.0, 32.0));
        ui.add(button)
    }
    
    /// Create a Windows-style file picker button
    pub fn file_picker_button(ui: &mut egui::Ui, label: &str, current_path: &str) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::TextEdit::singleline(&mut current_path.to_string()).hint_text("Select file..."));
            ui.button("📁")
        }).inner
    }
    
    /// Create a Windows-style folder picker button
    pub fn folder_picker_button(ui: &mut egui::Ui, label: &str, current_path: &str) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::TextEdit::singleline(&mut current_path.to_string()).hint_text("Select folder..."));
            ui.button("📂")
        }).inner
    }
    
    /// Create a Windows-style loading spinner
    pub fn loading_spinner(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.add_space(8.0);
            ui.label(text);
        });
    }
    
    /// Create a Windows-style confirmation dialog
    pub fn confirmation_dialog(ui: &mut egui::Ui, title: &str, message: &str) -> Option<bool> {
        let mut result = None;
        
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(message);
                ui.add_space(16.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        result = Some(true);
                    }
                    if ui.button("No").clicked() {
                        result = Some(false);
                    }
                });
            });
        
        result
    }
    
    /// Create a Windows-style tooltip
    pub fn tooltip(ui: &mut egui::Ui, text: &str) {
        if ui.rect_contains_pointer(ui.available_rect_before_wrap()) {
            egui::show_tooltip(ui.ctx(), egui::Id::new("tooltip"), |ui| {
                ui.label(text);
            });
        }
    }
    
    /// Create a Windows-style context menu
    pub fn context_menu(ui: &mut egui::Ui, items: &[(&str, &str)]) -> Option<String> {
        let mut selected = None;
        
        if ui.button("⋮").clicked() {
            // This would show a context menu
            // For now, we'll just return the first item
            if let Some((_, id)) = items.first() {
                selected = Some(id.to_string());
            }
        }
        
        selected
    }
    
    /// Create a Windows-style search box
    pub fn search_box(ui: &mut egui::Ui, query: &mut String, placeholder: &str) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(query).hint_text(placeholder))
        }).inner
    }
    
    /// Create a Windows-style tab bar
    pub fn tab_bar(ui: &mut egui::Ui, tabs: &[(&str, &str)], active_tab: &str) -> Option<String> {
        let mut selected = None;
        
        ui.horizontal(|ui| {
            for (id, label) in tabs {
                let is_active = active_tab == *id;
                let button = egui::Button::new(*label)
                    .fill(if is_active { ui.style().visuals.selection.bg_fill } else { egui::Color32::TRANSPARENT });
                
                if ui.add(button).clicked() {
                    selected = Some(id.to_string());
                }
            }
        });
        
        selected
    }
    
    /// Create a Windows-style splitter
    pub fn splitter(ui: &mut egui::Ui, ratio: &mut f32) -> egui::Response {
        ui.add(egui::Slider::new(ratio, 0.1..=0.9).show_value(false))
    }
    
    /// Create a Windows-style property grid
    pub fn property_grid(ui: &mut egui::Ui, properties: &[(&str, &str)]) {
        egui::Grid::new("property_grid")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for (name, value) in properties {
                    ui.label(*name);
                    ui.label(*value);
                    ui.end_row();
                }
            });
    }
}

/// Notification levels for Windows-style notifications
#[cfg(feature = "ui")]
#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct WindowsComponents;

#[cfg(not(feature = "ui"))]
impl WindowsComponents {
    pub fn navigation_bar(_ui: &mut eframe::egui::Ui, _current_view: &str, _views: &[(&str, &str, &str)]) -> Option<String> { None }
    pub fn status_bar(_ui: &mut eframe::egui::Ui, _status_items: &[(&str, eframe::egui::Color32)]) {}
    pub fn card_with_header(_ui: &mut eframe::egui::Ui, _title: &str, _content: impl FnOnce(&mut eframe::egui::Ui)) {}
    pub fn data_table(_ui: &mut eframe::egui::Ui, _headers: &[&str], _rows: &[Vec<String>]) {}
    pub fn progress_indicator(_ui: &mut eframe::egui::Ui, _progress: f32, _text: &str) {}
    pub fn notification(_ui: &mut eframe::egui::Ui, _level: NotificationLevel, _title: &str, _message: &str) {}
    pub fn labeled_input(_ui: &mut eframe::egui::Ui, _label: &str, _value: &mut String, _hint: &str) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn number_input(_ui: &mut eframe::egui::Ui, _label: &str, _value: &mut f64, _min: f64, _max: f64) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn slider(_ui: &mut eframe::egui::Ui, _label: &str, _value: &mut f32, _min: f32, _max: f32) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn checkbox(_ui: &mut eframe::egui::Ui, _label: &str, _checked: &mut bool) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn dropdown(_ui: &mut eframe::egui::Ui, _label: &str, _selected: &mut String, _options: &[String]) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn icon_button(_ui: &mut eframe::egui::Ui, _icon: &str, _text: &str) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn action_button(_ui: &mut eframe::egui::Ui, _text: &str, _primary: bool) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn file_picker_button(_ui: &mut eframe::egui::Ui, _label: &str, _current_path: &str) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn folder_picker_button(_ui: &mut eframe::egui::Ui, _label: &str, _current_path: &str) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn loading_spinner(_ui: &mut eframe::egui::Ui, _text: &str) {}
    pub fn confirmation_dialog(_ui: &mut eframe::egui::Ui, _title: &str, _message: &str) -> Option<bool> { None }
    pub fn tooltip(_ui: &mut eframe::egui::Ui, _text: &str) {}
    pub fn context_menu(_ui: &mut eframe::egui::Ui, _items: &[(&str, &str)]) -> Option<String> { None }
    pub fn search_box(_ui: &mut eframe::egui::Ui, _query: &mut String, _placeholder: &str) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn tab_bar(_ui: &mut eframe::egui::Ui, _tabs: &[(&str, &str)], _active_tab: &str) -> Option<String> { None }
    pub fn splitter(_ui: &mut eframe::egui::Ui, _ratio: &mut f32) -> eframe::egui::Response { eframe::egui::Response::default() }
    pub fn property_grid(_ui: &mut eframe::egui::Ui, _properties: &[(&str, &str)]) {}
}

#[cfg(not(feature = "ui"))]
#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}
