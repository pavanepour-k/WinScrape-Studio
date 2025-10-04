#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use std::sync::Arc;
#[cfg(feature = "ui")]
use tracing::{info, error, debug, warn};

#[cfg(feature = "ui")]
use crate::core::WinScrapeStudio;
#[cfg(feature = "ui")]
use super::{
    windows_ui::WindowsUI,
    windows_launcher::WindowsLauncher,
    icon_manager::IconManager,
};
use crate::i18n::{I18nManager, Language};

/// Windows application wrapper with proper initialization and error handling
#[cfg(feature = "ui")]
pub struct WindowsApp {
    ui: Option<WindowsUI>,
    launcher: WindowsLauncher,
    icon_manager: IconManager,
    i18n_manager: I18nManager,
    initialized: bool,
    error_message: Option<String>,
}

#[cfg(feature = "ui")]
impl WindowsApp {
    /// Create new Windows application
    pub fn new() -> Self {
        Self {
            ui: None,
            launcher: WindowsLauncher,
            icon_manager: IconManager::new(),
            i18n_manager: I18nManager::new(),
            initialized: false,
            error_message: None,
        }
    }
    
    /// Initialize the application
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing WinScrape Studio Windows application...");
        
        // Check system requirements
        if let Err(e) = WindowsLauncher::check_system_requirements() {
            self.error_message = Some(format!("System requirements not met: {}", e));
            return Err(e.into());
        }
        
        // Load configuration
        let config = crate::config::AppConfig::load().await?;
        info!("Configuration loaded successfully");
        
        // Initialize the core application
        let app = WinScrapeStudio::new(config).await?;
        info!("Core application initialized");
        
        // Create UI
        let app_arc = Arc::new(app);
        self.ui = Some(WindowsUI::new(app_arc));
        
        self.initialized = true;
        info!("Windows application initialized successfully");
        
