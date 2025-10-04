/// Utility functions and helpers
use anyhow::Result;
use std::path::Path;
use tracing::info;

pub mod file_utils;
pub mod string_utils;
pub mod validation_utils;
pub mod crypto_utils;

/// Initialize application directories
pub async fn init_app_directories() -> Result<AppDirectories> {
    let dirs = AppDirectories::new()?;
    dirs.create_all().await?;
    Ok(dirs)
}

/// Application directory structure
#[derive(Debug, Clone)]
pub struct AppDirectories {
    pub data_dir: std::path::PathBuf,
    pub config_dir: std::path::PathBuf,
    pub cache_dir: std::path::PathBuf,
    pub logs_dir: std::path::PathBuf,
    pub exports_dir: std::path::PathBuf,
    pub models_dir: std::path::PathBuf,
}

impl AppDirectories {
    /// Create new app directories
    pub fn new() -> Result<Self> {
        let project_dirs = directories::ProjectDirs::from("com", "winscrape", "studio")
            .ok_or_else(|| anyhow::anyhow!("Failed to determine project directories"))?;
        
        let data_dir = project_dirs.data_dir().to_path_buf();
        let config_dir = project_dirs.config_dir().to_path_buf();
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        
        Ok(Self {
            logs_dir: data_dir.join("logs"),
            exports_dir: data_dir.join("exports"),
            models_dir: data_dir.join("models"),
            data_dir,
            config_dir,
            cache_dir,
        })
    }
    
    /// Create all directories
    pub async fn create_all(&self) -> Result<()> {
        let dirs = [
            &self.data_dir,
            &self.config_dir,
            &self.cache_dir,
            &self.logs_dir,
            &self.exports_dir,
            &self.models_dir,
        ];
        
        for dir in &dirs {
            if !dir.exists() {
                tokio::fs::create_dir_all(dir).await?;
                info!("Created directory: {}", dir.display());
            }
        }
        
        Ok(())
    }
}

/// Format file size in human readable format
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;
    
    if bytes < THRESHOLD {
        return format!("{} B", bytes);
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format duration in human readable format
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    
    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        format!("{}h {}m {}s", hours, minutes, seconds)
    }
}

/// Generate unique identifier
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Get current timestamp as string
pub fn current_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Validate URL format
pub fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// Extract domain from URL
pub fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .host_str()
        .map(|s| s.to_string())
}

/// Truncate string to specified length
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Check if file exists and is readable
pub fn is_file_accessible<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists() && path.as_ref().is_file()
}

/// Get file extension
pub fn get_file_extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Sanitize filename for filesystem
pub fn sanitize_filename(filename: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*', '/', '\\'];
    let mut sanitized = filename.to_string();
    
    for &ch in &invalid_chars {
        sanitized = sanitized.replace(ch, "_");
    }
    
    // Limit length
    if sanitized.len() > 255 {
        sanitized.truncate(252);
        sanitized.push_str("...");
    }
    
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(std::time::Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(std::time::Duration::from_secs(3661)), "1h 1m 1s");
    }
    
    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path"), Some("example.com".to_string()));
        assert_eq!(extract_domain("http://sub.example.com:8080"), Some("sub.example.com".to_string()));
        assert_eq!(extract_domain("invalid-url"), None);
    }
    
    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 8), "hi");
    }
    
    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("file<name>.txt"), "file_name_.txt");
        assert_eq!(sanitize_filename("normal_file.txt"), "normal_file.txt");
        
        let long_name = "a".repeat(300);
        let sanitized = sanitize_filename(&long_name);
        assert!(sanitized.len() <= 255);
        assert!(sanitized.ends_with("..."));
    }
}
