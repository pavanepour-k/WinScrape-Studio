#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use std::sync::Arc;
#[cfg(feature = "ui")]
use tracing::{info, error, debug, warn};
#[cfg(feature = "ui")]
use std::collections::HashMap;

#[cfg(feature = "ui")]
use crate::core::WinScrapeStudio;
#[cfg(feature = "ui")]
use crate::core::orchestrator::{WorkflowResult, WorkflowStage};
#[cfg(feature = "ui")]
use super::{
    chat::ChatInterface,
    state::{UIState, View, JobInfo, JobStatus, WorkflowState},
    windows_theme::WindowsTheme,
    windows_components::{WindowsComponents, NotificationLevel},
    results_viewer::ResultsViewer,
    icon_manager::IconManager,
};
use crate::i18n::{I18nManager, Language};

/// Main Windows-native UI application
#[cfg(feature = "ui")]
/// Outcomes of spawned background actions (cancel/export/approve/view
/// results) delivered back to the UI thread via `ui_event_rx`.
enum UiEvent {
    JobResultsLoaded {
        job_id: String,
        results: Vec<serde_json::Value>,
    },
    Notify {
        level: NotificationLevel,
        title: String,
        message: String,
    },
}

pub struct WindowsUI {
    app: Arc<WinScrapeStudio>,
    state: UIState,
    chat: ChatInterface,
    theme: WindowsTheme,
    icon_manager: IconManager,
    i18n_manager: I18nManager,
    results_viewer: Option<ResultsViewer>,
    notifications: Vec<Notification>,
    show_about: bool,
    show_export_dialog: bool,
    show_language_dialog: bool,
    show_icon_dialog: bool,
    export_path: String,
    /// Selected format in the export dialog dropdown. Previously this was
    /// a local variable re-created every frame, so the dropdown always
    /// reset to "CSV" and the user's selection was silently discarded.
    export_format: String,
    /// Which job the export dialog is currently exporting. Previously the
    /// dialog had no notion of a target job at all.
    export_job_id: Option<String>,
    window_title: String,
    /// Sender handed to spawned workflow tasks; results are delivered back
    /// to the UI thread via `workflow_rx` and picked up in
    /// `handle_background_tasks` on the next frame.
    workflow_tx: std::sync::mpsc::Sender<WorkflowState>,
    workflow_rx: std::sync::mpsc::Receiver<WorkflowState>,
    jobs_tx: std::sync::mpsc::Sender<Vec<JobInfo>>,
    jobs_rx: std::sync::mpsc::Receiver<Vec<JobInfo>>,
    ui_event_tx: std::sync::mpsc::Sender<UiEvent>,
    ui_event_rx: std::sync::mpsc::Receiver<UiEvent>,
    /// Local working copy of the app's real configuration, loaded once at
    /// startup. The Settings screen edits this directly and "Save
    /// Settings" persists it to disk - previously the Settings screen was
    /// backed by a separate `UISettings` struct that was never connected
    /// to the app's actual config at all, so nothing there did anything.
    /// Note: some fields (concurrency, robots.txt, browser fallback) are
    /// read once when the scraping engine is built at startup, so changes
    /// take effect on next app restart rather than instantly.
    config_draft: crate::config::AppConfig,
    /// Text-edit buffer for the blocked-domains list (one per line);
    /// synced to/from config_draft.security.blocked_domains on load/save.
    blocked_domains_text: String,
    /// Text-edit buffer for the allowed URL schemes list (one per line);
    /// synced to/from config_draft.security.allowed_schemes on load/save.
    allowed_schemes_text: String,
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
struct Notification {
    id: String,
    level: NotificationLevel,
    title: String,
    message: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    auto_close: bool,
}

/// Convert a storage-layer Job into the UI's JobInfo, used wherever a
/// freshly-fetched job list needs to be reflected in the UI state.
#[cfg(feature = "ui")]
fn storage_job_to_info(job: crate::storage::Job) -> JobInfo {
    let status = match job.status {
        crate::storage::JobStatus::Queued => JobStatus::Queued,
        crate::storage::JobStatus::Running => JobStatus::Running,
        crate::storage::JobStatus::Completed => JobStatus::Completed,
        crate::storage::JobStatus::Failed => JobStatus::Failed,
        crate::storage::JobStatus::Cancelled => JobStatus::Cancelled,
    };
    JobInfo {
        id: job.id,
        title: job.title,
        description: job.user_prompt,
        status,
        created_at: job.created_at,
        completed_at: None,
        result_count: None,
    }
}

#[cfg(feature = "ui")]
impl WindowsUI {
    /// Create new Windows UI application
    pub fn new(app: Arc<WinScrapeStudio>) -> Self {
        let state = UIState::new();
        let chat = ChatInterface::new();
        let theme = WindowsTheme::windows11_dark();
        let icon_manager = IconManager::new();
        let i18n_manager = I18nManager::new();
        let (workflow_tx, workflow_rx) = std::sync::mpsc::channel();
        let (jobs_tx, jobs_rx) = std::sync::mpsc::channel();
        let (ui_event_tx, ui_event_rx) = std::sync::mpsc::channel();
        let config_draft = app.get_config();
        let blocked_domains_text = config_draft.security.blocked_domains.join("\n");
        let allowed_schemes_text = config_draft.security.allowed_schemes.join("\n");
        
        let mut ui = Self {
            app,
            state,
            chat,
            theme,
            icon_manager,
            i18n_manager,
            results_viewer: None,
            notifications: Vec::new(),
            show_about: false,
            show_export_dialog: false,
            show_language_dialog: false,
            show_icon_dialog: false,
            export_path: String::new(),
            export_format: "CSV".to_string(),
            export_job_id: None,
            window_title: format!("WinScrape Studio v{}", env!("CARGO_PKG_VERSION")),
            workflow_tx,
            workflow_rx,
            jobs_tx,
            jobs_rx,
            ui_event_tx,
            ui_event_rx,
            config_draft,
            blocked_domains_text,
            allowed_schemes_text,
        };
        ui.refresh_jobs();
        ui
    }
    
    /// Set theme
    pub fn set_theme(&mut self, is_dark: bool) {
        self.theme = if is_dark {
            WindowsTheme::windows11_dark()
        } else {
            WindowsTheme::windows11_light()
        };
    }
    
    /// Add notification
    pub fn add_notification(&mut self, level: NotificationLevel, title: String, message: String) {
        let notification = Notification {
            id: uuid::Uuid::new_v4().to_string(),
            level,
            title,
            message,
            timestamp: chrono::Utc::now(),
            auto_close: true,
        };
        self.notifications.push(notification);
    }
    
    /// Remove notification
    pub fn remove_notification(&mut self, id: &str) {
        self.notifications.retain(|n| n.id != id);
    }
    
    /// Set language
    pub fn set_language(&mut self, language: Language) {
        self.i18n_manager.set_language(language);
        info!("Language changed to: {}", language.name());
    }
    
