use anyhow::Result;
use tracing::debug;

use crate::config::ExportConfig;
use super::{InternalExportStats, DataTransformer};

/// Export data to a real XLSX (OOXML) workbook.
///
/// Note: this previously wrote a "SpreadsheetML 2003" XML document with a
/// `.xlsx` extension. That format is XML, not the ZIP-based OOXML package
/// modern Excel expects for `.xlsx`, so Excel and other strict readers
/// could refuse to open (or warn about) the resulting file. This now
/// writes an actual `.xlsx` workbook via `rust_xlsxwriter`.
pub async fn export_xlsx(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to XLSX: {}", data.len(), output_path);

    let is_empty = data.is_empty();
    let column_names = if is_empty {
        vec!["no_data".to_string()]
    } else {
        DataTransformer::get_column_names(data)
    };
    let flattened_data = if is_empty {
        Vec::new()
    } else {
        DataTransformer::flatten_json(data)?
    };

    // rust_xlsxwriter's Workbook API is synchronous/blocking (it builds
    // and zips the whole package in memory before writing), so run it on
    // a blocking thread rather than tying up the async runtime.
    let output_path_owned = output_path.to_string();
    tokio::task::spawn_blocking(move || -> Result<()> {
        use rust_xlsxwriter::{Format, Workbook};

        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        let header_format = Format::new().set_bold();

        if is_empty {
            worksheet.write_string_with_format(0, 0, "No Data", &header_format)?;
        } else {
            for (col_idx, column) in column_names.iter().enumerate() {
                worksheet.write_string_with_format(0, col_idx as u16, column.as_str(), &header_format)?;
            }

            for (row_idx, row) in flattened_data.iter().enumerate() {
                let excel_row = (row_idx + 1) as u32;
                for (col_idx, column) in column_names.iter().enumerate() {
                    let excel_col = col_idx as u16;
                    let value = row.get(column).map(String::as_str).unwrap_or("");

                    // Write numeric-looking values as real numbers so
                    // Excel treats them as numbers (sortable, summable)
                    // rather than text.
                    if !value.is_empty() {
                        if let Ok(num) = value.parse::<f64>() {
                            worksheet.write_number(excel_row, excel_col, num)?;
                            continue;
                        }
                    }
                    worksheet.write_string(excel_row, excel_col, value)?;
                }
            }

            // Auto-size columns so the exported sheet is readable without
            // the user manually resizing every column.
            worksheet.autofit();
        }

        workbook.save(&output_path_owned)?;
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("XLSX export task panicked: {}", e))??;

    let file_size = tokio::fs::metadata(output_path).await?.len();

    Ok(InternalExportStats {
        file_size_bytes: file_size,
        compression_ratio: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;

    fn test_config() -> ExportConfig {
        ExportConfig {
            default_format: "xlsx".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
            include_metadata: true,
        }
    }

    #[tokio::test]
    async fn test_xlsx_export() {
        let data = vec![
            json!({"name": "John", "age": 30}),
            json!({"name": "Jane", "age": 25}),
        ];

        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();

        let stats = export_xlsx(&data, output_path, &test_config()).await.unwrap();

        assert!(stats.file_size_bytes > 0);

        // A real .xlsx is a ZIP package; its file signature starts with "PK".
        let contents = std::fs::read(output_path).unwrap();
        assert_eq!(&contents[0..2], b"PK");
    }

    #[tokio::test]
    async fn test_empty_xlsx_export() {
        let data: Vec<serde_json::Value> = vec![];

        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();

        let stats = export_xlsx(&data, output_path, &test_config()).await.unwrap();

        assert!(stats.file_size_bytes > 0);
        let contents = std::fs::read(output_path).unwrap();
        assert_eq!(&contents[0..2], b"PK");
    }
}
