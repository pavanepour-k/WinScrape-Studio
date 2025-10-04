use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, error};

pub mod csv_exporter;
pub mod json_exporter;
pub mod xlsx_exporter;
pub mod parquet_exporter;

use crate::config::ExportConfig;

/// Export manager for handling different output formats
pub struct ExportManager {
    config: ExportConfig,
}

/// Export format enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
    Xlsx,
    Parquet,
}

impl std::str::FromStr for ExportFormat {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            "xlsx" => Ok(ExportFormat::Xlsx),
            "parquet" => Ok(ExportFormat::Parquet),
            _ => Err(anyhow::anyhow!("Invalid export format: {}", s)),
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Csv => write!(f, "csv"),
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Xlsx => write!(f, "xlsx"),
            ExportFormat::Parquet => write!(f, "parquet"),
        }
    }
}

/// Export statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStats {
    pub format: ExportFormat,
    pub file_path: String,
    pub record_count: usize,
    pub file_size_bytes: u64,
    pub export_duration_ms: u64,
    pub compression_ratio: Option<f64>,
}

impl ExportManager {
    /// Create new export manager
    pub fn new(config: &ExportConfig) -> Result<Self> {
        // Ensure output directory exists
        std::fs::create_dir_all(&config.output_directory)?;
        
        Ok(Self {
            config: config.clone(),
        })
    }
    
    /// Export data to specified format
    pub async fn export(
        &self,
        data: &[serde_json::Value],
        output_path: &str,
        format: ExportFormat,
    ) -> Result<ExportStats> {
        info!("Exporting {} records to {} as {}", data.len(), output_path, format);
        
        let start_time = std::time::Instant::now();
        
        // Validate data size
        self.validate_export_size(data)?;
        
        // Perform export based on format
        let stats = match format {
            ExportFormat::Csv => {
                csv_exporter::export_csv(data, output_path, &self.config).await?
            }
            ExportFormat::Json => {
                json_exporter::export_json(data, output_path, &self.config).await?
            }
            ExportFormat::Xlsx => {
                xlsx_exporter::export_xlsx(data, output_path, &self.config).await?
            }
            ExportFormat::Parquet => {
                parquet_exporter::export_parquet(data, output_path, &self.config).await?
            }
        };
        
        let export_duration = start_time.elapsed().as_millis() as u64;
        
        let final_stats = ExportStats {
            format,
            file_path: output_path.to_string(),
            record_count: data.len(),
            file_size_bytes: stats.file_size_bytes,
            export_duration_ms: export_duration,
            compression_ratio: stats.compression_ratio,
        };
        
        info!("Export completed: {} records in {}ms, file size: {} bytes", 
              final_stats.record_count, 
              final_stats.export_duration_ms,
              final_stats.file_size_bytes);
        
        Ok(final_stats)
    }
    
    /// Export to multiple formats
    pub async fn export_multiple(
        &self,
        data: &[serde_json::Value],
        base_path: &str,
        formats: &[ExportFormat],
    ) -> Result<Vec<ExportStats>> {
        let mut all_stats = Vec::new();
        
        for format in formats {
            let file_extension = format.to_string();
            let output_path = format!("{}.{}", base_path, file_extension);
            
            match self.export(data, &output_path, format.clone()).await {
                Ok(stats) => all_stats.push(stats),
                Err(e) => {
                    error!("Failed to export to {}: {}", format, e);
                    // Continue with other formats
                }
            }
        }
        
        Ok(all_stats)
    }
    
    /// Validate export data size
    fn validate_export_size(&self, data: &[serde_json::Value]) -> Result<()> {
        // Estimate memory usage
        let estimated_size = data.len() * 1024; // Rough estimate: 1KB per record
        let max_size = self.config.max_file_size_mb * 1024 * 1024;
        
        if estimated_size > max_size {
            return Err(anyhow::anyhow!(
                "Export data too large: estimated {} bytes, max allowed {} bytes",
                estimated_size,
                max_size
            ));
        }
        
        Ok(())
    }
    
    /// Get supported formats
    pub fn get_supported_formats() -> Vec<ExportFormat> {
        vec![
            ExportFormat::Csv,
            ExportFormat::Json,
            ExportFormat::Xlsx,
            ExportFormat::Parquet,
        ]
    }
    
