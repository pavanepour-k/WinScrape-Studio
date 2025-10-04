use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_enabled: bool,
    pub console_enabled: bool,
    pub json_format: bool,
    pub max_file_size_mb: usize,
    pub max_files: usize,
    pub log_directory: PathBuf,
    pub include_spans: bool,
    pub include_targets: bool,
    pub structured_fields: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_enabled: true,
            console_enabled: true,
            json_format: false,
            max_file_size_mb: 10,
            max_files: 5,
            log_directory: PathBuf::from("logs"),
            include_spans: true,
            include_targets: true,
            structured_fields: true,
        }
    }
}

/// Initialize logging system
pub fn init_logging(config: &LoggingConfig) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let mut layers = Vec::new();

    // Console layer
    if config.console_enabled {
        let console_layer = fmt::layer()
            .with_target(config.include_targets)
            .with_span_events(if config.include_spans {
                FmtSpan::CLOSE
            } else {
                FmtSpan::NONE
            })
            .with_writer(std::io::stdout)
            .boxed();
        
        layers.push(console_layer);
    }

    // File layer
    if config.file_enabled {
        // Ensure log directory exists
        std::fs::create_dir_all(&config.log_directory)?;
        
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("winscrape")
            .filename_suffix("log")
            .max_log_files(config.max_files)
            .build(&config.log_directory)?;

        let file_layer = if config.json_format {
            fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(config.include_targets)
                .with_span_events(if config.include_spans {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_writer(file_appender)
                .boxed()
        } else {
            fmt::layer()
                .with_target(config.include_targets)
                .with_span_events(if config.include_spans {
                    FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_writer(file_appender)
                .boxed()
        };
        
        layers.push(file_layer);
    }

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(layers)
        .init();

    info!("Logging system initialized");
    info!("Log level: {}", config.level);
    info!("Console logging: {}", config.console_enabled);
    info!("File logging: {}", config.file_enabled);
    if config.file_enabled {
        info!("Log directory: {}", config.log_directory.display());
    }

    Ok(())
}

/// Structured logging context
#[derive(Debug, Clone, Serialize)]
pub struct LogContext {
    pub component: String,
    pub operation: String,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub job_id: Option<String>,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub duration_ms: Option<u64>,
    pub status: Option<String>,
    pub error_category: Option<String>,
    pub additional_fields: HashMap<String, serde_json::Value>,
}

impl LogContext {
    pub fn new(component: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            operation: operation.into(),
            request_id: None,
            user_id: None,
            job_id: None,
            url: None,
            domain: None,
            duration_ms: None,
            status: None,
            error_category: None,
            additional_fields: HashMap::new(),
        }
    }
    
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    
    pub fn with_job_id(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }
    
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
    
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
    
    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self
    }
    
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
    
    pub fn with_error_category(mut self, category: impl Into<String>) -> Self {
        self.error_category = Some(category.into());
        self
    }
    
    pub fn with_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.additional_fields.insert(key.into(), value);
        self
    }
    
    pub fn with_string_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_fields.insert(key.into(), serde_json::Value::String(value.into()));
        self
    }
    
    pub fn with_number_field(mut self, key: impl Into<String>, value: impl Into<i64>) -> Self {
        self.additional_fields.insert(key.into(), serde_json::Value::Number(serde_json::Number::from(value.into())));
        self
    }
    
    pub fn with_bool_field(mut self, key: impl Into<String>, value: bool) -> Self {
        self.additional_fields.insert(key.into(), serde_json::Value::Bool(value));
        self
    }
}

/// Structured logging macros
#[macro_export]
macro_rules! log_info {
    ($context:expr, $message:expr) => {
        tracing::info!(
            component = $context.component,
            operation = $context.operation,
            request_id = $context.request_id,
            user_id = $context.user_id,
            job_id = $context.job_id,
            url = $context.url,
            domain = $context.domain,
            duration_ms = $context.duration_ms,
            status = $context.status,
            additional_fields = ?$context.additional_fields,
            $message
        );
    };
    
    ($context:expr, $message:expr, $($key:expr => $value:expr),*) => {
        tracing::info!(
            component = $context.component,
            operation = $context.operation,
            request_id = $context.request_id,
            user_id = $context.user_id,
            job_id = $context.job_id,
            url = $context.url,
            domain = $context.domain,
            duration_ms = $context.duration_ms,
            status = $context.status,
            additional_fields = ?$context.additional_fields,
            $($key = $value,)*
            $message
        );
    };
}

#[macro_export]
macro_rules! log_warn {
    ($context:expr, $message:expr) => {
        tracing::warn!(
            component = $context.component,
            operation = $context.operation,
            request_id = $context.request_id,
            user_id = $context.user_id,
            job_id = $context.job_id,
            url = $context.url,
            domain = $context.domain,
            duration_ms = $context.duration_ms,
            status = $context.status,
            error_category = $context.error_category,
            additional_fields = ?$context.additional_fields,
            $message
        );
    };
}