    /// Get current language
    pub fn current_language(&self) -> Language {
        self.i18n_manager.current_language()
    }
    
    /// Get available languages
    pub fn available_languages(&self) -> Vec<Language> {
        self.i18n_manager.available_languages()
    }
    
    /// Set icon theme
    pub fn set_icon_theme(&mut self, theme: super::icon_manager::IconTheme) {
        self.icon_manager.set_theme(theme);
        info!("Icon theme changed to: {}", theme.name());
    }
    
    /// Get current icon theme
    pub fn current_icon_theme(&self) -> super::icon_manager::IconTheme {
        self.icon_manager.current_theme()
    }
    
    /// Get available icon themes
    pub fn available_icon_themes(&self) -> Vec<super::icon_manager::IconTheme> {
        super::icon_manager::IconTheme::all()
    }
    
    /// Get translation
    pub fn t(&self, key: &str) -> String {
        self.i18n_manager.t(key)
    }
}

#[cfg(feature = "ui")]
impl eframe::App for WindowsUI {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Apply Windows theme
        self.theme.apply(ctx);
        
        // Set window title
        // Note: set_window_title is not available in current eframe API
        // The title is set during window creation
        
        // Main UI layout
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_main_ui(ui, ctx);
        });
        
        // Render notifications
        self.render_notifications(ctx);
        
        // Render dialogs
        self.render_dialogs(ctx);
        
        // Handle background tasks
        self.handle_background_tasks(ctx);
        
        // Request repaint for animations
        ctx.request_repaint();
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
        
        // Save theme preference
        storage.set_string("theme", if self.theme.is_dark { "dark".to_string() } else { "light".to_string() });
    }
    
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }
}

#[cfg(feature = "ui")]
impl WindowsUI {
    /// Render main UI
    fn render_main_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Top navigation bar
        self.render_navigation_bar(ui);
        
        ui.separator();
        