    /// Get file extension for format
    pub fn get_file_extension(format: &ExportFormat) -> &'static str {
        match format {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Parquet => "parquet",
        }
    }
    
    /// Generate default filename
    pub fn generate_filename(job_id: &str, format: &ExportFormat) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let extension = Self::get_file_extension(format);
        format!("winscrape_{}_{}.{}", job_id, timestamp, extension)
    }
    
    /// Compress file if enabled
    pub async fn compress_file(&self, file_path: &str) -> Result<String> {
        if !self.config.compression_enabled {
            return Ok(file_path.to_string());
        }
        
        let compressed_path = format!("{}.gz", file_path);
        
        // Read original file
        let data = tokio::fs::read(file_path).await?;
        
        // Compress data
        let compressed_data = self.compress_data(&data)?;
        
        // Write compressed file
        tokio::fs::write(&compressed_path, compressed_data).await?;
        
        // Remove original file
        tokio::fs::remove_file(file_path).await?;
        
        info!("File compressed: {} -> {}", file_path, compressed_path);
        Ok(compressed_path)
    }
    
    /// Compress data using gzip
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }
    
    /// Get export statistics for a file
    pub async fn get_file_stats(&self, file_path: &str) -> Result<FileStats> {
        let metadata = tokio::fs::metadata(file_path).await?;
        
        Ok(FileStats {
            file_path: file_path.to_string(),
            file_size_bytes: metadata.len(),
            created_at: metadata.created()
                .map(|t| chrono::DateTime::from(t))
                .unwrap_or_else(|_| chrono::Utc::now()),
            modified_at: metadata.modified()
                .map(|t| chrono::DateTime::from(t))
                .unwrap_or_else(|_| chrono::Utc::now()),
        })
    }
}

/// File statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStats {
    pub file_path: String,
    pub file_size_bytes: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

/// Internal export result
#[derive(Debug)]
pub(crate) struct InternalExportStats {
    pub file_size_bytes: u64,
    pub compression_ratio: Option<f64>,
}

/// Data transformation utilities for export
pub struct DataTransformer;

impl DataTransformer {
    /// Flatten nested JSON objects for tabular export
    pub fn flatten_json(data: &[serde_json::Value]) -> Result<Vec<std::collections::HashMap<String, String>>> {
        let mut flattened = Vec::new();
        
        for item in data {
            let mut flat_item = std::collections::HashMap::new();
            Self::flatten_object(item, "", &mut flat_item);
            flattened.push(flat_item);
        }
        
        Ok(flattened)
    }
    
    /// Recursively flatten a JSON object
    fn flatten_object(
        value: &serde_json::Value,
        prefix: &str,
        result: &mut std::collections::HashMap<String, String>,
    ) {
        match value {
            serde_json::Value::Object(obj) => {
                for (key, val) in obj {
                    let new_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}_{}", prefix, key)
                    };
                    Self::flatten_object(val, &new_key, result);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_key = format!("{}_{}", prefix, i);
                    Self::flatten_object(val, &new_key, result);
                }
            }
            _ => {
                result.insert(prefix.to_string(), Self::value_to_string(value));
            }
        }
    }
    
    /// Convert JSON value to string
    fn value_to_string(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => String::new(),
            _ => value.to_string(),
        }
    }
    
    /// Get all unique column names from data
    pub fn get_column_names(data: &[serde_json::Value]) -> Vec<String> {
        let mut columns = std::collections::HashSet::new();
        
        for item in data {
            if let serde_json::Value::Object(obj) = item {
                for key in obj.keys() {
                    columns.insert(key.clone());
                }
            }
        }
        
        let mut sorted_columns: Vec<String> = columns.into_iter().collect();
        sorted_columns.sort();
        sorted_columns
    }
    
    /// Normalize data types for consistent export
    pub fn normalize_data(data: &mut [serde_json::Value]) {
        for item in data {
            if let serde_json::Value::Object(obj) = item {
                for (_, value) in obj.iter_mut() {
                    *value = Self::normalize_value(value);
                }
            }
        }
    }
    
    /// Normalize a single value
    fn normalize_value(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                // Try to parse as number if it looks like one
                if let Ok(num) = s.parse::<f64>() {
                    serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0)))
                } else {
                    // Clean up string
                    serde_json::Value::String(s.trim().to_string())
                }
            }
            _ => value.clone(),
        }
    }
}
