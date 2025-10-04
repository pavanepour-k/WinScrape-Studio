use std::fmt;
use thiserror::Error;

/// Comprehensive error types for WinScrape Studio
#[derive(Error, Debug)]
pub enum WinScrapeError {
    // Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Invalid configuration file: {path}")]
    InvalidConfig { path: String },
    
    // Database errors
    #[error("Database error: {message}")]
    Database { message: String },
    
    #[error("Database connection failed: {message}")]
    DatabaseConnection { message: String },
    
    #[error("Migration failed: {version}")]
    Migration { version: i32 },
    
    // Network errors
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("HTTP request failed: {url} - {status}")]
    HttpRequest { url: String, status: u16 },
    
    #[error("Connection timeout: {url}")]
    Timeout { url: String },
    
    // Scraping errors
    #[error("Scraping error: {message}")]
    Scraping { message: String },
    
    #[error("Robots.txt violation: {url}")]
    RobotsViolation { url: String },
    
    #[error("Rate limit exceeded for domain: {domain}")]
    RateLimit { domain: String },
    
    #[error("Selector failed: {selector} on {url}")]
    SelectorFailed { selector: String, url: String },
    
    // DSL errors
    #[error("DSL validation error: {message}")]
    DSLValidation { message: String },
    
    #[error("DSL parsing error: {message}")]
    DSLParsing { message: String },
    
    #[error("Invalid DSL structure: {field}")]
    InvalidDSL { field: String },
    
    // LLM errors
    #[error("LLM processing error: {message}")]
    LLM { message: String },
    
    #[error("Model loading failed: {path}")]
    ModelLoad { path: String },
    
    #[error("Text generation failed: {reason}")]
    TextGeneration { reason: String },
    
    // Security errors
    #[error("Security validation failed: {message}")]
    Security { message: String },
    
    #[error("Input validation failed: {input}")]
    InputValidation { input: String },
    
    #[error("Domain blocked: {domain}")]
    DomainBlocked { domain: String },
    
    #[error("Suspicious activity detected: {details}")]
    SuspiciousActivity { details: String },
    
    // Export errors
    #[error("Export error: {message}")]
    Export { message: String },
    
    #[error("File write failed: {path}")]
    FileWrite { path: String },
    
    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },
    
    // Job management errors
    #[error("Job error: {message}")]
    Job { message: String },
    
    #[error("Job not found: {job_id}")]
    JobNotFound { job_id: String },
    
    #[error("Job already running: {job_id}")]
    JobAlreadyRunning { job_id: String },
    
    #[error("Job queue full")]
    JobQueueFull,
    
    // UI errors
    #[error("UI error: {message}")]
    UI { message: String },
    
    #[error("Theme loading failed: {theme}")]
    ThemeLoad { theme: String },
    
    // System errors
    #[error("System error: {message}")]
    System { message: String },
    
    #[error("File system error: {path}")]
    FileSystem { path: String },
    
    #[error("Permission denied: {resource}")]
    PermissionDenied { resource: String },
    
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
    
    // Generic errors
    #[error("Internal error: {message}")]
    Internal { message: String },
    
    #[error("Operation cancelled")]
    Cancelled,
    
    #[error("Operation timeout")]
    OperationTimeout,
    
    #[error("Invalid state: {state}")]
    InvalidState { state: String },
}

impl WinScrapeError {
    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Configuration { message: message.into() }
    }
    
    /// Create a database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database { message: message.into() }
    }
    
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network { message: message.into() }
    }
    
    /// Create a scraping error
    pub fn scraping(message: impl Into<String>) -> Self {
        Self::Scraping { message: message.into() }
    }
    
    /// Create a DSL validation error
    pub fn dsl_validation(message: impl Into<String>) -> Self {
        Self::DSLValidation { message: message.into() }
    }
    
    /// Create a security error
    pub fn security(message: impl Into<String>) -> Self {
        Self::Security { message: message.into() }
    }
    
    /// Create an export error
    pub fn export(message: impl Into<String>) -> Self {
        Self::Export { message: message.into() }
    }
    
    /// Create a job error
    pub fn job(message: impl Into<String>) -> Self {
        Self::Job { message: message.into() }
    }
    
    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into() }
    }
    
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Recoverable errors
            Self::Network { .. } |
            Self::HttpRequest { .. } |
            Self::Timeout { .. } |
            Self::RateLimit { .. } |
            Self::JobQueueFull => true,
            
            // Non-recoverable errors
            Self::Configuration { .. } |
            Self::InvalidConfig { .. } |
            Self::DatabaseConnection { .. } |
            Self::Migration { .. } |
            Self::Security { .. } |
            Self::DomainBlocked { .. } |
            Self::PermissionDenied { .. } => false,
            
            // Context-dependent
            _ => false,
        }
    }
    
    /// Get error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::Configuration { .. } | Self::InvalidConfig { .. } => "configuration",
            Self::Database { .. } | Self::DatabaseConnection { .. } | Self::Migration { .. } => "database",
            Self::Network { .. } | Self::HttpRequest { .. } | Self::Timeout { .. } => "network",
            Self::Scraping { .. } | Self::RobotsViolation { .. } | Self::RateLimit { .. } | Self::SelectorFailed { .. } => "scraping",
            Self::DSLValidation { .. } | Self::DSLParsing { .. } | Self::InvalidDSL { .. } => "dsl",
            Self::LLM { .. } | Self::ModelLoad { .. } | Self::TextGeneration { .. } => "llm",
            Self::Security { .. } | Self::InputValidation { .. } | Self::DomainBlocked { .. } | Self::SuspiciousActivity { .. } => "security",
            Self::Export { .. } | Self::FileWrite { .. } | Self::UnsupportedFormat { .. } => "export",
            Self::Job { .. } | Self::JobNotFound { .. } | Self::JobAlreadyRunning { .. } | Self::JobQueueFull => "job",
            Self::UI { .. } | Self::ThemeLoad { .. } => "ui",
            Self::System { .. } | Self::FileSystem { .. } | Self::PermissionDenied { .. } | Self::ResourceExhausted { .. } => "system",
            Self::Internal { .. } | Self::Cancelled | Self::OperationTimeout | Self::InvalidState { .. } => "internal",
        }
    }
    
    /// Get suggested retry delay for recoverable errors
    pub fn retry_delay(&self) -> Option<std::time::Duration> {
        match self {
            Self::Network { .. } => Some(std::time::Duration::from_secs(5)),
            Self::HttpRequest { .. } => Some(std::time::Duration::from_secs(10)),
            Self::Timeout { .. } => Some(std::time::Duration::from_secs(15)),
            Self::RateLimit { .. } => Some(std::time::Duration::from_secs(60)),
            Self::JobQueueFull => Some(std::time::Duration::from_secs(30)),
            _ => None,
        }
    }
}

