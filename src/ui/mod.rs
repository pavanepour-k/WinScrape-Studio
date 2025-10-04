#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use std::sync::Arc;
#[cfg(feature = "ui")]
use tracing::{info, error, debug};

#[cfg(feature = "ui")]
pub mod chat;
#[cfg(feature = "ui")]
pub mod components;
#[cfg(feature = "ui")]
pub mod theme;
#[cfg(feature = "ui")]
pub mod state;
#[cfg(feature = "ui")]
pub mod windows_theme;
#[cfg(feature = "ui")]
pub mod windows_components;
#[cfg(feature = "ui")]
pub mod results_viewer;
#[cfg(feature = "ui")]
pub mod windows_ui;
#[cfg(feature = "ui")]
pub mod windows_launcher;
#[cfg(feature = "ui")]
pub mod windows_app;
#[cfg(feature = "ui")]
pub mod icon_manager;

#[cfg(feature = "ui")]
use crate::core::WinScrapeStudio;
#[cfg(feature = "ui")]
use crate::core::orchestrator::{WorkflowResult, WorkflowStage};

/// Main UI application
#[cfg(feature = "ui")]
pub struct WinScrapeUI {
    app: Arc<WinScrapeStudio>,
    state: state::UIState,
    chat: chat::ChatInterface,
    theme: theme::Theme,
}

#[cfg(feature = "ui")]
impl WinScrapeUI {
    /// Create new UI application
    pub fn new(app: Arc<WinScrapeStudio>) -> Self {
        let state = state::UIState::new();
        let chat = chat::ChatInterface::new();
        let theme = theme::Theme::dark();
        
        Self {
            app,
            state,
            chat,
            theme,
        }
    }
    
    /// Set theme
    pub fn set_theme(&mut self, theme: theme::Theme) {
        self.theme = theme;
    }
}

#[cfg(feature = "ui")]
impl eframe::App for WinScrapeUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        self.theme.apply(ctx);
        
        // Main UI layout
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_main_ui(ui, ctx);
        });
        
        // Handle background tasks
        self.handle_background_tasks(ctx);
    }
    
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Save UI state
        if let Ok(state_json) = serde_json::to_string(&self.state) {
            storage.set_string("ui_state", state_json);
        }
        
        // Save chat history
        if let Ok(chat_json) = serde_json::to_string(&self.chat) {
            storage.set_string("chat_history", chat_json);
        }
    }
    
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
}

#[cfg(feature = "ui")]
impl WinScrapeUI {
    /// Render main UI
    fn render_main_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Top bar
        self.render_top_bar(ui, ctx);
        
        ui.separator();
        
