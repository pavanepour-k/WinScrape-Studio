use anyhow::Result;
use tracing::debug;

use crate::config::ExportConfig;
use super::InternalExportStats;

/// Export data to a real Apache Parquet file.
///
/// Note: this previously wrote plain newline-delimited JSON text with a
/// `.parquet` extension, and reported a fake `compression_ratio: 0.7` even
/// though nothing was compressed. Real Parquet readers (pandas, DuckDB,
/// Spark, ...) could not open that file at all. This now infers an Arrow
/// schema from the JSON records and writes an actual Snappy-compressed
/// Parquet file via the `parquet`/`arrow-json` crates.
///
/// CAVEAT: this is the one export path in this codebase we were not able
/// to compile-check (see repo notes on the sandbox's Rust toolchain being
/// too old for this crate's edition2024 dependency chain). The general
/// approach (infer_json_schema -> ReaderBuilder -> ArrowWriter) is the
/// standard way to do this with these crates, but double-check this file
/// first if `cargo build --features parquet-export` (or your normal
/// build) reports errors here.
pub async fn export_parquet(
    data: &[serde_json::Value],
    output_path: &str,
    _config: &ExportConfig,
) -> Result<InternalExportStats> {
    debug!("Exporting {} records to Parquet: {}", data.len(), output_path);

    let records: Vec<serde_json::Value> = if data.is_empty() {
        // Arrow schema inference needs at least one record to build a
        // schema from; emit a single placeholder row for empty input so
        // we still produce a valid (if trivial) Parquet file rather than
        // an empty/corrupt one.
        vec![serde_json::json!({"no_data": true})]
    } else {
        data.to_vec()
    };

    let output_path_owned = output_path.to_string();

    // Schema inference + Arrow/Parquet writing is all synchronous, CPU
    // bound work, so run it on a blocking thread rather than tying up the
    // async runtime.
    tokio::task::spawn_blocking(move || -> Result<()> {
        use std::io::Cursor;
        use std::sync::Arc;
        use parquet::arrow::ArrowWriter;
        use parquet::file::properties::WriterProperties;
        use parquet::basic::Compression;

        // Serialize as newline-delimited JSON, which is what arrow-json's
        // line-delimited reader expects.
        let mut ndjson = String::new();
        for record in &records {
            ndjson.push_str(&serde_json::to_string(record)?);
            ndjson.push('\n');
        }
        let ndjson_bytes = ndjson.into_bytes();

        // Infer an Arrow schema from the JSON records.
        let (schema, _) = arrow_json::reader::infer_json_schema_from_seekable(
            &mut Cursor::new(ndjson_bytes.clone()),
            None,
        )?;
        let schema = Arc::new(schema);

        // Decode the JSON records into Arrow RecordBatches using the
        // inferred schema.
        let json_reader = arrow_json::ReaderBuilder::new(schema.clone())
            .build(Cursor::new(ndjson_bytes))?;

        let file = std::fs::File::create(&output_path_owned)?;
        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();
        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;

        for batch_result in json_reader {
            let batch = batch_result?;
            writer.write(&batch)?;
        }

        writer.close()?;
        Ok(())
    })
    .await
    .map_err(|e| anyhow::anyhow!("Parquet export task panicked: {}", e))??;

    let file_size = tokio::fs::metadata(output_path).await?.len();

    Ok(InternalExportStats {
        file_size_bytes: file_size,
        // Snappy is a fast, moderate-ratio codec; unlike the previous
        // implementation this file is genuinely Snappy-compressed, but we
        // don't have an uncompressed baseline handy to report a precise
        // ratio, so we leave it unset rather than report another
        // made-up number.
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
            default_format: "parquet".to_string(),
            max_file_size_mb: 100,
            compression_enabled: false,
            output_directory: std::path::PathBuf::from("/tmp"),
            include_metadata: true,
        }
    }

    #[tokio::test]
    async fn test_parquet_export() {
        let data = vec![
            json!({"name": "John", "age": 30}),
            json!({"name": "Jane", "age": 25}),
        ];

        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();

        let stats = export_parquet(&data, output_path, &test_config()).await.unwrap();

        assert!(stats.file_size_bytes > 0);

        // Real Parquet files start and end with the 4-byte magic "PAR1".
        let contents = std::fs::read(output_path).unwrap();
        assert_eq!(&contents[0..4], b"PAR1");
        assert_eq!(&contents[contents.len() - 4..], b"PAR1");
    }

    #[tokio::test]
    async fn test_empty_parquet_export() {
        let data: Vec<serde_json::Value> = vec![];

        let temp_file = NamedTempFile::new().unwrap();
        let output_path = temp_file.path().to_str().unwrap();

        let stats = export_parquet(&data, output_path, &test_config()).await.unwrap();

        assert!(stats.file_size_bytes > 0);
        let contents = std::fs::read(output_path).unwrap();
        assert_eq!(&contents[0..4], b"PAR1");
    }
}