#[macro_export]
macro_rules! log_error {
    ($context:expr, $message:expr) => {
        tracing::error!(
            component = $context.component,
            operation = $context.operation,
            request_id = $context.request_id,
            user_id = $context.user_id,
            job_id = $context.job_id,
            url = $context.url,
            domain = $context.domain,
            duration_ms = $context.duration_ms,
            status = $context.status,
            error_category = $context.error_category,
            additional_fields = ?$context.additional_fields,
            $message
        );
    };
    
    ($context:expr, $error:expr, $message:expr) => {
        tracing::error!(
            component = $context.component,
            operation = $context.operation,
            request_id = $context.request_id,
            user_id = $context.user_id,
            job_id = $context.job_id,
            url = $context.url,
            domain = $context.domain,
            duration_ms = $context.duration_ms,
            status = $context.status,
            error_category = $context.error_category,
            additional_fields = ?$context.additional_fields,
            error = %$error,
            $message
        );
    };
}

#[macro_export]
macro_rules! log_debug {
    ($context:expr, $message:expr) => {
        tracing::debug!(
            component = $context.component,
            operation = $context.operation,
            request_id = $context.request_id,
            additional_fields = ?$context.additional_fields,
            $message
        );
    };
}

/// Performance measurement utilities
pub struct PerformanceLogger {
    context: LogContext,
    start_time: std::time::Instant,
}

impl PerformanceLogger {
    pub fn new(context: LogContext) -> Self {
        Self {
            context,
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn finish(self, message: &str) {
        let duration = self.start_time.elapsed();
        let context = self.context.with_duration(duration);
        tracing::info!(
            component = context.component,
            operation = context.operation,
            request_id = context.request_id,
            user_id = context.user_id,
            job_id = context.job_id,
            url = context.url,
            domain = context.domain,
            duration_ms = context.duration_ms,
            status = context.status,
            additional_fields = ?context.additional_fields,
            "{}", message
        );
    }
    
    pub fn finish_with_status(self, message: &str, status: impl Into<String>) {
        let duration = self.start_time.elapsed();
        let context = self.context
            .with_duration(duration)
            .with_status(status);
        tracing::info!(
            component = context.component,
            operation = context.operation,
            request_id = context.request_id,
            user_id = context.user_id,
            job_id = context.job_id,
            url = context.url,
            domain = context.domain,
            duration_ms = context.duration_ms,
            status = context.status,
            additional_fields = ?context.additional_fields,
            "{}", message
        );
    }
    
    pub fn finish_with_error(self, message: &str, error: &crate::error::WinScrapeError) {
        let duration = self.start_time.elapsed();
        let context = self.context
            .with_duration(duration)
            .with_status("error")
            .with_error_category(error.category());
        tracing::error!(
            component = context.component,
            operation = context.operation,
            request_id = context.request_id,
            user_id = context.user_id,
            job_id = context.job_id,
            url = context.url,
            domain = context.domain,
            duration_ms = context.duration_ms,
            status = context.status,
            error_category = context.error_category,
            additional_fields = ?context.additional_fields,
            error = %error,
            "{}", message
        );
    }
}

/// Audit logging for security events
pub struct AuditLogger;

impl AuditLogger {
    pub fn log_security_event(
        event_type: &str,
        severity: &str,
        description: &str,
        context: Option<LogContext>,
    ) {
        let audit_context = context.unwrap_or_else(|| {
            LogContext::new("security", "audit")
        })
        .with_string_field("event_type", event_type)
        .with_string_field("severity", severity)
        .with_string_field("audit", "true");
        
        match severity {
            "critical" | "high" => {
                tracing::error!(
                    component = audit_context.component,
                    operation = audit_context.operation,
                    request_id = audit_context.request_id,
                    user_id = audit_context.user_id,
                    job_id = audit_context.job_id,
                    url = audit_context.url,
                    domain = audit_context.domain,
                    duration_ms = audit_context.duration_ms,
                    status = audit_context.status,
                    error_category = audit_context.error_category,
                    additional_fields = ?audit_context.additional_fields,
                    "{}", description
                );
            },
            "medium" => {
                tracing::warn!(
                    component = audit_context.component,
                    operation = audit_context.operation,
                    request_id = audit_context.request_id,
                    user_id = audit_context.user_id,
                    job_id = audit_context.job_id,
                    url = audit_context.url,
                    domain = audit_context.domain,
                    duration_ms = audit_context.duration_ms,
                    status = audit_context.status,
                    error_category = audit_context.error_category,
                    additional_fields = ?audit_context.additional_fields,
                    "{}", description
                );
            },
            _ => {
                tracing::info!(
                    component = audit_context.component,
                    operation = audit_context.operation,
                    request_id = audit_context.request_id,
                    user_id = audit_context.user_id,
                    job_id = audit_context.job_id,
                    url = audit_context.url,
                    domain = audit_context.domain,
                    duration_ms = audit_context.duration_ms,
                    status = audit_context.status,
                    additional_fields = ?audit_context.additional_fields,
                    "{}", description
                );
            },
        }
    }
    
    pub fn log_access_attempt(
        resource: &str,
        user_id: Option<&str>,
        success: bool,
        reason: Option<&str>,
    ) {
        let mut context = LogContext::new("security", "access_control")
            .with_string_field("resource", resource)
            .with_bool_field("success", success)
            .with_string_field("audit", "true");
        
        if let Some(uid) = user_id {
            context = context.with_user_id(uid);
        }
        
        if let Some(r) = reason {
            context = context.with_string_field("reason", r);
        }
        
        let message = if success {
            "Access granted"
        } else {
            "Access denied"
        };
        
        if success {
            tracing::info!(
                component = context.component,
                operation = context.operation,
                request_id = context.request_id,
                user_id = context.user_id,
                job_id = context.job_id,
                url = context.url,
                domain = context.domain,
                duration_ms = context.duration_ms,
                status = context.status,
                additional_fields = ?context.additional_fields,
                "{}", message
            );
        } else {
            tracing::warn!(
                component = context.component,
                operation = context.operation,
                request_id = context.request_id,
                user_id = context.user_id,
                job_id = context.job_id,
                url = context.url,
                domain = context.domain,
                duration_ms = context.duration_ms,
                status = context.status,
                error_category = context.error_category,
                additional_fields = ?context.additional_fields,
                "{}", message
            );
        }
    }
    
    pub fn log_data_operation(
        operation: &str,
        data_type: &str,
        record_count: usize,
        user_id: Option<&str>,
    ) {
        let mut context = LogContext::new("data", operation)
            .with_string_field("data_type", data_type)
            .with_number_field("record_count", record_count as i64)
            .with_string_field("audit", "true");
        
        if let Some(uid) = user_id {
            context = context.with_user_id(uid);
        }
        
        log_info!(context, "Data operation completed");
    }
}

/// Request ID generation and management
pub struct RequestIdGenerator;

impl RequestIdGenerator {
    pub fn generate() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let timestamp = chrono::Utc::now().timestamp_millis();
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        
        format!("req-{}-{:04}", timestamp, counter % 10000)
    }
}

/// Log aggregation and metrics
pub struct LogMetrics {
    pub error_counts: HashMap<String, u64>,
    pub warning_counts: HashMap<String, u64>,
    pub performance_stats: HashMap<String, Vec<u64>>,
}

impl LogMetrics {
    pub fn new() -> Self {
        Self {
            error_counts: HashMap::new(),
            warning_counts: HashMap::new(),
            performance_stats: HashMap::new(),
        }
    }
    
