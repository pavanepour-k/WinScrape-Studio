use anyhow::Result;
use regex::Regex;
use tracing::{warn, error, debug};
use url::Url;

pub mod input_validator;
pub mod output_filter;
// pub mod rate_limiter; // Moved to scraper module
pub mod domain_whitelist;

use crate::config::SecurityConfig;
use crate::dsl::ScrapePlan;

/// Security manager for validating inputs and enforcing policies
pub struct SecurityManager {
    config: SecurityConfig,
    input_validator: input_validator::InputValidator,
    output_filter: output_filter::OutputFilter,
    domain_whitelist: domain_whitelist::DomainWhitelist,
    blocked_patterns: Vec<Regex>,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: &SecurityConfig) -> Result<Self> {
        let input_validator = input_validator::InputValidator::new(config)?;
        let output_filter = output_filter::OutputFilter::new(config)?;
        let domain_whitelist = domain_whitelist::DomainWhitelist::new(&config.blocked_domains)?;
        
        // Compile blocked patterns
        let blocked_patterns = Self::compile_blocked_patterns()?;
        
        Ok(Self {
            config: config.clone(),
            input_validator,
            output_filter,
            domain_whitelist,
            blocked_patterns,
        })
    }
    
    /// Validate user input for security issues
    pub fn validate_input(&self, input: &str) -> Result<()> {
        debug!("Validating user input for security issues");
        
        // Check input length
        if input.len() > self.config.max_input_length {
            return Err(anyhow::anyhow!(
                "Input too long: {} characters, max allowed: {}",
                input.len(),
                self.config.max_input_length
            ));
        }
        
        // Use input validator
        self.input_validator.validate(input)?;
        
        // Check for blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(input) {
                warn!("Input contains blocked pattern");
                return Err(anyhow::anyhow!("Input contains potentially dangerous content"));
            }
        }
        
        debug!("Input validation passed");
        Ok(())
    }
    
    /// Validate DSL for security compliance
    pub fn validate_dsl(&self, dsl: &ScrapePlan) -> Result<()> {
        debug!("Validating DSL for security compliance");
        
        // Check domain whitelist
        self.domain_whitelist.validate_domain(&dsl.target.domain)?;
        
        // Validate URLs
        for url_str in &dsl.target.start_urls {
            self.validate_url(url_str)?;
        }
        
        // Validate URL patterns if present
        if let Some(patterns) = &dsl.target.url_patterns {
            for pattern in patterns {
                self.validate_url_pattern(pattern)?;
            }
        }
        
        // Check selectors for dangerous content
        self.validate_selectors(dsl)?;
        
        // Validate anti-blocking settings
        self.validate_anti_blocking_settings(dsl)?;
        
        debug!("DSL security validation passed");
        Ok(())
    }
    
    /// Validate URL for security
    fn validate_url(&self, url_str: &str) -> Result<()> {
        let url = Url::parse(url_str)
            .map_err(|e| anyhow::anyhow!("Invalid URL '{}': {}", url_str, e))?;
        
        // Check scheme
        if !self.config.allowed_schemes.contains(&url.scheme().to_string()) {
            return Err(anyhow::anyhow!(
                "URL scheme '{}' not allowed: {}",
                url.scheme(),
                url_str
            ));
        }
        
        // Check for localhost/internal addresses
        if let Some(host) = url.host_str() {
            if self.is_internal_address(host) {
                return Err(anyhow::anyhow!(
                    "Access to internal/localhost addresses not allowed: {}",
                    url_str
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate URL pattern
    fn validate_url_pattern(&self, pattern: &str) -> Result<()> {
        // Replace placeholders with test values
        let test_url = pattern.replace("{page}", "1");
        self.validate_url(&test_url)
    }
    
    /// Check if address is internal/localhost
    fn is_internal_address(&self, host: &str) -> bool {
        // Check for localhost variants
        if host == "localhost" || host == "127.0.0.1" || host == "::1" {
            return true;
        }
        
        // Check for private IP ranges
        if host.starts_with("192.168.") || 
           host.starts_with("10.") || 
           host.starts_with("172.") {
            return true;
        }
        
        // Check for link-local addresses
        if host.starts_with("169.254.") || host.starts_with("fe80:") {
            return true;
        }
        
        false
    }
    
    /// Validate selectors for dangerous content
    fn validate_selectors(&self, dsl: &ScrapePlan) -> Result<()> {
        // Check item selector
        self.validate_selector(&dsl.rules.item_selector)?;
        
        // Check field selectors
        for field in &dsl.rules.fields {
            self.validate_selector(&field.selector)?;
        }
        
        // Check pagination selectors
        if let Some(pagination) = &dsl.rules.pagination {
            match &pagination.method {
                crate::dsl::PaginationMethod::Link { next_selector } => {
                    self.validate_selector(next_selector)?;
                }
                crate::dsl::PaginationMethod::Button { button_selector } => {
                    self.validate_selector(button_selector)?;
                }
                _ => {}
            }
            
            if let Some(selector) = &pagination.selector {
                self.validate_selector(selector)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate individual selector
    fn validate_selector(&self, selector: &str) -> Result<()> {
        // Check for dangerous JavaScript execution
        if selector.contains("javascript:") || selector.contains("data:") {
            return Err(anyhow::anyhow!(
                "Selector contains potentially dangerous content: {}",
                selector
            ));
        }
        
        // Check for script injection attempts
        if selector.to_lowercase().contains("<script") || 
           selector.to_lowercase().contains("eval(") ||
           selector.to_lowercase().contains("function(") {
            return Err(anyhow::anyhow!(
                "Selector contains script injection attempt: {}",
                selector
            ));
        }
        
        Ok(())
    }
    
    /// Validate anti-blocking settings
    fn validate_anti_blocking_settings(&self, dsl: &ScrapePlan) -> Result<()> {
        let anti_blocking = &dsl.anti_blocking;
        
        // Check delay settings
        if anti_blocking.randomized_delays.min_ms < 100 {
            warn!("Very low minimum delay ({}ms) may overwhelm servers", 
                  anti_blocking.randomized_delays.min_ms);
        }
        
        // Check custom headers for dangerous content
        if let Some(headers) = &anti_blocking.headers {
            for (name, value) in headers {
                self.validate_header(name, value)?;
            }
        }
        
        // Validate proxy settings
        if let Some(proxy) = &anti_blocking.proxy {
            if proxy.enabled {
                for proxy_url in &proxy.proxies {
                    self.validate_proxy_url(proxy_url)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate HTTP header
    fn validate_header(&self, name: &str, value: &str) -> Result<()> {
        // Block dangerous headers
        let dangerous_headers = [
            "host", "content-length", "transfer-encoding", "connection",
            "upgrade", "proxy-authorization", "authorization"
        ];
        
        if dangerous_headers.contains(&name.to_lowercase().as_str()) {
            return Err(anyhow::anyhow!(
                "Header '{}' is not allowed for security reasons",
                name
            ));
        }
        
        // Check for injection attempts in header values
        if value.contains('\n') || value.contains('\r') {
            return Err(anyhow::anyhow!(
                "Header value contains line breaks (potential injection): {}",
                value
            ));
        }
        
        Ok(())
    }
    
    /// Validate proxy URL
    fn validate_proxy_url(&self, proxy_url: &str) -> Result<()> {
        let url = Url::parse(proxy_url)
            .map_err(|e| anyhow::anyhow!("Invalid proxy URL '{}': {}", proxy_url, e))?;
        
        // Only allow specific proxy schemes
        match url.scheme() {
            "http" | "https" | "socks5" => {}
            _ => {
                return Err(anyhow::anyhow!(
                    "Proxy scheme '{}' not allowed: {}",
                    url.scheme(),
                    proxy_url
                ));
            }
        }
        
        // Check for internal addresses
        if let Some(host) = url.host_str() {
            if self.is_internal_address(host) {
                return Err(anyhow::anyhow!(
                    "Proxy cannot use internal address: {}",
                    proxy_url
                ));
            }
        }
        
        Ok(())
    }
    
    /// Filter output data for sensitive information
    pub fn filter_output(&self, data: &mut [serde_json::Value]) -> Result<()> {
        if !self.config.enable_output_filtering {
            return Ok(());
        }
        
        debug!("Filtering output data for sensitive information");
        
        for item in data {
            self.output_filter.filter_item(item)?;
        }
        
        debug!("Output filtering completed");
        Ok(())
    }
    
    /// Compile blocked patterns for input validation
    fn compile_blocked_patterns() -> Result<Vec<Regex>> {
        let patterns = vec![
            // SQL injection patterns
            r"(?i)(union\s+select|drop\s+table|delete\s+from|insert\s+into)",
            // XSS patterns
            r"(?i)(<script|javascript:|on\w+\s*=)",
            // Command injection patterns
            r"(?i)(;\s*rm\s|;\s*cat\s|;\s*ls\s|&&\s*rm\s)",
            // Path traversal patterns
            r"(\.\./|\.\.\\|%2e%2e%2f|%2e%2e%5c)",
            // LDAP injection patterns
            r"(\*\)|\(\||\)\()",
        ];
        
        let mut compiled_patterns = Vec::new();
        for pattern in patterns {
            match Regex::new(pattern) {
                Ok(regex) => compiled_patterns.push(regex),
                Err(e) => {
                    error!("Failed to compile security pattern '{}': {}", pattern, e);
                    // Continue with other patterns
                }
            }
        }
        
        Ok(compiled_patterns)
    }
    
    /// Check if robots.txt should be enforced
    pub fn should_enforce_robots(&self, dsl: &ScrapePlan) -> bool {
        dsl.anti_blocking.respect_robots_txt
    }
    
    /// Get security configuration
    pub fn get_config(&self) -> &SecurityConfig {
        &self.config
    }
    
    /// Generate security report for a scraping plan
    pub fn generate_security_report(&self, dsl: &ScrapePlan) -> SecurityReport {
        let mut report = SecurityReport::new();
        
        // Check domain security
        if self.domain_whitelist.is_blocked(&dsl.target.domain) {
            report.add_warning("Target domain is in blocked list".to_string());
        }
        
        // Check URL security
        for url_str in &dsl.target.start_urls {
            if let Err(e) = self.validate_url(url_str) {
                report.add_error(format!("URL validation failed: {}", e));
            }
        }
        
        // Check delay settings
        if dsl.anti_blocking.randomized_delays.min_ms < 500 {
            report.add_warning("Very low delay settings may overwhelm target server".to_string());
        }
        
        // Check robots.txt compliance
        if !dsl.anti_blocking.respect_robots_txt {
            report.add_warning("Robots.txt compliance is disabled".to_string());
        }
        
        report
    }
}

/// Security report structure
#[derive(Debug, Clone)]
pub struct SecurityReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl SecurityReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn add_info(&mut self, info: String) {
        self.info.push(info);
    }
    
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }
}

impl Default for SecurityReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Security validation errors
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Input validation failed: {0}")]
    InputValidation(String),
    
    #[error("URL validation failed: {0}")]
    UrlValidation(String),
    
    #[error("Domain blocked: {0}")]
    DomainBlocked(String),
    
    #[error("Selector validation failed: {0}")]
    SelectorValidation(String),
    
    #[error("Header validation failed: {0}")]
    HeaderValidation(String),
    
    #[error("Output filtering failed: {0}")]
    OutputFiltering(String),
}
