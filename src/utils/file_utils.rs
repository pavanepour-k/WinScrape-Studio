use anyhow::Result;
use std::path::Path;

/// File utility functions
pub struct FileUtils;

impl FileUtils {
    /// Check if file exists and is readable
    pub fn is_readable<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        path.exists() && path.is_file()
    }
    
    /// Get file size in bytes
    pub async fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
        let metadata = tokio::fs::metadata(path).await?;
        Ok(metadata.len())
    }
    
    /// Create directory if it doesn't exist
    pub async fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }
}
