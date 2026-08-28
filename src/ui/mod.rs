//! UI module tree for WinScrape Studio.
//!
//! The active GUI implementation lives in `windows_app` / `windows_ui`
//! (wired up from `main.rs`). This module just declares the submodules
//! shared across the UI layer.
//!
//! Note: an earlier, unused parallel UI implementation (`WinScrapeUI` /
//! `theme` / `components`) used to live in this file. It was never wired
//! into `main.rs` or anything else, duplicated `windows_ui::WindowsUI`,
//! and contained the same "chat input doesn't actually run the workflow"
//! stub that was fixed in `windows_ui.rs`. It was removed as dead code.

#[cfg(feature = "ui")]
pub mod chat;
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
