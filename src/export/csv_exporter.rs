use anyhow::Result;
use csv::WriterBuilder;
use tracing::{debug, info};

use crate::config::ExportConfig;
use super::{DataTransformer, InternalExportStats};

/// Export data to CSV format
pub async fn export_csv(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to CSV: {}", data.len(), output_path);
    
    if data.is_empty() {
        return create_empty_csv(output_path).await;
    }
    
    // Flatten JSON data for tabular format
    let flattened_data = DataTransformer::flatten_json(data)?;
    
    // Get all unique column names
    let mut all_columns = std::collections::HashSet::new();
    for row in &flattened_data {
        for key in row.keys() {
            all_columns.insert(key.clone());
        }
    }
    
    let mut columns: Vec<String> = all_columns.into_iter().collect();
    columns.sort();
    
    // Create CSV writer
    let file = std::fs::File::create(output_path)?;
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_writer(file);
    
    // Write headers
    writer.write_record(&columns)?;
    
    // Write data rows
    for row in &flattened_data {
        let record: Vec<String> = columns.iter()
            .map(|col| row.get(col).cloned().unwrap_or_default())
            .collect();
        writer.write_record(&record)?;
    }
    
    writer.flush()?;
    drop(writer);
    
    // Get file stats
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("CSV export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Create empty CSV file with headers only
async fn create_empty_csv(output_path: &str) -> Result<InternalExportStats> {
    let file = std::fs::File::create(output_path)?;
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_writer(file);
    
    // Write empty headers
    writer.write_record(&["no_data"])?;
    writer.flush()?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export with custom CSV configuration
pub async fn export_csv_with_config(
    data: &[serde_json::Value],
    output_path: &str,
    delimiter: u8,
    quote_style: csv::QuoteStyle,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to CSV with custom config: {}", data.len(), output_path);
    
    if data.is_empty() {
        return create_empty_csv(output_path).await;
    }
    
    let flattened_data = DataTransformer::flatten_json(data)?;
    let columns = DataTransformer::get_column_names(data);
    
    let file = std::fs::File::create(output_path)?;
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .delimiter(delimiter)
        .quote_style(quote_style)
        .from_writer(file);
    
    writer.write_record(&columns)?;
    
    for row in &flattened_data {
        let record: Vec<String> = columns.iter()
            .map(|col| row.get(col).cloned().unwrap_or_default())
            .collect();
        writer.write_record(&record)?;
    }
    
    writer.flush()?;
    drop(writer);
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export data to TSV (Tab-Separated Values) format
pub async fn export_tsv(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    export_csv_with_config(
        data,
        output_path,
        b'\t',
        csv::QuoteStyle::Necessary,
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_csv_export() {
        let data = vec![
            json!({"name": "John", "age": 30, "city": "New York"}),
            json!({"name": "Jane", "age": 25, "city": "Los Angeles"}),
        ];
        
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();
        
        let config = ExportConfig {
            default_format: "csv".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
        };
        
        let stats = export_csv(&data, output_path, &config).await.unwrap();
        
        assert!(stats.file_size_bytes > 0);
        
        // Verify file contents
        let contents = std::fs::read_to_string(output_path).unwrap();
        // Columns are sorted alphabetically: age, city, name
        assert!(contents.contains("age,city,name"));
        assert!(contents.contains("30,New York,John"));
        assert!(contents.contains("25,Los Angeles,Jane"));
    }
    
    #[tokio::test]
    async fn test_empty_csv_export() {
        let data: Vec<serde_json::Value> = vec![];
        
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();
        
        let config = ExportConfig {
            default_format: "csv".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
        };
        
        let stats = export_csv(&data, output_path, &config).await.unwrap();
        
        assert!(stats.file_size_bytes > 0);
        
        let contents = std::fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("no_data"));
    }
    
    #[tokio::test]
    async fn test_nested_json_flattening() {
        let data = vec![
            json!({
                "user": {
                    "name": "John",
                    "details": {
                        "age": 30,
                        "location": "NYC"
                    }
                },
                "scores": [85, 92, 78]
            }),
        ];
        
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();
        
        let config = ExportConfig {
            default_format: "csv".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
        };
        
        let stats = export_csv(&data, output_path, &config).await.unwrap();
        
        assert!(stats.file_size_bytes > 0);
        
        let contents = std::fs::read_to_string(output_path).unwrap();
        assert!(contents.contains("user_name"));
        assert!(contents.contains("user_details_age"));
        assert!(contents.contains("scores_0"));
    }
}
