//! WinScrape Studio - A powerful web scraping application built in Rust
//! 
//! This library provides the core functionality for WinScrape Studio, including:
//! - DSL-based scraping plan definition
//! - Multi-layered scraping engine (HTTP + Browser)
//! - Security and validation framework
//! - Export capabilities
//! - Job management and orchestration

pub mod core;
pub mod config;
pub mod storage;
pub mod scraper;
pub mod llm;
pub mod dsl;
pub mod export;
pub mod security;
pub mod utils;
pub mod error;
pub mod logging;
pub mod performance;
pub mod i18n;

#[cfg(feature = "ui")]
pub mod ui;

#[cfg(feature = "api")]
pub mod api;

// Re-export main types for convenience
pub use crate::core::WinScrapeStudio;
pub use crate::config::AppConfig;
pub use crate::dsl::ScrapePlan;
pub use crate::error::ContextualError;
