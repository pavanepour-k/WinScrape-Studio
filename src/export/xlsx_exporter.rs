use anyhow::Result;
use tracing::debug;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::config::ExportConfig;
use super::{InternalExportStats, DataTransformer};

/// Export data to XLSX format
/// Note: This creates a simplified Excel XML format
/// For production use, consider using rust_xlsxwriter or similar crate
pub async fn export_xlsx(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to XLSX: {}", data.len(), output_path);
    
    if data.is_empty() {
        return create_empty_xlsx(output_path).await;
    }
    
    // Flatten JSON data for tabular format
    let flattened_data = DataTransformer::flatten_json(data)?;
    let column_names = DataTransformer::get_column_names(data);
    
    let mut file = File::create(output_path).await?;
    
    // Write Excel XML format (simplified)
    file.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n").await?;
    file.write_all(b"<Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\"\n").await?;
    file.write_all(b"          xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\">\n").await?;
    file.write_all(b"  <Worksheet ss:Name=\"Sheet1\">\n").await?;
    file.write_all(b"    <Table>\n").await?;
    
    // Write header row
    file.write_all(b"      <Row>").await?;
    for column in &column_names {
        let escaped_column = html_escape::encode_text(column);
        file.write_all(format!("<Cell><Data ss:Type=\"String\">{}</Data></Cell>", escaped_column).as_bytes()).await?;
    }
    file.write_all(b"</Row>\n").await?;
    
    // Write data rows
    for row in &flattened_data {
        file.write_all(b"      <Row>").await?;
        for column in &column_names {
            let empty_string = String::new();
            let value = row.get(column).unwrap_or(&empty_string);
            let escaped_value = html_escape::encode_text(value);
            file.write_all(format!("<Cell><Data ss:Type=\"String\">{}</Data></Cell>", escaped_value).as_bytes()).await?;
        }
        file.write_all(b"</Row>\n").await?;
    }
    
    file.write_all(b"    </Table>\n").await?;
    file.write_all(b"  </Worksheet>\n").await?;
    file.write_all(b"</Workbook>\n").await?;
    
    file.flush().await?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

async fn create_empty_xlsx(output_path: &str) -> Result<InternalExportStats> {
    let mut file = File::create(output_path).await?;
    
    file.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n").await?;
    file.write_all(b"<Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\"\n").await?;
    file.write_all(b"          xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\">\n").await?;
    file.write_all(b"  <Worksheet ss:Name=\"Sheet1\">\n").await?;
    file.write_all(b"    <Table>\n").await?;
    file.write_all(b"      <Row><Cell><Data ss:Type=\"String\">No Data</Data></Cell></Row>\n").await?;
    file.write_all(b"    </Table>\n").await?;
    file.write_all(b"  </Worksheet>\n").await?;
    file.write_all(b"</Workbook>\n").await?;
    
    file.flush().await?;
    
    let file_size = tokio::fs::metadata(output_path).await?.len();
    
    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}
