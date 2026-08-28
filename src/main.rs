use anyhow::Result;
use tracing::{info, error, warn};

#[cfg(feature = "ui")]
use eframe::egui;

mod core;
mod ui;
mod config;
mod storage;
mod scraper;
mod llm;
mod dsl;
mod export;
mod security;
mod utils;
mod error;
mod logging;
mod i18n;
#[cfg(feature = "api")]
mod api;

#[cfg(not(feature = "ui"))]
use crate::core::WinScrapeStudio;
#[cfg(not(feature = "ui"))]
use crate::config::AppConfig;
use crate::logging::{LoggingConfig, LogContext, RequestIdGenerator};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging()?;
    
    info!("Starting WinScrape Studio v{}", env!("CARGO_PKG_VERSION"));
    
    // Start the UI
    #[cfg(feature = "ui")]
    {
        // WindowsApp loads its own configuration and initializes the core
        // application internally (see WindowsApp::initialize), so we don't
        // create a separate WinScrapeStudio instance here. Doing so
        // previously caused the app to be initialized twice (duplicate
        // storage/config setup) with the first instance never used.
        info!("Starting Windows GUI interface");

        let mut windows_app = crate::ui::windows_app::WindowsApp::new();
        if let Err(e) = windows_app.initialize().await {
            error!("Failed to initialize GUI: {}", e);
            return Err(anyhow::anyhow!("GUI initialization failed: {}", e));
        }
        if let Err(e) = windows_app.run() {
            error!("Failed to run GUI: {}", e);
            return Err(anyhow::anyhow!("GUI execution failed: {}", e));
        }
    }

    #[cfg(not(feature = "ui"))]
    {
        warn!("GUI feature not enabled, running in headless mode");

        // Load configuration
        let config = AppConfig::load().await?;
        info!("Configuration loaded successfully");

        // Initialize the core application
        let app = WinScrapeStudio::new(config).await?;
        info!("Core application initialized");

        app.run_headless().await?;
    }
    
    info!("WinScrape Studio shutting down");
    Ok(())
}

fn init_logging() -> Result<()> {
    let log_dir = directories::ProjectDirs::from("com", "winscrape", "studio")
        .map(|dirs| dirs.data_dir().join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));
    
    let logging_config = LoggingConfig {
        level: std::env::var("WSS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        file_enabled: true,
        console_enabled: true,
        json_format: false,
        max_file_size_mb: 10,
        max_files: 5,
        log_directory: log_dir,
        include_spans: true,
        include_targets: true,
        structured_fields: true,
    };
    
    crate::logging::init_logging(&logging_config)?;
    
    // Log startup information with context
    let context = LogContext::new("main", "startup")
        .with_request_id(&RequestIdGenerator::generate())
        .with_string_field("version", env!("CARGO_PKG_VERSION"));
    
    crate::log_info!(context, "WinScrape Studio starting up");
    
    Ok(())
}