    pub fn record_error(&mut self, category: &str) {
        *self.error_counts.entry(category.to_string()).or_insert(0) += 1;
    }
    
    pub fn record_warning(&mut self, category: &str) {
        *self.warning_counts.entry(category.to_string()).or_insert(0) += 1;
    }
    
    pub fn record_performance(&mut self, operation: &str, duration_ms: u64) {
        self.performance_stats
            .entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration_ms);
    }
    
    pub fn get_error_rate(&self, category: &str) -> f64 {
        let errors = self.error_counts.get(category).unwrap_or(&0);
        let warnings = self.warning_counts.get(category).unwrap_or(&0);
        let total = errors + warnings;
        
        if total == 0 {
            0.0
        } else {
            *errors as f64 / total as f64
        }
    }
    
    pub fn get_average_performance(&self, operation: &str) -> Option<f64> {
        self.performance_stats.get(operation).map(|durations| {
            let sum: u64 = durations.iter().sum();
            sum as f64 / durations.len() as f64
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_context_creation() {
        let context = LogContext::new("test_component", "test_operation")
            .with_request_id("req-123")
            .with_url("https://example.com")
            .with_string_field("custom", "value");
        
        assert_eq!(context.component, "test_component");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.request_id, Some("req-123".to_string()));
        assert_eq!(context.url, Some("https://example.com".to_string()));
        assert!(context.additional_fields.contains_key("custom"));
    }
    
    #[test]
    fn test_request_id_generation() {
        let id1 = RequestIdGenerator::generate();
        let id2 = RequestIdGenerator::generate();
        
        assert_ne!(id1, id2);
        assert!(id1.starts_with("req-"));
        assert!(id2.starts_with("req-"));
    }
    
    #[test]
    fn test_log_metrics() {
        let mut metrics = LogMetrics::new();
        
        metrics.record_error("network");
        metrics.record_error("network");
        metrics.record_warning("network");
        
        assert_eq!(metrics.get_error_rate("network"), 2.0 / 3.0);
        
        metrics.record_performance("scraping", 100);
        metrics.record_performance("scraping", 200);
        
        assert_eq!(metrics.get_average_performance("scraping"), Some(150.0));
    }
}