/// Result type alias for WinScrape Studio
pub type WinScrapeResult<T> = std::result::Result<T, WinScrapeError>;

/// Error context for enhanced debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub additional_data: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            component: component.into(),
            timestamp: chrono::Utc::now(),
            request_id: None,
            user_id: None,
            additional_data: std::collections::HashMap::new(),
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
    
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_data.insert(key.into(), value.into());
        self
    }
}

/// Enhanced error with context
#[derive(Debug)]
pub struct ContextualError {
    pub error: WinScrapeError,
    pub context: ErrorContext,
    pub chain: Vec<String>,
}

impl ContextualError {
    pub fn new(error: WinScrapeError, context: ErrorContext) -> Self {
        Self {
            error,
            context,
            chain: Vec::new(),
        }
    }
    
    pub fn with_chain(mut self, chain: Vec<String>) -> Self {
        self.chain = chain;
        self
    }
    
    pub fn add_to_chain(&mut self, message: impl Into<String>) {
        self.chain.push(message.into());
    }
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} in {}::{}", self.error, self.context.component, self.context.operation)?;
        
        if !self.chain.is_empty() {
            write!(f, " (chain: {})", self.chain.join(" -> "))?;
        }
        
        if let Some(request_id) = &self.context.request_id {
            write!(f, " [req: {}]", request_id)?;
        }
        
        Ok(())
    }
}

impl std::error::Error for ContextualError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Macro for creating contextual errors
#[macro_export]
macro_rules! contextual_error {
    ($error:expr, $operation:expr, $component:expr) => {
        $crate::error::ContextualError::new(
            $error,
            $crate::error::ErrorContext::new($operation, $component)
        )
    };
    
    ($error:expr, $operation:expr, $component:expr, $($key:expr => $value:expr),*) => {
        {
            let mut context = $crate::error::ErrorContext::new($operation, $component);
            $(
                context = context.with_data($key, $value);
            )*
            $crate::error::ContextualError::new($error, context)
        }
    };
}

/// Macro for early return with contextual error
#[macro_export]
macro_rules! bail_contextual {
    ($error:expr, $operation:expr, $component:expr) => {
        return Err($crate::contextual_error!($error, $operation, $component).into())
    };
}

/// Convert anyhow::Error to WinScrapeError
impl From<anyhow::Error> for WinScrapeError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal { message: err.to_string() }
    }
}

/// Convert ContextualError to anyhow::Error
// Removed conflicting From implementation

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let error = WinScrapeError::config("Invalid setting");
        assert_eq!(error.category(), "configuration");
        assert!(!error.is_recoverable());
    }
    
    #[test]
    fn test_recoverable_errors() {
        let network_error = WinScrapeError::network("Connection failed");
        assert!(network_error.is_recoverable());
        assert!(network_error.retry_delay().is_some());
        
        let config_error = WinScrapeError::config("Invalid config");
        assert!(!config_error.is_recoverable());
        assert!(config_error.retry_delay().is_none());
    }
    
    #[test]
    fn test_contextual_error() {
        let error = WinScrapeError::scraping("Failed to extract data");
        let context = ErrorContext::new("extract_items", "scraper")
            .with_request_id("req-123")
            .with_data("url", "https://example.com");
        
        let contextual = ContextualError::new(error, context);
        let error_string = contextual.to_string();
        
        assert!(error_string.contains("scraper::extract_items"));
        assert!(error_string.contains("req-123"));
    }
}
