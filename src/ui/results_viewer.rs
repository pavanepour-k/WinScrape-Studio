#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use std::collections::HashMap;

/// Results viewer for displaying scraped data
#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsViewer {
    pub job_id: String,
    pub data: Vec<HashMap<String, serde_json::Value>>,
    pub current_page: usize,
    pub page_size: usize,
    pub sort_column: Option<String>,
    pub sort_ascending: bool,
    pub filter_text: String,
    pub selected_rows: std::collections::HashSet<usize>,
    pub view_mode: ViewMode,
    pub export_format: ExportFormat,
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Table,
    Cards,
    JSON,
    Statistics,
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    CSV,
    JSON,
    XLSX,
    Parquet,
}

#[cfg(feature = "ui")]
impl ResultsViewer {
    pub fn new(job_id: String, data: Vec<HashMap<String, serde_json::Value>>) -> Self {
        Self {
            job_id,
            data,
            current_page: 0,
            page_size: 50,
            sort_column: None,
            sort_ascending: true,
            filter_text: String::new(),
            selected_rows: std::collections::HashSet::new(),
            view_mode: ViewMode::Table,
            export_format: ExportFormat::CSV,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Header with controls
        self.render_header(ui);
        
        ui.separator();
        
        // Main content area
        match self.view_mode {
            ViewMode::Table => self.render_table_view(ui),
            ViewMode::Cards => self.render_cards_view(ui),
            ViewMode::JSON => self.render_json_view(ui),
            ViewMode::Statistics => self.render_statistics_view(ui),
        }
        
        // Footer with pagination
        self.render_footer(ui);
    }
    
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // View mode selector
            egui::ComboBox::from_label("View")
                .selected_text(format!("{:?}", self.view_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view_mode, ViewMode::Table, "Table");
                    ui.selectable_value(&mut self.view_mode, ViewMode::Cards, "Cards");
                    ui.selectable_value(&mut self.view_mode, ViewMode::JSON, "JSON");
                    ui.selectable_value(&mut self.view_mode, ViewMode::Statistics, "Statistics");
                });
            
            ui.separator();
            
            // Search/filter
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Filter data..."));
            
            ui.separator();
            
            // Page size selector
            ui.label("Page size:");
            egui::ComboBox::from_id_source("page_size")
                .selected_text(self.page_size.to_string())
                .show_ui(ui, |ui| {
                    for size in [25, 50, 100, 200] {
                        ui.selectable_value(&mut self.page_size, size, size.to_string());
                    }
                });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Export button
                if ui.button("📥 Export").clicked() {
                    self.export_data();
                }
                
                ui.separator();
                