        // Main content area with sidebar
        egui::TopBottomPanel::top("content_header").show(ctx, |ui| {
            self.render_content_header(ui);
        });
        
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(250.0)
            .min_width(200.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_main_content(ui, ctx);
        });
        
        // Bottom status bar
        self.render_status_bar(ui);
    }
    
    /// Render navigation bar
    fn render_navigation_bar(&mut self, ui: &mut egui::Ui) {
        let chat_label = self.t("nav.chat");
        let jobs_label = self.t("nav.jobs");
        let results_label = self.t("nav.results");
        let settings_label = self.t("nav.settings");
        let help_label = self.t("nav.help");
        
        let views = [
            ("chat", chat_label.as_str(), "💬"),
            ("jobs", jobs_label.as_str(), "📋"),
            ("results", results_label.as_str(), "📊"),
            ("settings", settings_label.as_str(), "⚙️"),
            ("help", help_label.as_str(), "❓"),
        ];
        
        let current_view_str = match self.state.current_view {
            View::Chat => "chat",
            View::Jobs => "jobs",
            View::Settings => "settings",
            View::Help => "help",
        };
        
        if let Some(selected_view) = WindowsComponents::navigation_bar(ui, current_view_str, &views) {
            self.state.current_view = match selected_view.as_str() {
                "chat" => View::Chat,
                "jobs" => View::Jobs,
                "settings" => View::Settings,
                "help" => View::Help,
                _ => View::Chat,
            };
        }
    }
    
    /// Render content header
    fn render_content_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let title = match self.state.current_view {
                View::Chat => self.t("chat.title"),
                View::Jobs => self.t("jobs.title"),
                View::Settings => self.t("settings.title"),
                View::Help => self.t("help.title"),
            };
            ui.heading(&title);
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle
                if ui.button(if self.theme.is_dark { "🌙" } else { "☀️" }).clicked() {
                    self.set_theme(!self.theme.is_dark);
                }
                
                ui.separator();
                
                // About button
                if ui.button("ℹ️ About").clicked() {
                    self.show_about = true;
                }
            });
        });
    }
    
    /// Render sidebar
    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        match self.state.current_view {
            View::Chat => self.render_chat_sidebar(ui),
            View::Jobs => self.render_jobs_sidebar(ui),
            View::Settings => self.render_settings_sidebar(ui),
            View::Help => self.render_help_sidebar(ui),
        }
    }
    
    /// Render chat sidebar
    fn render_chat_sidebar(&mut self, ui: &mut egui::Ui) {
        WindowsComponents::card_with_header(ui, "Quick Actions", |ui| {
            if ui.button("🔄 New Scraping Job").clicked() {
                self.chat.add_system_message("Ready for a new scraping request!".to_string());
            }
            
            if ui.button("📋 View Recent Jobs").clicked() {
                self.state.current_view = View::Jobs;
            }
            
            if ui.button("⚙️ Settings").clicked() {
                self.state.current_view = View::Settings;
            }
        });
        
        ui.add_space(16.0);
        
        WindowsComponents::card_with_header(ui, "Examples", |ui| {
            ui.label("Try these examples:");
            ui.add_space(8.0);
            
            let examples = [
                "Scrape product prices from shop.example.com",
                "Get news headlines from news.example.com",
                "Extract contact information from directory.example.com",
                "Find job listings from jobs.example.com",
            ];
            
            for example in examples {
                if ui.button(format!("💡 {}", example)).clicked() {
                    self.chat.add_user_message(example.to_string());
                }
                ui.add_space(4.0);
            }
        });
    }
    
    /// Render jobs sidebar
    fn render_jobs_sidebar(&mut self, ui: &mut egui::Ui) {
        WindowsComponents::card_with_header(ui, "Job Filters", |ui| {
            ui.label("Status:");
            ui.checkbox(&mut false, "Running");
            ui.checkbox(&mut false, "Completed");
            ui.checkbox(&mut false, "Failed");
            ui.checkbox(&mut false, "Queued");
            
            ui.add_space(8.0);
            
            ui.label("Date Range:");
            ui.label("Last 24 hours");
            ui.label("Last week");
            ui.label("Last month");
        });
        
        ui.add_space(16.0);
        
        WindowsComponents::card_with_header(ui, "Quick Actions", |ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_jobs();
            }
            
            if ui.button("🗑️ Clear Completed").clicked() {
                self.clear_completed_jobs();
            }
            
            if ui.button("📊 Export All").clicked() {
                self.export_all_jobs();
            }
        });
    }
    
    /// Render settings sidebar
    fn render_settings_sidebar(&mut self, ui: &mut egui::Ui) {
        WindowsComponents::card_with_header(ui, "Categories", |ui| {
            let categories = [
                ("general", "General", "⚙️"),
                ("scraping", "Scraping", "🕷️"),
                ("export", "Export", "📥"),
                ("security", "Security", "🔒"),
                ("ui", "Interface", "🎨"),
            ];
            
            for (id, label, icon) in categories {
                if ui.button(format!("{} {}", icon, label)).clicked() {
                    // Switch to category
                }
            }
        });
        
        ui.add_space(16.0);
        
        WindowsComponents::card_with_header(ui, "Actions", |ui| {
            if ui.button("💾 Save Settings").clicked() {
                self.save_settings();
            }
            
            if ui.button("🔄 Reset to Defaults").clicked() {
                self.reset_settings();
            }
            
            if ui.button("📤 Export Settings").clicked() {
                self.export_settings();
            }
            
            if ui.button("📥 Import Settings").clicked() {
                self.import_settings();
            }
        });
    }
    
    /// Render help sidebar
    fn render_help_sidebar(&mut self, ui: &mut egui::Ui) {
        WindowsComponents::card_with_header(ui, "Quick Help", |ui| {
            let help_items = [
                ("getting_started", "Getting Started", "🚀"),
                ("examples", "Examples", "💡"),
                ("troubleshooting", "Troubleshooting", "🔧"),
                ("faq", "FAQ", "❓"),
                ("contact", "Contact Support", "📞"),
            ];
            
            for (id, label, icon) in help_items {
                if ui.button(format!("{} {}", icon, label)).clicked() {
                    // Show help content
                }
            }
        });
        
        ui.add_space(16.0);
        
        WindowsComponents::card_with_header(ui, "Resources", |ui| {
            ui.hyperlink_to("📚 Documentation", "https://github.com/winscrape-studio/docs");
            ui.hyperlink_to("🐛 Report Bug", "https://github.com/winscrape-studio/issues");
            ui.hyperlink_to("💬 Community", "https://github.com/winscrape-studio/discussions");
            ui.hyperlink_to("⭐ Star Project", "https://github.com/winscrape-studio");
        });
    }
    
    /// Render main content
    fn render_main_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        match self.state.current_view {
            View::Chat => self.render_chat_view(ui, ctx),
            View::Jobs => self.render_jobs_view(ui, ctx),
            View::Settings => self.render_settings_view(ui, ctx),
            View::Help => self.render_help_view(ui, ctx),
        }
    }
    
    /// Render chat view
    fn render_chat_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        WindowsComponents::card_with_header(ui, "Natural Language Input", |ui| {
            ui.label("Describe what you want to scrape in plain English. The AI will generate a scraping plan for you.");
            ui.add_space(8.0);
            
            // Chat interface
            self.chat.render(ui, ctx);
            
            // Handle chat input
            if let Some(user_input) = self.chat.get_pending_input() {
                self.handle_chat_input(user_input, ctx);
            }
        });
        
        // Show pending approval if any
        if let Some(approval) = self.state.pending_approval.clone() {
            self.render_approval_dialog(ui, &approval);
        }
    }
    
    /// Render jobs view
    fn render_jobs_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // If a results viewer is active (populated by "View Results" on a
        // job card), show it instead of the job list. It was previously
        // being populated but never actually rendered anywhere.
        if self.results_viewer.is_some() {
            if ui.button("⬅ Back to Jobs").clicked() {
                self.results_viewer = None;
                return;
            }
            ui.add_space(4.0);
            if let Some(viewer) = &mut self.results_viewer {
                viewer.render(ui);
            }
            return;
        }
        
        if self.state.jobs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("No Jobs Yet");
                    ui.add_space(16.0);
                    ui.label("Start by describing what you want to scrape in the Chat tab.");
                    ui.add_space(16.0);
                    if ui.button("Go to Chat").clicked() {
                        self.state.current_view = View::Chat;
                    }
                });
            });
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let jobs = self.state.jobs.clone();
                    for job in &jobs {
                        self.render_job_card(ui, job);
                        ui.add_space(8.0);
                    }
                });
        }
    }
    
    /// Render settings view
    fn render_settings_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // General Settings
                WindowsComponents::card_with_header(ui, &self.t("settings.general"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(&self.t("settings.theme"));
                        if ui.selectable_label(self.theme.is_dark, &self.t("settings.theme.dark")).clicked() {
                            self.set_theme(true);
                        }
                        if ui.selectable_label(!self.theme.is_dark, &self.t("settings.theme.light")).clicked() {
                            self.set_theme(false);
                        }
                    });
                    
                    ui.add_space(8.0);
                    
                    // Language selection
                    ui.horizontal(|ui| {
                        ui.label(&self.t("settings.language"));
                        let current = self.current_language();
                        let mut chosen = current;
                        egui::ComboBox::from_id_source("language_combo")
                            .selected_text(current.name())
                            .show_ui(ui, |ui| {
                                for language in self.available_languages() {
                                    ui.selectable_value(&mut chosen, language, language.name());
                                }
                            });
                        if chosen != current {
                            self.set_language(chosen);
                        }
                    });
                    
                    ui.add_space(8.0);
                    
                    // Icon theme selection
                    ui.horizontal(|ui| {
                        ui.label(&self.t("settings.icon_theme"));
                        let mut current_icon_theme = self.current_icon_theme();
                        egui::ComboBox::from_id_source("icon_theme_combo")
                            .selected_text(&self.t(current_icon_theme.translation_key()))
                            .show_ui(ui, |ui| {
                                for theme in self.available_icon_themes() {
                                    if ui.selectable_value(
                                        &mut current_icon_theme,
                                        theme,
                                        &self.t(theme.translation_key())
                                    ).clicked() {
                                        self.set_icon_theme(current_icon_theme);
                                    }
                                }
                            });
                    });
                    
                    ui.add_space(8.0);
                    
                    WindowsComponents::checkbox(ui, &self.t("settings.auto_save"), &mut self.config_draft.ui.auto_save);
                    WindowsComponents::checkbox(ui, &self.t("settings.notifications"), &mut self.config_draft.ui.enable_notifications);
                    WindowsComponents::checkbox(ui, &self.t("settings.minimize_to_tray"), &mut self.config_draft.ui.minimize_to_tray);
                });
                
                ui.add_space(16.0);
                
                // Scraping Settings
                WindowsComponents::card_with_header(ui, "Scraping Settings", |ui| {
                    ui.colored_label(egui::Color32::from_gray(150), "Applies on next restart");
                    let mut max_requests = self.config_draft.scraping.max_concurrent_requests as f32;
                    let mut timeout = self.config_draft.scraping.request_timeout_seconds as f32;
                    let mut max_retries = self.config_draft.scraping.max_retries as f32;
                    let mut retry_delay = self.config_draft.scraping.retry_delay_seconds as f32;
                    let mut default_delay = self.config_draft.scraping.default_delay_ms as f32;
                    let mut browser_timeout = self.config_draft.scraping.browser_timeout_seconds as f32;
                    WindowsComponents::slider(ui, "Max concurrent requests", &mut max_requests, 1.0, 20.0);
                    WindowsComponents::slider(ui, "Request timeout (seconds)", &mut timeout, 5.0, 120.0);
                    WindowsComponents::slider(ui, "Max retries per request", &mut max_retries, 0.0, 10.0);
                    WindowsComponents::slider(ui, "Retry delay (seconds)", &mut retry_delay, 0.0, 30.0);
                    WindowsComponents::slider(ui, "Default delay between requests (ms)", &mut default_delay, 0.0, 5000.0);
                    WindowsComponents::slider(ui, "Browser fallback timeout (seconds)", &mut browser_timeout, 5.0, 180.0);
                    self.config_draft.scraping.max_concurrent_requests = max_requests as usize;
                    self.config_draft.scraping.request_timeout_seconds = timeout as u64;
                    self.config_draft.scraping.max_retries = max_retries as usize;
                    self.config_draft.scraping.retry_delay_seconds = retry_delay as u64;
                    self.config_draft.scraping.default_delay_ms = default_delay as u64;
                    self.config_draft.scraping.browser_timeout_seconds = browser_timeout as u64;
                    
                    ui.add_space(8.0);
                    
                    WindowsComponents::checkbox(ui, "Respect robots.txt", &mut self.config_draft.scraping.respect_robots_txt);
                    WindowsComponents::checkbox(ui, "Enable browser fallback", &mut self.config_draft.scraping.enable_browser_fallback);
                });
                
                ui.add_space(16.0);
                
                // Export Settings
                WindowsComponents::card_with_header(ui, "Export Settings", |ui| {
                    let formats = ["csv", "json", "xlsx", "parquet"];
                    WindowsComponents::dropdown(ui, "Default export format", &mut self.config_draft.export.default_format, &formats.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                    
                    ui.add_space(8.0);
                    
                    let mut max_file_size = self.config_draft.export.max_file_size_mb as f32;
                    WindowsComponents::slider(ui, "Max export file size (MB)", &mut max_file_size, 1.0, 1000.0);
                    self.config_draft.export.max_file_size_mb = max_file_size as usize;
                    
                    ui.add_space(8.0);
                    
                    ui.label("Output directory:");
                    let mut output_dir = self.config_draft.export.output_directory.display().to_string();
                    WindowsComponents::folder_picker_button(ui, "", &mut output_dir);
                    self.config_draft.export.output_directory = std::path::PathBuf::from(output_dir);
                    
                    ui.add_space(8.0);
                    
                    WindowsComponents::checkbox(ui, "Include metadata in exports", &mut self.config_draft.export.include_metadata);
                    WindowsComponents::checkbox(ui, "Compress large exports", &mut self.config_draft.export.compression_enabled);
                });
                
                ui.add_space(16.0);
                
                // Security Settings
                WindowsComponents::card_with_header(ui, "Security Settings", |ui| {
                    ui.colored_label(egui::Color32::from_gray(150), "Applies on next restart");
                    WindowsComponents::checkbox(ui, "Enable input validation", &mut self.config_draft.security.enable_input_validation);
                    WindowsComponents::checkbox(ui, "Filter sensitive data from output", &mut self.config_draft.security.enable_output_filtering);
                    WindowsComponents::checkbox(ui, "Enable rate limiting", &mut self.config_draft.security.enable_rate_limiting);
                    
                    ui.add_space(8.0);
                    
                    let mut max_input_len = self.config_draft.security.max_input_length as f32;
                    let mut rate_limit = self.config_draft.security.rate_limit_requests_per_minute as f32;
                    WindowsComponents::slider(ui, "Max input length (characters)", &mut max_input_len, 100.0, 20000.0);
                    WindowsComponents::slider(ui, "Rate limit (requests/minute)", &mut rate_limit, 1.0, 300.0);
                    self.config_draft.security.max_input_length = max_input_len as usize;
                    self.config_draft.security.rate_limit_requests_per_minute = rate_limit as usize;
                    
                    ui.add_space(8.0);
                    
                    ui.label("Blocked domains (one per line):");
                    ui.add(egui::TextEdit::multiline(&mut self.blocked_domains_text).hint_text("Enter domains to block, one per line"));
                    
                    ui.add_space(8.0);
                    
                    ui.label("Allowed URL schemes (one per line):");
                    ui.add(egui::TextEdit::multiline(&mut self.allowed_schemes_text).hint_text("http\nhttps"));
                });
                
                ui.add_space(16.0);
                
                // LLM Settings
                WindowsComponents::card_with_header(ui, "Natural Language Processing", |ui| {
                    ui.colored_label(egui::Color32::from_gray(150), "Applies on next restart");
                    
                    ui.label("Local GGUF model (candle) - fully offline, no server needed:");
                    if cfg!(feature = "local-llm") {
                        WindowsComponents::checkbox(ui, "Enable local GGUF inference", &mut self.config_draft.llm.enable_candle);
                        
                        ui.label("Model file (.gguf):");
                        let mut model_path = self.config_draft.llm.model_path.display().to_string();
                        WindowsComponents::file_picker_button(ui, "", &mut model_path);
                        self.config_draft.llm.model_path = std::path::PathBuf::from(model_path);
                        ui.small("You need to download a compatible GGUF model yourself - this app doesn't fetch models.");
                    } else {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "This build wasn't compiled with the 'local-llm' feature, so this option isn't available. Rebuild with --features local-llm to enable it.",
                        );
                    }
                    
                    ui.add_space(12.0);
                    
                    ui.label("Local Ollama server - free, runs on your machine, needs Ollama installed separately:");
                    WindowsComponents::checkbox(ui, "Enable Ollama", &mut self.config_draft.llm.enable_ollama);
                    
                    ui.add_space(8.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("Ollama server URL:");
                        ui.text_edit_singleline(&mut self.config_draft.llm.ollama_url);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Model name:");
                        ui.text_edit_singleline(&mut self.config_draft.llm.ollama_model);
                    });
                    ui.small("Must be pulled first, e.g. run: ollama pull llama3.2");
                    
                    ui.add_space(8.0);
                    ui.small("If a local GGUF model and/or Ollama are both unavailable or disabled, natural-language descriptions still work via a built-in rule-based generator - no AI backend is required to use this app.");
                });
                
                ui.add_space(16.0);
                
                // Save / Reset
                ui.horizontal(|ui| {
                    if ui.button("💾 Save Settings").clicked() {
                        self.save_settings();
                    }
                    if ui.button("Reset to Defaults").clicked() {
                        self.reset_settings();
                    }
                });
            });
    }
    
    /// Persist config_draft (including the blocked-domains text buffer) to
    /// disk so Settings changes actually survive an app restart. Some
    /// fields (concurrency, robots.txt, browser fallback, security
    /// validation toggles) only take effect on the *next* app start since
    /// the scraping/security engines are built once at startup - this is
    /// called out in the UI next to those sections.
    fn save_settings(&mut self) {
        self.config_draft.security.blocked_domains = self.blocked_domains_text
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.config_draft.security.allowed_schemes = self.allowed_schemes_text
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        let config = self.config_draft.clone();
        let tx = self.ui_event_tx.clone();
        
        tokio::spawn(async move {
            let event = match config.validate() {
                Ok(()) => match config.save().await {
                    Ok(()) => UiEvent::Notify {
                        level: NotificationLevel::Success,
                        title: "Settings Saved".to_string(),
                        message: "Your settings have been saved.".to_string(),
                    },
                    Err(e) => {
                        error!("Failed to save settings: {}", e);
                        UiEvent::Notify {
                            level: NotificationLevel::Error,
                            title: "Save Failed".to_string(),
                            message: e.to_string(),
                        }
                    }
                },
                Err(e) => UiEvent::Notify {
                    level: NotificationLevel::Error,
                    title: "Invalid Settings".to_string(),
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }
    
    /// Reset the in-progress draft back to defaults (not yet saved until
    /// the user clicks "Save Settings").
    fn reset_settings(&mut self) {
        self.config_draft = crate::config::AppConfig::default();
        self.blocked_domains_text = self.config_draft.security.blocked_domains.join("\n");
        self.allowed_schemes_text = self.config_draft.security.allowed_schemes.join("\n");
        self.add_notification(
            NotificationLevel::Info,
            "Settings Reset".to_string(),
            "Settings reset to defaults. Click Save to persist.".to_string(),
        );
    }
    
    /// Render help view
    fn render_help_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                WindowsComponents::card_with_header(ui, "Getting Started", |ui| {
                    ui.label("1. Go to the Chat tab");
                    ui.label("2. Describe what you want to scrape in plain English");
                    ui.label("3. Review the generated scraping plan");
                    ui.label("4. Approve and run the scraping job");
                    ui.label("5. Export your results");
                });
                
                ui.add_space(16.0);
                
                WindowsComponents::card_with_header(ui, "Example Requests", |ui| {
                    let examples = [
                        "Scrape product prices from shop.example.com",
                        "Get news headlines from news.example.com",
                        "Extract contact information from directory.example.com",
                        "Find job listings from jobs.example.com",
                        "Get product reviews from review.example.com",
                    ];
                    
                    for example in examples {
                        ui.label(format!("• \"{}\"", example));
                    }
                });
                
                ui.add_space(16.0);
                
                WindowsComponents::card_with_header(ui, "Features", |ui| {
                    let features = [
                        "✅ Natural language to scraping plan conversion",
                        "✅ HTTP-first with browser fallback",
                        "✅ Robots.txt compliance",
                        "✅ Rate limiting and anti-blocking",
                        "✅ Multiple export formats (CSV, JSON, XLSX, Parquet)",
                        "✅ Data validation and filtering",
                        "✅ Real-time job monitoring",
                        "✅ Windows-native interface",
                    ];
                    
                    for feature in features {
                        ui.label(feature);
                    }
                });
                
                ui.add_space(16.0);
                
                WindowsComponents::card_with_header(ui, "About", |ui| {
                    ui.label(format!("WinScrape Studio v{}", env!("CARGO_PKG_VERSION")));
                    ui.label("A natural language web scraping tool");
                    ui.label("Built with Rust and egui");
                    ui.hyperlink_to("Documentation", "https://github.com/winscrape-studio/docs");
                    ui.hyperlink_to("GitHub Repository", "https://github.com/winscrape-studio");
                });
            });
    }
    
    /// Render job card
    fn render_job_card(&mut self, ui: &mut egui::Ui, job: &JobInfo) {
        WindowsComponents::card_with_header(ui, &job.title, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(&job.description);
                    ui.label(format!("Created: {}", job.created_at.format("%Y-%m-%d %H:%M")));
                    if let Some(completed_at) = job.completed_at {
                        ui.label(format!("Completed: {}", completed_at.format("%Y-%m-%d %H:%M")));
                    }
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Status indicator
                    let (color, text) = match job.status {
                        JobStatus::Running => (self.theme.get_status_color("running"), "🔄 Running"),
                        JobStatus::Completed => (self.theme.get_status_color("completed"), "✅ Completed"),
                        JobStatus::Failed => (self.theme.get_status_color("error"), "❌ Failed"),
                        JobStatus::Queued => (self.theme.get_status_color("info"), "⏳ Queued"),
                        JobStatus::Cancelled => (self.theme.get_status_color("warning"), "🚫 Cancelled"),
                    };
                    
                    ui.colored_label(color, text);
                });
            });
            
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                if ui.button("📊 View Results").clicked() {
                    self.view_job_results(&job.id);
                }
                
                if ui.button("📥 Export").clicked() {
                    self.export_job_results(&job.id);
                }
                
                if matches!(job.status, JobStatus::Running) {
                    if ui.button("⏹️ Cancel").clicked() {
                        self.cancel_job(&job.id);
                    }
                }
                
                if ui.button("🔄 Rerun").clicked() {
                    self.rerun_job(&job.id);
                }
            });
        });
    }
    
    /// Render approval dialog
    fn render_approval_dialog(&mut self, ui: &mut egui::Ui, approval: &crate::core::orchestrator::PendingApproval) {
        egui::Window::new("Review Scraping Plan")
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ui.ctx(), |ui| {
                ui.label("Please review the generated scraping plan:");
                ui.add_space(8.0);
                
                // Show DSL preview
                if let Ok(dsl_yaml) = serde_yaml::to_string(&approval.dsl) {
                    ui.add(egui::TextEdit::multiline(&mut dsl_yaml.clone())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(15));
                }
                
                ui.add_space(8.0);
                
                ui.horizontal(|ui| {
                    if ui.button("✅ Approve & Run").clicked() {
                        self.approve_scraping_plan();
                    }
                    
                    if ui.button("❌ Reject").clicked() {
                        self.reject_scraping_plan();
                    }
                    
                    if ui.button("✏️ Edit").clicked() {
                        self.edit_scraping_plan();
                    }
                });
            });
    }
    
    /// Render status bar
    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        let job_count = format!("{}", self.state.jobs.len());
        let mut status_items = vec![
            ("Jobs", self.theme.get_status_color("info")),
            (job_count.as_str(), self.theme.get_status_color("info")),
        ];
        
        if let Some(status) = &self.state.status_message {
            status_items.push((status.as_str(), self.theme.get_status_color("info")));
        }
        
        WindowsComponents::status_bar(ui, &status_items);
    }
    
    /// Render notifications
    fn render_notifications(&mut self, ctx: &egui::Context) {
        let mut to_remove = Vec::new();
        let now = chrono::Utc::now();
        // Auto-close notifications were previously never actually
        // auto-closed: the `auto_close`/`timestamp` fields were set but
        // nothing ever checked elapsed time, so every notification
        // (including routine success toasts) sat on screen until the
        // user manually clicked "Close".
        const AUTO_CLOSE_AFTER: chrono::Duration = chrono::Duration::seconds(5);
        
        for (i, notification) in self.notifications.iter().enumerate() {
            if notification.auto_close && now - notification.timestamp > AUTO_CLOSE_AFTER {
                to_remove.push(notification.id.clone());
                continue;
            }
            
            let mut open = true;
            
            egui::Window::new(&notification.title)
                .id(egui::Id::new(&notification.id))
                .anchor(egui::Align2::RIGHT_TOP, [-16.0, 16.0 + (i as f32 * 100.0)])
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    WindowsComponents::notification(ui, notification.level, &notification.title, &notification.message);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            open = false;
                        }
                    });
                });
            
            if !open {
                to_remove.push(notification.id.clone());
            }
        }
        
        for id in to_remove {
            self.remove_notification(&id);
        }
        
        // Keep repainting periodically while any auto-close notification
        // is pending, so its timeout actually gets checked even if the
        // user isn't interacting with the app (egui otherwise only
        // redraws on input).
        if self.notifications.iter().any(|n| n.auto_close) {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
    
    /// Render dialogs
    fn render_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_about {
            self.render_about_dialog(ctx);
        }
        
        if self.show_export_dialog {
            self.render_export_dialog(ctx);
        }
    }
    
    /// Render about dialog
    fn render_about_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("About WinScrape Studio")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("🕷️ WinScrape Studio");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(16.0);
                    
                    ui.label("A natural language web scraping tool");
                    ui.label("Built with Rust and egui");
                    ui.add_space(16.0);
                    
                    ui.hyperlink_to("GitHub Repository", "https://github.com/winscrape-studio");
                    ui.hyperlink_to("Documentation", "https://github.com/winscrape-studio/docs");
                    ui.add_space(16.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
            });
    }
    
    /// Render export dialog
    fn render_export_dialog(&mut self, ctx: &egui::Context) {
        let job_id = self.export_job_id.clone();
        egui::Window::new("Export Data")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                if let Some(job_id) = &job_id {
                    ui.label(format!("Exporting job: {}", job_id));
                    ui.add_space(4.0);
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "No job selected to export.");
                    ui.add_space(4.0);
                }
                
                ui.label("Export Format:");
                let formats = ["CSV", "JSON", "XLSX", "Parquet"];
                WindowsComponents::dropdown(ui, "", &mut self.export_format, &formats.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                
                ui.add_space(8.0);
                
                ui.label("Output Path:");
                WindowsComponents::file_picker_button(ui, "", &mut self.export_path);
                
                ui.add_space(16.0);
                
                ui.horizontal(|ui| {
                    let can_export = job_id.is_some() && !self.export_path.trim().is_empty();
                    if ui.add_enabled(can_export, egui::Button::new("Export")).clicked() {
                        self.perform_export();
                        self.show_export_dialog = false;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        self.show_export_dialog = false;
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
        self.state.current_workflow = Some(WorkflowState::Processing);
        self.state.status_message = Some("Processing your request...".to_string());
        
        // Add notification
        self.add_notification(
            NotificationLevel::Info,
            "Processing Request".to_string(),
            "Generating scraping plan from your description...".to_string(),
        );
        
        // Spawn async task to run the natural-language -> DSL -> preview
        // workflow, then hand the result back to the UI thread through the
        // channel so `handle_background_tasks` can pick it up next frame.
        let app = self.app.clone();
        let ctx = ctx.clone();
        let tx = self.workflow_tx.clone();
        
        tokio::spawn(async move {
            info!("Starting workflow for input: {}", input);
            
            let orchestrator = crate::core::orchestrator::Orchestrator::new(app);
            // auto_approve = false: stop at the approval gate so the user
            // can review the generated plan and preview before we scrape.
            let outcome = orchestrator.execute_complete_workflow(&input, false).await;
            
            let workflow_state = match outcome {
                Ok(result) => WorkflowState::Completed(result),
                Err(e) => {
                    error!("Workflow execution failed: {}", e);
                    WorkflowState::Failed(e.to_string())
                }
            };
            
            if tx.send(workflow_state).is_err() {
                warn!("Failed to deliver workflow result: UI receiver was dropped");
            }
            
            // Wake the UI so it redraws immediately instead of waiting for
            // the next scheduled repaint.
            ctx.request_repaint();
        });
    }
    
    /// Handle background tasks
    fn handle_background_tasks(&mut self, ctx: &egui::Context) {
        // Pick up results delivered by spawned workflow tasks (see
        // handle_chat_input) and fold them into current_workflow so the
        // match below drives the UI update.
        while let Ok(workflow_state) = self.workflow_rx.try_recv() {
            self.state.current_workflow = Some(workflow_state);
        }
        
        while let Ok(jobs) = self.jobs_rx.try_recv() {
            self.state.jobs = jobs;
        }
        
        while let Ok(event) = self.ui_event_rx.try_recv() {
            match event {
                UiEvent::JobResultsLoaded { job_id, results } => {
                    let data: Vec<std::collections::HashMap<String, serde_json::Value>> = results
                        .into_iter()
                        .filter_map(|v| match v {
                            serde_json::Value::Object(obj) => Some(obj.into_iter().collect()),
                            _ => None,
                        })
                        .collect();
                    self.results_viewer = Some(ResultsViewer::new(job_id, data));
                    self.state.current_view = View::Jobs;
                }
                UiEvent::Notify { level, title, message } => {
                    self.add_notification(level, title, message);
                }
            }
        }
        
        // Check for completed workflows
        if let Some(workflow_state) = &self.state.current_workflow {
            match workflow_state {
                WorkflowState::Processing => {
                    // Show processing indicator
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                WorkflowState::Completed(result) => {
                    self.handle_workflow_completion(result.clone());
                    self.state.current_workflow = None;
                }
                WorkflowState::Failed(error) => {
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
                self.add_notification(
                    NotificationLevel::Info,
                    "Scraping Plan Ready".to_string(),
                    "Please review and approve the generated scraping plan.".to_string(),
                );
                if let Some(approval) = &result.pending_approval {
                    self.state.pending_approval = Some(approval.clone());
                }
            }
            WorkflowStage::Completed => {
                self.add_notification(
                    NotificationLevel::Success,
                    "Scraping Completed".to_string(),
                    "Your scraping job has completed successfully!".to_string(),
                );
                if let Some(job_id) = &result.job_id {
                    self.refresh_job_details(job_id);
                }
            }
            WorkflowStage::Failed => {
                let error_msg = result.errors.join("; ");
                self.add_notification(
                    NotificationLevel::Error,
                    "Scraping Failed".to_string(),
                    format!("Scraping failed: {}", error_msg),
                );
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
        self.add_notification(
            NotificationLevel::Error,
            "Error".to_string(),
            error,
        );
        self.state.status_message = None;
    }
    
    /// Refresh jobs list
    fn refresh_jobs(&mut self) {
        self.state.last_job_refresh = std::time::Instant::now();
        debug!("Refreshing jobs list");
        
        let app = self.app.clone();
        let tx = self.jobs_tx.clone();
        
        tokio::spawn(async move {
            match app.list_jobs(50).await {
                Ok(jobs) => {
                    let job_infos = jobs.into_iter().map(storage_job_to_info).collect();
                    
                    if tx.send(job_infos).is_err() {
                        warn!("Failed to deliver refreshed jobs list: UI receiver was dropped");
                    }
                }
                Err(e) => {
                    error!("Failed to refresh jobs list: {}", e);
                }
            }
        });
    }
    
    /// Refresh specific job details. There's no separate per-job detail
    /// view distinct from the jobs list, so this just re-fetches the
    /// whole list - previously this was a no-op stub that only logged.
    fn refresh_job_details(&mut self, _job_id: &str) {
        self.refresh_jobs();
    }
    
    /// View job results
    fn view_job_results(&mut self, job_id: &str) {
        info!("Viewing results for job: {}", job_id);
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        let job_id_owned = job_id.to_string();
        
        tokio::spawn(async move {
            match app.get_job_results(&job_id_owned).await {
                Ok(results) => {
                    let _ = tx.send(UiEvent::JobResultsLoaded {
                        job_id: job_id_owned,
                        results,
                    });
                }
                Err(e) => {
                    error!("Failed to load results for job {}: {}", job_id_owned, e);
                    let _ = tx.send(UiEvent::Notify {
                        level: NotificationLevel::Error,
                        title: "Failed to Load Results".to_string(),
                        message: e.to_string(),
                    });
                }
            }
        });
    }
    
    /// Export job results
    fn export_job_results(&mut self, job_id: &str) {
        info!("Opening export dialog for job: {}", job_id);
        self.export_job_id = Some(job_id.to_string());
        if self.export_path.trim().is_empty() {
            self.export_path = crate::export::ExportManager::generate_filename(
                job_id,
                &crate::export::ExportFormat::Csv,
            );
        }
        self.show_export_dialog = true;
    }
    
    /// Cancel running job
    fn cancel_job(&mut self, job_id: &str) {
        info!("Cancelling job: {}", job_id);
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        let jobs_tx = self.jobs_tx.clone();
        let job_id_owned = job_id.to_string();
        
        tokio::spawn(async move {
            let event = match app.cancel_job(&job_id_owned).await {
                Ok(()) => UiEvent::Notify {
                    level: NotificationLevel::Info,
                    title: "Job Cancelled".to_string(),
                    message: format!("Job {} has been cancelled.", job_id_owned),
                },
                Err(e) => {
                    error!("Failed to cancel job {}: {}", job_id_owned, e);
                    UiEvent::Notify {
                        level: NotificationLevel::Error,
                        title: "Cancel Failed".to_string(),
                        message: e.to_string(),
                    }
                }
            };
            let _ = tx.send(event);
            
            // Refresh the jobs list so the cancelled/failed-to-cancel
            // status is reflected right away.
            if let Ok(jobs) = app.list_jobs(50).await {
                let job_infos = jobs.into_iter().map(storage_job_to_info).collect();
                let _ = jobs_tx.send(job_infos);
            }
        });
    }
    
    /// Rerun job
    /// Rerun job: fetch the original job's saved DSL plan and start a new
    /// scraping run with it. Previously this only showed a fake "Job
    /// Restarted" notification without calling anything.
    fn rerun_job(&mut self, job_id: &str) {
        info!("Rerunning job: {}", job_id);
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        let jobs_tx = self.jobs_tx.clone();
        let job_id_owned = job_id.to_string();
        
        tokio::spawn(async move {
            let event = async {
                let job = app.get_job(&job_id_owned).await?;
                let plan = crate::dsl::parser::DSLParser::parse_yaml(&job.plan_yaml)?;
                let new_job_id = app.execute_scraping(&plan).await?;
                Ok::<String, anyhow::Error>(new_job_id)
            }.await;
            
            let ui_event = match event {
                Ok(new_job_id) => UiEvent::Notify {
                    level: NotificationLevel::Success,
                    title: "Job Restarted".to_string(),
                    message: format!("Started new job {} using the same plan as {}.", new_job_id, job_id_owned),
                },
                Err(e) => {
                    error!("Failed to rerun job {}: {}", job_id_owned, e);
                    UiEvent::Notify {
                        level: NotificationLevel::Error,
                        title: "Rerun Failed".to_string(),
                        message: e.to_string(),
                    }
                }
            };
            let _ = tx.send(ui_event);
            
            if let Ok(jobs) = app.list_jobs(50).await {
                let job_infos = jobs.into_iter().map(storage_job_to_info).collect();
                let _ = jobs_tx.send(job_infos);
            }
        });
    }
    
    /// Export the current settings draft to a JSON file the user picks.
    fn export_settings(&mut self) {
        // Fold the pending blocked-domains/allowed-schemes text edits in
        // before exporting, same as Save does.
        let mut config = self.config_draft.clone();
        config.security.blocked_domains = self.blocked_domains_text
            .lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        config.security.allowed_schemes = self.allowed_schemes_text
            .lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("winscrape-settings.json")
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        
        match serde_json::to_string_pretty(&config) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => self.add_notification(
                    NotificationLevel::Success,
                    "Settings Exported".to_string(),
                    format!("Settings exported to {}", path.display()),
                ),
                Err(e) => self.add_notification(
                    NotificationLevel::Error,
                    "Export Failed".to_string(),
                    e.to_string(),
                ),
            },
            Err(e) => self.add_notification(
                NotificationLevel::Error,
                "Export Failed".to_string(),
                e.to_string(),
            ),
        }
    }
    
    /// Import settings from a JSON file the user picks, replacing the
    /// current draft (not yet saved to disk until "Save Settings").
    fn import_settings(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        
        let result: Result<crate::config::AppConfig, String> = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|contents| serde_json::from_str(&contents).map_err(|e| e.to_string()));
        
        match result {
            Ok(config) => {
                self.blocked_domains_text = config.security.blocked_domains.join("\n");
                self.allowed_schemes_text = config.security.allowed_schemes.join("\n");
                self.config_draft = config;
                self.add_notification(
                    NotificationLevel::Success,
                    "Settings Imported".to_string(),
                    "Settings loaded. Click Save Settings to apply.".to_string(),
                );
            }
            Err(e) => {
                self.add_notification(
                    NotificationLevel::Error,
                    "Import Failed".to_string(),
                    format!("Could not read settings file: {}", e),
                );
            }
        }
    }
    
    /// Clear completed jobs: previously this only removed them from the
    /// local in-memory `state.jobs` list, which did nothing durable -
    /// they'd reappear on the very next `refresh_jobs()` call (including
    /// the automatic periodic refresh) since they were never actually
    /// deleted from storage. Now actually deletes them.
    fn clear_completed_jobs(&mut self) {
        let completed_ids: Vec<String> = self.state.jobs.iter()
            .filter(|job| matches!(job.status, JobStatus::Completed))
            .map(|job| job.id.clone())
            .collect();
        
        if completed_ids.is_empty() {
            self.add_notification(
                NotificationLevel::Info,
                "Nothing to Clear".to_string(),
                "No completed jobs to clear.".to_string(),
            );
            return;
        }
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        let jobs_tx = self.jobs_tx.clone();
        
        tokio::spawn(async move {
            let mut deleted = 0usize;
            let mut failed = 0usize;
            for job_id in &completed_ids {
                match app.delete_job(job_id).await {
                    Ok(()) => deleted += 1,
                    Err(e) => {
                        error!("Failed to delete job {}: {}", job_id, e);
                        failed += 1;
                    }
                }
            }
            
            let event = if failed == 0 {
                UiEvent::Notify {
                    level: NotificationLevel::Success,
                    title: "Jobs Cleared".to_string(),
                    message: format!("Deleted {} completed job(s).", deleted),
                }
            } else {
                UiEvent::Notify {
                    level: NotificationLevel::Error,
                    title: "Some Jobs Not Cleared".to_string(),
                    message: format!("Deleted {} job(s), failed to delete {}.", deleted, failed),
                }
            };
            let _ = tx.send(event);
            
            if let Ok(jobs) = app.list_jobs(50).await {
                let job_infos = jobs.into_iter().map(storage_job_to_info).collect();
                let _ = jobs_tx.send(job_infos);
            }
        });
    }
    
    /// Export all completed jobs. Previously this just opened the
    /// per-job export dialog without ever setting `export_job_id`, so
    /// clicking "Export" afterward always hit the "No job selected"
    /// error - the button was a dead end. Now it picks an output folder
    /// and exports every completed job's results to it as CSV.
    fn export_all_jobs(&mut self) {
        let completed_ids: Vec<String> = self.state.jobs.iter()
            .filter(|job| matches!(job.status, JobStatus::Completed))
            .map(|job| job.id.clone())
            .collect();
        
        if completed_ids.is_empty() {
            self.add_notification(
                NotificationLevel::Info,
                "Nothing to Export".to_string(),
                "No completed jobs to export.".to_string(),
            );
            return;
        }
        
        let Some(dir) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        
        tokio::spawn(async move {
            let mut exported = 0usize;
            let mut failed = 0usize;
            for job_id in &completed_ids {
                let path = dir.join(crate::export::ExportManager::generate_filename(
                    job_id,
                    &crate::export::ExportFormat::Csv,
                ));
                match app.export_job(job_id, &path.display().to_string(), crate::export::ExportFormat::Csv).await {
                    Ok(()) => exported += 1,
                    Err(e) => {
                        error!("Failed to export job {}: {}", job_id, e);
                        failed += 1;
                    }
                }
            }
            
            let event = if failed == 0 {
                UiEvent::Notify {
                    level: NotificationLevel::Success,
                    title: "Export Complete".to_string(),
                    message: format!("Exported {} job(s) to {}", exported, dir.display()),
                }
            } else {
                UiEvent::Notify {
                    level: NotificationLevel::Error,
                    title: "Some Exports Failed".to_string(),
                    message: format!("Exported {} job(s), {} failed.", exported, failed),
                }
            };
            let _ = tx.send(event);
        });
    }
    
    /// Perform export
    fn perform_export(&mut self) {
        let Some(job_id) = self.export_job_id.clone() else {
            self.add_notification(
                NotificationLevel::Error,
                "Export Failed".to_string(),
                "No job selected to export.".to_string(),
            );
            return;
        };
        
        let format = match self.export_format.to_lowercase().parse::<crate::export::ExportFormat>() {
            Ok(f) => f,
            Err(e) => {
                self.add_notification(NotificationLevel::Error, "Export Failed".to_string(), e.to_string());
                return;
            }
        };
        let output_path = self.export_path.clone();
        
        info!("Exporting job {} to {} as {:?}", job_id, output_path, self.export_format);
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        
        tokio::spawn(async move {
            let event = match app.export_job(&job_id, &output_path, format).await {
                Ok(()) => UiEvent::Notify {
                    level: NotificationLevel::Success,
                    title: "Export Complete".to_string(),
                    message: format!("Job {} exported to {}", job_id, output_path),
                },
                Err(e) => {
                    error!("Failed to export job {}: {}", job_id, e);
                    UiEvent::Notify {
                        level: NotificationLevel::Error,
                        title: "Export Failed".to_string(),
                        message: e.to_string(),
                    }
                }
            };
            let _ = tx.send(event);
        });
    }
    
    /// Approve scraping plan: actually kick off execution of the approved
    /// DSL. Previously this only cleared the pending-approval state and
    /// showed a fake "execution started" notification without calling
    /// anything - the single most important button in the app did nothing.
    fn approve_scraping_plan(&mut self) {
        let Some(approval) = self.state.pending_approval.take() else {
            warn!("Approve clicked with no pending plan");
            return;
        };
        
        let app = self.app.clone();
        let tx = self.ui_event_tx.clone();
        let jobs_tx = self.jobs_tx.clone();
        
        tokio::spawn(async move {
            let event = match app.execute_scraping(&approval.dsl).await {
                Ok(job_id) => {
                    info!("Scraping job {} started from approved plan", job_id);
                    UiEvent::Notify {
                        level: NotificationLevel::Success,
                        title: "Plan Approved".to_string(),
                        message: format!("Scraping job {} started.", job_id),
                    }
                }
                Err(e) => {
                    error!("Failed to start scraping job: {}", e);
                    UiEvent::Notify {
                        level: NotificationLevel::Error,
                        title: "Failed to Start Job".to_string(),
                        message: e.to_string(),
                    }
                }
            };
            let _ = tx.send(event);
            
            if let Ok(jobs) = app.list_jobs(50).await {
                let job_infos = jobs.into_iter().map(storage_job_to_info).collect();
                let _ = jobs_tx.send(job_infos);
            }
        });
    }
    
    /// Reject scraping plan
    fn reject_scraping_plan(&mut self) {
        self.state.pending_approval = None;
        self.add_notification(
            NotificationLevel::Info,
            "Plan Rejected".to_string(),
            "Scraping plan has been rejected.".to_string(),
        );
    }
    
    /// Edit scraping plan
    fn edit_scraping_plan(&mut self) {
        self.add_notification(
            NotificationLevel::Info,
            "Edit Mode".to_string(),
            "Scraping plan editor will be available in a future version.".to_string(),
        );
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct WindowsUI;

#[cfg(not(feature = "ui"))]
impl WindowsUI {
    pub fn new(_app: std::sync::Arc<crate::core::WinScrapeStudio>) -> Self {
        Self
    }
}