        Ok(())
    }
    
    /// Run the application
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Err("Application not initialized".into());
        }
        
        if let Some(error) = &self.error_message {
            WindowsLauncher::show_error_dialog("Initialization Error", error);
            return Err(error.clone().into());
        }
        
        info!("Starting Windows GUI application...");
        
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_min_inner_size([800.0, 600.0])
                .with_decorations(true)
                .with_transparent(false)
                .with_fullscreen(false)
                .with_maximized(false)
                .with_resizable(true)
                .with_icon(self.icon_manager.get_current_icon().unwrap_or_else(|| self.create_simple_icon().unwrap())),
            ..Default::default()
        };
        
        let ui = self.ui.take().ok_or("UI not initialized")?;
        
        eframe::run_native(
            "WinScrape Studio",
            native_options,
            Box::new(|_cc| Box::new(ui)),
        ).map_err(|e| anyhow::anyhow!("Failed to start GUI: {}", e))?;
        
        Ok(())
    }
    
    /// Load application icon
    fn load_icon(&self) -> Option<egui::IconData> {
        // Try to load icon from file
        if let Ok(icon_data) = std::fs::read("icon.ico") {
            if let Ok(image) = image::load_from_memory(&icon_data) {
                let rgba = image.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                return Some(egui::IconData {
                    rgba: rgba.into_raw(),
                    width: size[0] as u32,
                    height: size[1] as u32,
                });
            }
        }
        
        // Create a simple icon programmatically
        self.create_simple_icon()
    }
    
    /// Create a simple icon programmatically
    fn create_simple_icon(&self) -> Option<egui::IconData> {
        // Create a simple 32x32 icon with a spider web pattern
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // Create a simple spider web pattern
                let r = if distance < 2.0 { 255 } else if distance < 8.0 { 200 } else if distance < 15.0 { 150 } else { 100 };
                let g = if distance < 2.0 { 100 } else if distance < 8.0 { 150 } else if distance < 15.0 { 200 } else { 255 };
                let b = if distance < 2.0 { 100 } else if distance < 8.0 { 100 } else if distance < 15.0 { 100 } else { 100 };
                let a = if distance < 16.0 { 255 } else { 0 };
                
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(a);
            }
        }
        
        Some(egui::IconData {
            rgba,
            width: size as u32,
            height: size as u32,
        })
    }
    
    /// Show about dialog
    pub fn show_about(&self) {
        let about_text = format!(
            "{}\nVersion: {}\n\n{}\n\nWebsite: {}\nSupport: {}",
            WindowsLauncher::get_name(),
            WindowsLauncher::get_version(),
            WindowsLauncher::get_description(),
            WindowsLauncher::get_website(),
            WindowsLauncher::get_support_email()
        );
        
        WindowsLauncher::show_info_dialog("About WinScrape Studio", &about_text);
    }
    
    /// Check for updates
    pub async fn check_for_updates(&self) {
        match WindowsLauncher::check_for_updates() {
            Ok(Some(update_url)) => {
                WindowsLauncher::show_info_dialog(
                    "Update Available",
                    "A new version is available. Would you like to download it?",
                );
                
                if let Err(e) = WindowsLauncher::install_update(&update_url) {
                    error!("Failed to install update: {}", e);
                    WindowsLauncher::show_error_dialog(
                        "Update Error",
                        &format!("Failed to install update: {}", e),
                    );
                }
            }
            Ok(None) => {
                WindowsLauncher::show_info_dialog(
                    "No Updates",
                    "You are running the latest version of WinScrape Studio.",
                );
            }
            Err(e) => {
                error!("Failed to check for updates: {}", e);
                WindowsLauncher::show_error_dialog(
                    "Update Check Error",
                    &format!("Failed to check for updates: {}", e),
                );
            }
        }
    }
    
    /// Create shortcuts
    pub fn create_shortcuts(&self) -> Result<(), Box<dyn std::error::Error>> {
        WindowsLauncher::create_desktop_shortcut()?;
        WindowsLauncher::create_start_menu_shortcut()?;
        WindowsLauncher::register_file_associations()?;
        
        WindowsLauncher::show_info_dialog(
            "Shortcuts Created",
            "Desktop and Start Menu shortcuts have been created successfully.",
        );
        
        Ok(())
    }
    
    /// Remove shortcuts
    pub fn remove_shortcuts(&self) -> Result<(), Box<dyn std::error::Error>> {
        WindowsLauncher::unregister_file_associations()?;
        
        WindowsLauncher::show_info_dialog(
            "Shortcuts Removed",
            "File associations have been removed successfully.",
        );
        
        Ok(())
    }
    
    /// Get application info
    pub fn get_app_info(&self) -> AppInfo {
        AppInfo {
            name: WindowsLauncher::get_name(),
            version: WindowsLauncher::get_version(),
            description: WindowsLauncher::get_description(),
            website: WindowsLauncher::get_website(),
            support_email: WindowsLauncher::get_support_email(),
        }
    }
    
    /// Set application language
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
    
    /// Set custom icon
    pub fn set_custom_icon(&mut self, path: String) -> Result<(), Box<dyn std::error::Error>> {
        self.icon_manager.set_custom_icon(path)?;
        Ok(())
    }
    
    /// Get available icon themes
    pub fn available_icon_themes(&self) -> Vec<super::icon_manager::IconTheme> {
        super::icon_manager::IconTheme::all()
    }
    
    /// Save current icon to file
    pub fn save_current_icon(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.icon_manager.save_current_icon(path)?;
        Ok(())
    }
    
    /// Get translation
    pub fn t(&self, key: &str) -> String {
        self.i18n_manager.t(key)
    }
}

/// Application information
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub website: String,
    pub support_email: String,
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct WindowsApp;

#[cfg(not(feature = "ui"))]
impl WindowsApp {
    pub fn new() -> Self { Self }
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("GUI feature not enabled".into())
    }
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("GUI feature not enabled".into())
    }
    pub fn show_about(&self) {}
    pub async fn check_for_updates(&self) {}
    pub fn create_shortcuts(&self) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    pub fn remove_shortcuts(&self) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    pub fn get_app_info(&self) -> AppInfo {
        AppInfo {
            name: "WinScrape Studio".to_string(),
            version: "0.1.0".to_string(),
            description: "A natural language web scraping tool".to_string(),
            website: "https://github.com/winscrape-studio".to_string(),
            support_email: "pavanepour@outlook.com".to_string(),
        }
    }
    pub fn set_language(&mut self, _language: Language) {}
    pub fn current_language(&self) -> Language { Language::English }
    pub fn available_languages(&self) -> Vec<Language> { vec![Language::English] }
    pub fn set_icon_theme(&mut self, _theme: super::icon_manager::IconTheme) {}
    pub fn current_icon_theme(&self) -> super::icon_manager::IconTheme { super::icon_manager::IconTheme::Default }
    pub fn set_custom_icon(&mut self, _path: String) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    pub fn available_icon_themes(&self) -> Vec<super::icon_manager::IconTheme> { vec![super::icon_manager::IconTheme::Default] }
    pub fn save_current_icon(&self, _path: &str) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    pub fn t(&self, key: &str) -> String { key.to_string() }
}

#[cfg(not(feature = "ui"))]
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub website: String,
    pub support_email: String,
}