                // Export format selector
                egui::ComboBox::from_label("Format")
                    .selected_text(format!("{:?}", self.export_format))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.export_format, ExportFormat::CSV, "CSV");
                        ui.selectable_value(&mut self.export_format, ExportFormat::JSON, "JSON");
                        ui.selectable_value(&mut self.export_format, ExportFormat::XLSX, "XLSX");
                        ui.selectable_value(&mut self.export_format, ExportFormat::Parquet, "Parquet");
                    });
            });
        });
    }
    
    fn render_table_view(&mut self, ui: &mut egui::Ui) {
        let filtered_data = self.get_filtered_data();
        let sorted_data = self.get_sorted_data(&filtered_data);
        let paginated_data = self.get_paginated_data(&sorted_data);
        
        if paginated_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No data to display");
            });
            return;
        }
        
        // Get column headers
        let headers = self.get_column_headers(&paginated_data);
        
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("results_table")
                    .num_columns(headers.len() + 1) // +1 for selection checkbox
                    .spacing([4.0, 2.0])
                    .show(ui, |ui| {
                        // Header row
                        ui.checkbox(&mut false, ""); // Select all checkbox
                        for header in &headers {
                            let header_text = if self.sort_column.as_ref() == Some(header) {
                                format!("{} {}", header, if self.sort_ascending { "↑" } else { "↓" })
                            } else {
                                header.clone()
                            };
                            
                            if ui.button(header_text).clicked() {
                                self.sort_by_column(header);
                            }
                        }
                        ui.end_row();
                        
                        // Data rows
                        for (row_idx, row) in paginated_data.iter().enumerate() {
                            let global_idx = self.current_page * self.page_size + row_idx;
                            let mut is_selected = self.selected_rows.contains(&global_idx);
                            
                            ui.checkbox(&mut is_selected, "");
                            if is_selected {
                                self.selected_rows.insert(global_idx);
                            } else {
                                self.selected_rows.remove(&global_idx);
                            }
                            
                            for header in &headers {
                                let value = row.get(header)
                                    .map(|v| format_value(v))
                                    .unwrap_or_else(|| "".to_string());
                                ui.label(value);
                            }
                            ui.end_row();
                        }
                    });
            });
    }
    
    fn render_cards_view(&mut self, ui: &mut egui::Ui) {
        let filtered_data = self.get_filtered_data();
        let sorted_data = self.get_sorted_data(&filtered_data);
        let paginated_data = self.get_paginated_data(&sorted_data);
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (row_idx, row) in paginated_data.iter().enumerate() {
                    let global_idx = self.current_page * self.page_size + row_idx;
                    let is_selected = self.selected_rows.contains(&global_idx);
                    
                    egui::Frame::group(&egui::Style::default())
                        .fill(if is_selected { 
                            ui.style().visuals.selection.bg_fill 
                        } else { 
                            ui.style().visuals.panel_fill 
                        })
                        .stroke(egui::Stroke::new(1.0, ui.style().visuals.window_stroke.color))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mut is_selected_copy = self.selected_rows.contains(&global_idx);
                                ui.checkbox(&mut is_selected_copy, "");
                                if is_selected_copy {
                                    self.selected_rows.insert(global_idx);
                                } else {
                                    self.selected_rows.remove(&global_idx);
                                }
                                
                                ui.vertical(|ui| {
                                    for (key, value) in row {
                                        ui.horizontal(|ui| {
                                            ui.strong(format!("{}: ", key));
                                            ui.label(format_value(value));
                                        });
                                    }
                                });
                            });
                        });
                    
                    ui.add_space(8.0);
                }
            });
    }
    
    fn render_json_view(&mut self, ui: &mut egui::Ui) {
        let filtered_data = self.get_filtered_data();
        
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Ok(json_str) = serde_json::to_string_pretty(&filtered_data) {
                    ui.add(egui::TextEdit::multiline(&mut json_str.clone())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(20));
                } else {
                    ui.label("Failed to serialize data to JSON");
                }
            });
    }
    
    fn render_statistics_view(&mut self, ui: &mut egui::Ui) {
        let filtered_data = self.get_filtered_data();
        
        ui.vertical(|ui| {
            ui.heading("Data Statistics");
            
            // Basic stats
            ui.group(|ui| {
                ui.label(format!("Total records: {}", filtered_data.len()));
                ui.label(format!("Selected records: {}", self.selected_rows.len()));
                ui.label(format!("Current page: {}", self.current_page + 1));
                ui.label(format!("Total pages: {}", (filtered_data.len() + self.page_size - 1) / self.page_size));
            });
            
            // Column statistics
            if !filtered_data.is_empty() {
                ui.add_space(16.0);
                ui.heading("Column Statistics");
                
                let headers = self.get_column_headers(&filtered_data);
                
                for header in headers {
                    ui.group(|ui| {
                        ui.strong(&header);
                        
                        let values: Vec<&serde_json::Value> = filtered_data.iter()
                            .filter_map(|row| row.get(&header))
                            .collect();
                        
                        if !values.is_empty() {
                            ui.label(format!("Count: {}", values.len()));
                            
                            // Type analysis
                            let types: std::collections::HashMap<String, usize> = values.iter()
                                .map(|v| match v {
                                    serde_json::Value::String(_) => "String",
                                    serde_json::Value::Number(_) => "Number",
                                    serde_json::Value::Bool(_) => "Boolean",
                                    serde_json::Value::Null => "Null",
                                    _ => "Other",
                                })
                                .fold(std::collections::HashMap::new(), |mut acc, t| {
                                    *acc.entry(t.to_string()).or_insert(0) += 1;
                                    acc
                                });
                            
                            for (type_name, count) in types {
                                ui.label(format!("  {}: {}", type_name, count));
                            }
                            
                            // Sample values
                            ui.label("Sample values:");
                            for (i, value) in values.iter().take(3).enumerate() {
                                ui.label(format!("  {}: {}", i + 1, format_value(value)));
                            }
                        }
                    });
                }
            }
        });
    }
    
    fn render_footer(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            let total_pages = (self.get_filtered_data().len() + self.page_size - 1) / self.page_size;
            
            ui.label(format!("Page {} of {}", self.current_page + 1, total_pages));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Next").clicked() && self.current_page < total_pages - 1 {
                    self.current_page += 1;
                }
                
                if ui.button("Previous").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                }
            });
        });
    }
    
    fn get_filtered_data(&self) -> Vec<HashMap<String, serde_json::Value>> {
        if self.filter_text.is_empty() {
            return self.data.clone();
        }
        
        self.data.iter()
            .filter(|row| {
                row.values().any(|value| {
                    format_value(value).to_lowercase().contains(&self.filter_text.to_lowercase())
                })
            })
            .cloned()
            .collect()
    }
    
    fn get_sorted_data(&self, data: &[HashMap<String, serde_json::Value>]) -> Vec<HashMap<String, serde_json::Value>> {
        if let Some(ref column) = self.sort_column {
            let mut sorted = data.to_vec();
            sorted.sort_by(|a, b| {
                let a_val = a.get(column).map(format_value).unwrap_or_default();
                let b_val = b.get(column).map(format_value).unwrap_or_default();
                
                if self.sort_ascending {
                    a_val.cmp(&b_val)
                } else {
                    b_val.cmp(&a_val)
                }
            });
            sorted
        } else {
            data.to_vec()
        }
    }
    
    fn get_paginated_data(&self, data: &[HashMap<String, serde_json::Value>]) -> Vec<HashMap<String, serde_json::Value>> {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(data.len());
        
        if start >= data.len() {
            Vec::new()
        } else {
            data[start..end].to_vec()
        }
    }
    
    fn get_column_headers(&self, data: &[HashMap<String, serde_json::Value>]) -> Vec<String> {
        if data.is_empty() {
            return Vec::new();
        }
        
        let mut headers: std::collections::HashSet<String> = std::collections::HashSet::new();
        for row in data {
            headers.extend(row.keys().cloned());
        }
        
        let mut headers: Vec<String> = headers.into_iter().collect();
        headers.sort();
        headers
    }
    
    fn sort_by_column(&mut self, column: &str) {
        if self.sort_column.as_ref() == Some(&column.to_string()) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(column.to_string());
            self.sort_ascending = true;
        }
    }
    
    fn export_data(&self) {
        // This would trigger the export functionality
        println!("Exporting data in {:?} format", self.export_format);
    }
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "".to_string(),
        serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
        serde_json::Value::Object(obj) => format!("{{{} fields}}", obj.len()),
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct ResultsViewer;

#[cfg(not(feature = "ui"))]
impl ResultsViewer {
    pub fn new(_job_id: String, _data: Vec<HashMap<String, serde_json::Value>>) -> Self { Self }
    pub fn render(&mut self, _ui: &mut eframe::egui::Ui) {}
}

#[cfg(not(feature = "ui"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    Table,
    Cards,
    JSON,
    Statistics,
}

#[cfg(not(feature = "ui"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    CSV,
    JSON,
    XLSX,
    Parquet,
}