        // Main content area
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.state.current_view {
                    state::View::Chat => self.render_chat_view(ui, ctx),
                    state::View::Jobs => self.render_jobs_view(ui, ctx),
                    state::View::Settings => self.render_settings_view(ui, ctx),
                    state::View::Help => self.render_help_view(ui, ctx),
                }
            });
        
        // Bottom status bar
        self.render_status_bar(ui, ctx);
    }
    
    /// Render top navigation bar
    fn render_top_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.heading("🕷️ WinScrape Studio");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle
                if ui.button(if self.theme.is_dark() { "🌙" } else { "☀" }).clicked() {
                    self.theme = if self.theme.is_dark() {
                        theme::Theme::light()
                    } else {
                        theme::Theme::dark()
                    };
                }
                
                ui.separator();
                
                // Navigation buttons
                if ui.selectable_label(
                    matches!(self.state.current_view, state::View::Help),
                    "❓ Help"
                ).clicked() {
                    self.state.current_view = state::View::Help;
                }
                
                if ui.selectable_label(
                    matches!(self.state.current_view, state::View::Settings),
                    "⚙️ Settings"
                ).clicked() {
                    self.state.current_view = state::View::Settings;
                }
                
                if ui.selectable_label(
                    matches!(self.state.current_view, state::View::Jobs),
                    "📋 Jobs"
                ).clicked() {
                    self.state.current_view = state::View::Jobs;
                }
                
                if ui.selectable_label(
                    matches!(self.state.current_view, state::View::Chat),
                    "💬 Chat"
                ).clicked() {
                    self.state.current_view = state::View::Chat;
                }
            });
        });
    }
    
    /// Render chat view
    fn render_chat_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Natural Language Scraping");
        ui.label("Describe what you want to scrape in plain English:");
        
        // Chat interface
        self.chat.render(ui, ctx);
        
        // Handle chat input
        if let Some(user_input) = self.chat.get_pending_input() {
            self.handle_chat_input(user_input, ctx);
        }
    }
    
    /// Render jobs view
    fn render_jobs_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.heading("Scraping Jobs");
        
        // Jobs list
        if self.state.jobs.is_empty() {
            ui.label("No jobs yet. Start by describing what you want to scrape in the Chat tab.");
        } else {
        let jobs = self.state.jobs.clone();
        for job in &jobs {
            self.render_job_card(ui, job);
        }
        }
        
        // Refresh button
        if ui.button("🔄 Refresh Jobs").clicked() {
            self.refresh_jobs();
        }
    }
    
    /// Render settings view
    fn render_settings_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.heading("Settings");
        
        ui.group(|ui| {
            ui.label("Scraping Settings");
            
            ui.horizontal(|ui| {
                ui.label("Max concurrent requests:");
                ui.add(egui::Slider::new(&mut self.state.settings.max_concurrent_requests, 1..=20));
            });
            
            ui.horizontal(|ui| {
                ui.label("Request timeout (seconds):");
                ui.add(egui::Slider::new(&mut self.state.settings.request_timeout, 5..=120));
            });
            
            ui.checkbox(&mut self.state.settings.respect_robots_txt, "Respect robots.txt");
            ui.checkbox(&mut self.state.settings.enable_browser_fallback, "Enable browser fallback");
        });
        
        ui.group(|ui| {
            ui.label("Export Settings");
            
            egui::ComboBox::from_label("Default export format")
                .selected_text(&self.state.settings.default_export_format)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.state.settings.default_export_format, "csv".to_string(), "CSV");
                    ui.selectable_value(&mut self.state.settings.default_export_format, "json".to_string(), "JSON");
                    ui.selectable_value(&mut self.state.settings.default_export_format, "xlsx".to_string(), "XLSX");
                    ui.selectable_value(&mut self.state.settings.default_export_format, "parquet".to_string(), "Parquet");
                });
        });
        
        ui.group(|ui| {
            ui.label("Security Settings");
            
            ui.checkbox(&mut self.state.settings.enable_input_validation, "Enable input validation");
            ui.checkbox(&mut self.state.settings.enable_output_filtering, "Filter sensitive data from output");
        });
        
        if ui.button("💾 Save Settings").clicked() {
            self.save_settings();
        }
    }
    
    /// Render help view
    fn render_help_view(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.heading("Help & Documentation");
        
        ui.group(|ui| {
            ui.label("Getting Started");
            ui.label("1. Go to the Chat tab");
            ui.label("2. Describe what you want to scrape in plain English");
            ui.label("3. Review the generated scraping plan");
            ui.label("4. Approve and run the scraping job");
            ui.label("5. Export your results");
        });
        
        ui.group(|ui| {
            ui.label("Example Requests");
            ui.label("• \"Scrape product prices from shop.example.com\"");
            ui.label("• \"Get news headlines from news.example.com\"");
            ui.label("• \"Extract contact information from directory.example.com\"");
        });
        
        ui.group(|ui| {
            ui.label("Features");
            ui.label("✅ Natural language to scraping plan conversion");
            ui.label("✅ HTTP-first with browser fallback");
            ui.label("✅ Robots.txt compliance");
            ui.label("✅ Rate limiting and anti-blocking");
            ui.label("✅ Multiple export formats (CSV, JSON, XLSX, Parquet)");
            ui.label("✅ Data validation and filtering");
        });
        
        ui.group(|ui| {
            ui.label("About");
            ui.label(format!("WinScrape Studio v{}", env!("CARGO_PKG_VERSION")));
            ui.label("A natural language web scraping tool");
            ui.hyperlink_to("Documentation", "https://github.com/winscrape-studio/docs");
        });
    }
    
    /// Render job card
    fn render_job_card(&mut self, ui: &mut egui::Ui, job: &state::JobInfo) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(&job.title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(job.created_at.format("%Y-%m-%d %H:%M").to_string());
                    
                    // Status indicator
                    let (color, text) = match job.status {
                        state::JobStatus::Running => (egui::Color32::YELLOW, "🔄 Running"),
                        state::JobStatus::Completed => (egui::Color32::GREEN, "✅ Completed"),
                        state::JobStatus::Failed => (egui::Color32::RED, "❌ Failed"),
                        state::JobStatus::Queued => (egui::Color32::BLUE, "⏳ Queued"),
                        state::JobStatus::Cancelled => (egui::Color32::GRAY, "🚫 Cancelled"),
                    };
                    
                    ui.colored_label(color, text);
                });
            });
            
            ui.label(&job.description);
            
            ui.horizontal(|ui| {
                if ui.button("📊 View Results").clicked() {
                    self.view_job_results(&job.id);
                }
                
                if ui.button("📥 Export").clicked() {
                    self.export_job_results(&job.id);
                }
                
                if matches!(job.status, state::JobStatus::Running) {
                    if ui.button("⏹️ Cancel").clicked() {
                        self.cancel_job(&job.id);
                    }
                }
            });
        });
    }
    
    /// Render status bar
    fn render_status_bar(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!("Jobs: {}", self.state.jobs.len()));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(status) = &self.state.status_message {
                    ui.label(status);
                }
            });
        });
    }
    
    /// Handle chat input
    fn handle_chat_input(&mut self, input: String, ctx: &egui::Context) {
        debug!("Processing chat input: {}", input);
        
        // Add user message to chat
        self.chat.add_user_message(input.clone());
        
        // Start processing workflow
        self.state.current_workflow = Some(state::WorkflowState::Processing);
        self.state.status_message = Some("Processing your request...".to_string());
        
        // Spawn async task to handle the request
        let _app = self.app.clone();
        let _ctx = ctx.clone();
        
        tokio::spawn(async move {
            // This would need to be handled differently in a real implementation
            // For now, we'll simulate the workflow
            info!("Starting workflow for input: {}", input);
        });
    }
    
    /// Handle background tasks
    fn handle_background_tasks(&mut self, ctx: &egui::Context) {
        // Check for completed workflows
        if let Some(workflow_state) = &self.state.current_workflow {
            match workflow_state {
                state::WorkflowState::Processing => {
                    // Show processing indicator
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                state::WorkflowState::Completed(result) => {
                    self.handle_workflow_completion(result.clone());
                    self.state.current_workflow = None;
                }
                state::WorkflowState::Failed(error) => {
                    self.handle_workflow_error(error.clone());
                    self.state.current_workflow = None;
                }
            }
        }
        
        // Refresh jobs periodically
        if self.state.last_job_refresh.elapsed() > std::time::Duration::from_secs(30) {
            self.refresh_jobs();
        }
    }
    
    /// Handle workflow completion
    fn handle_workflow_completion(&mut self, result: WorkflowResult) {
        info!("Workflow completed: {:?}", result.stage);
        
        match result.stage {
            WorkflowStage::Approval => {
                // Show approval dialog
                self.chat.add_system_message("Please review the scraping plan and approve to continue.".to_string());
                if let Some(approval) = &result.pending_approval {
                    self.state.pending_approval = Some(approval.clone());
                }
            }
            WorkflowStage::Completed => {
                self.chat.add_system_message("Scraping completed successfully!".to_string());
                if let Some(job_id) = &result.job_id {
                    self.refresh_job_details(job_id);
                }
            }
            WorkflowStage::Failed => {
                let error_msg = result.errors.join("; ");
                self.chat.add_system_message(format!("Scraping failed: {}", error_msg));
            }
            _ => {
                self.chat.add_system_message(format!("Workflow stage: {}", result.stage));
            }
        }
        
        self.state.status_message = None;
    }
    
    /// Handle workflow error
    fn handle_workflow_error(&mut self, error: String) {
        error!("Workflow error: {}", error);
        self.chat.add_system_message(format!("Error: {}", error));
        self.state.status_message = None;
    }
    
    /// Refresh jobs list
    fn refresh_jobs(&mut self) {
        // This would fetch jobs from the app
        self.state.last_job_refresh = std::time::Instant::now();
        debug!("Refreshing jobs list");
    }
    
    /// Refresh specific job details
    fn refresh_job_details(&mut self, job_id: &str) {
        debug!("Refreshing job details: {}", job_id);
    }
    
    /// View job results
    fn view_job_results(&mut self, job_id: &str) {
        info!("Viewing results for job: {}", job_id);
        // This would open a results viewer
    }
    
    /// Export job results
    fn export_job_results(&mut self, job_id: &str) {
        info!("Exporting results for job: {}", job_id);
        // This would trigger export dialog
    }
    
    /// Cancel running job
    fn cancel_job(&mut self, job_id: &str) {
        info!("Cancelling job: {}", job_id);
        // This would cancel the job
    }
    
    /// Save settings
    fn save_settings(&mut self) {
        info!("Saving settings");
        // This would save settings to config
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct WinScrapeUI;

#[cfg(not(feature = "ui"))]
impl WinScrapeUI {
    pub fn new(_app: std::sync::Arc<crate::core::WinScrapeStudio>) -> Self {
        Self
    }
}
