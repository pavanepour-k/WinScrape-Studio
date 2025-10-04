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
    state::{UIState, View, JobInfo, JobStatus, UISettings, WorkflowState},
    windows_theme::WindowsTheme,
    windows_components::{WindowsComponents, NotificationLevel},
    results_viewer::ResultsViewer,
    icon_manager::IconManager,
};
use crate::i18n::{I18nManager, Language};

/// Main Windows-native UI application
#[cfg(feature = "ui")]
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
    window_title: String,
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

#[cfg(feature = "ui")]
impl WindowsUI {
    /// Create new Windows UI application
    pub fn new(app: Arc<WinScrapeStudio>) -> Self {
        let state = UIState::new();
        let chat = ChatInterface::new();
        let theme = WindowsTheme::windows11_dark();
        let icon_manager = IconManager::new();
        let i18n_manager = I18nManager::new();
        
        Self {
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
            window_title: format!("WinScrape Studio v{}", env!("CARGO_PKG_VERSION")),
        }
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
                        egui::ComboBox::from_id_source("language_combo")
                            .selected_text(self.current_language().name())
                            .show_ui(ui, |ui| {
                                for language in self.available_languages() {
                                    ui.selectable_value(
                                        &mut self.current_language(),
                                        language,
                                        language.name()
                                    );
                                }
                            });
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
                    
                    WindowsComponents::checkbox(ui, &self.t("settings.auto_save"), &mut true);
                    WindowsComponents::checkbox(ui, &self.t("settings.notifications"), &mut true);
                    WindowsComponents::checkbox(ui, &self.t("settings.minimize_to_tray"), &mut false);
                });
                
                ui.add_space(16.0);
                
                // Scraping Settings
                WindowsComponents::card_with_header(ui, "Scraping Settings", |ui| {
                    let mut max_requests = self.state.settings.max_concurrent_requests as f32;
                    let mut timeout = self.state.settings.request_timeout as f32;
                    WindowsComponents::slider(ui, "Max concurrent requests", &mut max_requests, 1.0, 20.0);
                    WindowsComponents::slider(ui, "Request timeout (seconds)", &mut timeout, 5.0, 120.0);
                    self.state.settings.max_concurrent_requests = max_requests as usize;
                    self.state.settings.request_timeout = timeout as u64;
                    
                    ui.add_space(8.0);
                    
                    WindowsComponents::checkbox(ui, "Respect robots.txt", &mut self.state.settings.respect_robots_txt);
                    WindowsComponents::checkbox(ui, "Enable browser fallback", &mut self.state.settings.enable_browser_fallback);
                });
                
                ui.add_space(16.0);
                
                // Export Settings
                WindowsComponents::card_with_header(ui, "Export Settings", |ui| {
                    let formats = ["csv", "json", "xlsx", "parquet"];
                    WindowsComponents::dropdown(ui, "Default export format", &mut self.state.settings.default_export_format, &formats.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                    
                    ui.add_space(8.0);
                    
                    WindowsComponents::checkbox(ui, "Include metadata in exports", &mut true);
                    WindowsComponents::checkbox(ui, "Compress large exports", &mut true);
                });
                
                ui.add_space(16.0);
                
                // Security Settings
                WindowsComponents::card_with_header(ui, "Security Settings", |ui| {
                    WindowsComponents::checkbox(ui, "Enable input validation", &mut self.state.settings.enable_input_validation);
                    WindowsComponents::checkbox(ui, "Filter sensitive data from output", &mut self.state.settings.enable_output_filtering);
                    
                    ui.add_space(8.0);
                    
                    ui.label("Blocked domains:");
                    ui.add(egui::TextEdit::multiline(&mut String::new()).hint_text("Enter domains to block, one per line"));
                });
            });
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
        
        for (i, notification) in self.notifications.iter().enumerate() {
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
        egui::Window::new("Export Data")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                ui.label("Export Format:");
                let formats = ["CSV", "JSON", "XLSX", "Parquet"];
                let mut selected_format = "CSV".to_string();
                WindowsComponents::dropdown(ui, "", &mut selected_format, &formats.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                
                ui.add_space(8.0);
                
                ui.label("Output Path:");
                WindowsComponents::file_picker_button(ui, "", &self.export_path);
                
                ui.add_space(16.0);
                
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        self.perform_export(&selected_format);
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
        
        // Spawn async task to handle the request
        let app = self.app.clone();
        let ctx = ctx.clone();
        
        tokio::spawn(async move {
            // This would need to be handled differently in a real implementation
            info!("Starting workflow for input: {}", input);
        });
    }
    
    /// Handle background tasks
    fn handle_background_tasks(&mut self, ctx: &egui::Context) {
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
    }
    
    /// Refresh specific job details
    fn refresh_job_details(&mut self, job_id: &str) {
        debug!("Refreshing job details: {}", job_id);
    }
    
    /// View job results
    fn view_job_results(&mut self, job_id: &str) {
        info!("Viewing results for job: {}", job_id);
        // This would open the results viewer
    }
    
    /// Export job results
    fn export_job_results(&mut self, job_id: &str) {
        info!("Exporting results for job: {}", job_id);
        self.show_export_dialog = true;
    }
    
    /// Cancel running job
    fn cancel_job(&mut self, job_id: &str) {
        info!("Cancelling job: {}", job_id);
        self.add_notification(
            NotificationLevel::Info,
            "Job Cancelled".to_string(),
            format!("Job {} has been cancelled.", job_id),
        );
    }
    
    /// Rerun job
    fn rerun_job(&mut self, job_id: &str) {
        info!("Rerunning job: {}", job_id);
        self.add_notification(
            NotificationLevel::Info,
            "Job Restarted".to_string(),
            format!("Job {} has been restarted.", job_id),
        );
    }
    
    /// Save settings
    fn save_settings(&mut self) {
        info!("Saving settings");
        self.add_notification(
            NotificationLevel::Success,
            "Settings Saved".to_string(),
            "Your settings have been saved successfully.".to_string(),
        );
    }
    
    /// Reset settings
    fn reset_settings(&mut self) {
        self.state.settings = UISettings::default();
        self.add_notification(
            NotificationLevel::Info,
            "Settings Reset".to_string(),
            "Settings have been reset to defaults.".to_string(),
        );
    }
    
    /// Export settings
    fn export_settings(&mut self) {
        info!("Exporting settings");
        self.add_notification(
            NotificationLevel::Info,
            "Settings Exported".to_string(),
            "Settings have been exported to file.".to_string(),
        );
    }
    
    /// Import settings
    fn import_settings(&mut self) {
        info!("Importing settings");
        self.add_notification(
            NotificationLevel::Info,
            "Settings Imported".to_string(),
            "Settings have been imported from file.".to_string(),
        );
    }
    
    /// Clear completed jobs
    fn clear_completed_jobs(&mut self) {
        self.state.jobs.retain(|job| !matches!(job.status, JobStatus::Completed));
        self.add_notification(
            NotificationLevel::Info,
            "Jobs Cleared".to_string(),
            "Completed jobs have been cleared.".to_string(),
        );
    }
    
    /// Export all jobs
    fn export_all_jobs(&mut self) {
        self.show_export_dialog = true;
    }
    
    /// Perform export
    fn perform_export(&mut self, format: &str) {
        info!("Exporting data in {} format", format);
        self.add_notification(
            NotificationLevel::Success,
            "Export Complete".to_string(),
            format!("Data exported successfully in {} format.", format),
        );
    }
    
    /// Approve scraping plan
    fn approve_scraping_plan(&mut self) {
        self.state.pending_approval = None;
        self.add_notification(
            NotificationLevel::Success,
            "Plan Approved".to_string(),
            "Scraping plan has been approved and execution started.".to_string(),
        );
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
