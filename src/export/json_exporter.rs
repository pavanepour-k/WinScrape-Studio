use anyhow::Result;
use serde_json::{Value, to_writer_pretty, to_vec_pretty};
use std::fs::File;
use tracing::{debug, info};

use crate::config::ExportConfig;
use super::InternalExportStats;

/// Export data to JSON format
pub async fn export_json(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to JSON: {}", data.len(), output_path);
    
    // Create output file
    let file = File::create(output_path)?;
    
    // Write JSON data with pretty formatting
    to_writer_pretty(file, data)?;
    
    // Get file stats
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("JSON export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export data to JSONL (JSON Lines) format
pub async fn export_jsonl(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to JSONL: {}", data.len(), output_path);
    
    let mut output = String::new();
    
    for item in data {
        let line = serde_json::to_string(item)?;
        output.push_str(&line);
        output.push('\n');
    }
    
    tokio::fs::write(output_path, output).await?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("JSONL export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export data to compact JSON format (no pretty printing)
pub async fn export_json_compact(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to compact JSON: {}", data.len(), output_path);
    
    let json_string = serde_json::to_string(data)?;
    tokio::fs::write(output_path, json_string).await?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("Compact JSON export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export data with metadata wrapper
pub async fn export_json_with_metadata(
    data: &[serde_json::Value],
    output_path: &str,
    metadata: &std::collections::HashMap<String, Value>,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to JSON with metadata: {}", data.len(), output_path);
    
    let export_data = serde_json::json!({
        "metadata": metadata,
        "data": data,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "record_count": data.len()
    });
    
    let file = File::create(output_path)?;
    to_writer_pretty(file, &export_data)?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("JSON with metadata export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export data in streaming fashion for large datasets
pub async fn export_json_streaming(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to JSON (streaming): {}", data.len(), output_path);
    
    use tokio::io::AsyncWriteExt;
    
    let mut file = tokio::fs::File::create(output_path).await?;
    
    // Write opening bracket
    file.write_all(b"[\n").await?;
    
    // Write items
    for (i, item) in data.iter().enumerate() {
        let json_bytes = to_vec_pretty(item)?;
        file.write_all(&json_bytes).await?;
        
        // Add comma if not last item
        if i < data.len() - 1 {
            file.write_all(b",\n").await?;
        } else {
            file.write_all(b"\n").await?;
        }
    }
    
    // Write closing bracket
    file.write_all(b"]").await?;
    file.flush().await?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("Streaming JSON export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Export data with schema validation
pub async fn export_json_with_schema(
    data: &[serde_json::Value],
    output_path: &str,
    schema: &serde_json::Value,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to JSON with schema validation: {}", data.len(), output_path);
    
    // Validate data against schema (simplified validation)
    for (i, item) in data.iter().enumerate() {
        if let Err(e) = validate_against_schema(item, schema) {
            return Err(anyhow::anyhow!("Record {} failed schema validation: {}", i, e));
        }
    }
    
    let export_data = serde_json::json!({
        "schema": schema,
        "data": data,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "record_count": data.len(),
        "schema_validated": true
    });
    
    let file = File::create(output_path)?;
    to_writer_pretty(file, &export_data)?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    info!("JSON with schema export completed: {} records, {} bytes", data.len(), file_size);
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

/// Simple schema validation (basic type checking)
fn validate_against_schema(data: &Value, schema: &Value) -> Result<()> {
    // This is a simplified schema validation
    // In production, you might want to use a proper JSON Schema validator
    
    if let Some(schema_obj) = schema.as_object() {
        if let Some(properties) = schema_obj.get("properties") {
            if let Some(props) = properties.as_object() {
                if let Some(data_obj) = data.as_object() {
                    for (key, prop_schema) in props {
                        if let Some(data_value) = data_obj.get(key) {
                            validate_value_type(data_value, prop_schema)?;
                        } else if prop_schema.get("required").and_then(|v| v.as_bool()).unwrap_or(false) {
                            return Err(anyhow::anyhow!("Required field '{}' is missing", key));
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Validate value type against schema
fn validate_value_type(value: &Value, schema: &Value) -> Result<()> {
    if let Some(expected_type) = schema.get("type").and_then(|v| v.as_str()) {
        let actual_type = match value {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Null => "null",
        };
        
        if actual_type != expected_type {
            return Err(anyhow::anyhow!(
                "Type mismatch: expected '{}', got '{}'", 
                expected_type, 
                actual_type
            ));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_json_export() {
        let data = vec![
            json!({"name": "John", "age": 30}),
            json!({"name": "Jane", "age": 25}),
        ];
        
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();
        
        let config = ExportConfig {
            default_format: "json".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
        };
        
        let stats = export_json(&data, output_path, &config).await.unwrap();
        
        assert!(stats.file_size_bytes > 0);
        
        // Verify file contents
        let contents = std::fs::read_to_string(output_path).unwrap();
        let parsed: Vec<Value> = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.len(), 2);
    }
    
    #[tokio::test]
    async fn test_jsonl_export() {
        let data = vec![
            json!({"name": "John", "age": 30}),
            json!({"name": "Jane", "age": 25}),
        ];
        
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();
        
        let config = ExportConfig {
            default_format: "json".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
        };
        
        let stats = export_jsonl(&data, output_path, &config).await.unwrap();
        
        assert!(stats.file_size_bytes > 0);
        
        // Verify file contents
        let contents = std::fs::read_to_string(output_path).unwrap();
        let lines: Vec<&str> = contents.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
        
        // Each line should be valid JSON
        for line in lines {
            let _: Value = serde_json::from_str(line).unwrap();
        }
    }
    
    #[tokio::test]
    async fn test_json_with_metadata() {
        let data = vec![
            json!({"name": "John", "age": 30}),
        ];
        
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("source".to_string(), json!("test"));
        metadata.insert("version".to_string(), json!("1.0"));
        
        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();
        
        let config = ExportConfig {
            default_format: "json".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
        };
        
        let stats = export_json_with_metadata(&data, output_path, &metadata, &config).await.unwrap();
        
        assert!(stats.file_size_bytes > 0);
        
        // Verify file contents
        let contents = std::fs::read_to_string(output_path).unwrap();
        let parsed: Value = serde_json::from_str(&contents).unwrap();
        
        assert!(parsed.get("metadata").is_some());
        assert!(parsed.get("data").is_some());
        assert!(parsed.get("exported_at").is_some());
        assert_eq!(parsed.get("record_count").unwrap().as_u64().unwrap(), 1);
    }
}
