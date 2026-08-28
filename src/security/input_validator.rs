use anyhow::Result;
use regex::Regex;
use tracing::{debug, warn};

use crate::config::SecurityConfig;

/// Input validator for detecting malicious content
pub struct InputValidator {
    config: SecurityConfig,
    sql_injection_patterns: Vec<Regex>,
    xss_patterns: Vec<Regex>,
    command_injection_patterns: Vec<Regex>,
    path_traversal_patterns: Vec<Regex>,
}

impl InputValidator {
    /// Create new input validator
    pub fn new(config: &SecurityConfig) -> Result<Self> {
        let sql_injection_patterns = Self::compile_sql_patterns()?;
        let xss_patterns = Self::compile_xss_patterns()?;
        let command_injection_patterns = Self::compile_command_patterns()?;
        let path_traversal_patterns = Self::compile_path_patterns()?;
        
        Ok(Self {
            config: config.clone(),
            sql_injection_patterns,
            xss_patterns,
            command_injection_patterns,
            path_traversal_patterns,
        })
    }
    
    /// Validate input string for security issues
    pub fn validate(&self, input: &str) -> Result<()> {
        if !self.config.enable_input_validation {
            return Ok(());
        }
        
        debug!("Validating input: {} characters", input.len());
        
        // Check for SQL injection
        self.check_sql_injection(input)?;
        
        // Check for XSS
        self.check_xss(input)?;
        
        // Check for command injection
        self.check_command_injection(input)?;
        
        // Check for path traversal
        self.check_path_traversal(input)?;
        
        // Check for suspicious patterns
        self.check_suspicious_patterns(input)?;
        
        debug!("Input validation passed");
        Ok(())
    }
    
    /// Check for SQL injection patterns
    fn check_sql_injection(&self, input: &str) -> Result<()> {
        let input_lower = input.to_lowercase();
        
        for pattern in &self.sql_injection_patterns {
            if pattern.is_match(&input_lower) {
                warn!("SQL injection pattern detected in input");
                return Err(anyhow::anyhow!("Input contains potential SQL injection"));
            }
        }
        
        Ok(())
    }
    
    /// Check for XSS patterns
    fn check_xss(&self, input: &str) -> Result<()> {
        let input_lower = input.to_lowercase();
        
        for pattern in &self.xss_patterns {
            if pattern.is_match(&input_lower) {
                warn!("XSS pattern detected in input");
                return Err(anyhow::anyhow!("Input contains potential XSS"));
            }
        }
        
        Ok(())
    }
    
    /// Check for command injection patterns
    fn check_command_injection(&self, input: &str) -> Result<()> {
        for pattern in &self.command_injection_patterns {
            if pattern.is_match(input) {
                warn!("Command injection pattern detected in input");
                return Err(anyhow::anyhow!("Input contains potential command injection"));
            }
        }
        
        Ok(())
    }
    
    /// Check for path traversal patterns
    fn check_path_traversal(&self, input: &str) -> Result<()> {
        for pattern in &self.path_traversal_patterns {
            if pattern.is_match(input) {
                warn!("Path traversal pattern detected in input");
                return Err(anyhow::anyhow!("Input contains potential path traversal"));
            }
        }
        
        Ok(())
    }
    
    /// Check for other suspicious patterns
    fn check_suspicious_patterns(&self, input: &str) -> Result<()> {
        // Check for excessive special characters
        let special_char_count = input.chars()
            .filter(|c| "!@#$%^&*(){}[]|\\:;\"'<>?/~`".contains(*c))
            .count();
        
        let special_char_ratio = special_char_count as f64 / input.len() as f64;
        if special_char_ratio > 0.3 {
            warn!("Input has high ratio of special characters: {:.2}", special_char_ratio);
            return Err(anyhow::anyhow!("Input contains suspicious character patterns"));
        }
        
        // Check for repeated patterns that might indicate injection
        if self.has_repeated_injection_patterns(input) {
            warn!("Input contains repeated injection patterns");
            return Err(anyhow::anyhow!("Input contains repeated suspicious patterns"));
        }
        
        Ok(())
    }
    
    /// Check for repeated injection patterns
    fn has_repeated_injection_patterns(&self, input: &str) -> bool {
        // Note: this used to also include "select", "delete", and "drop",
        // but those are ordinary English words that a natural-language
        // scraping description can easily repeat (e.g. "select the name,
        // select the price, and select the rating" - three "select"s in
        // a completely normal field description). The sequences below
        // are unambiguous even in isolation, so repetition isn't needed
        // to make them suspicious, but we still only escalate to a hard
        // block when one recurs, to avoid flagging e.g. a single
        // legitimate use of `--` in scraped-content descriptions.
        let suspicious_sequences = [
            "''", "\"\"", "--", "/*", "*/", "union select",
            "<script", "</script>", "javascript:", "eval(", "alert(", "confirm(",
        ];
        
        for sequence in &suspicious_sequences {
            if input.to_lowercase().matches(sequence).count() > 2 {
                return true;
            }
        }
        
        false
    }
    
    /// Compile SQL injection patterns
    fn compile_sql_patterns() -> Result<Vec<Regex>> {
        let patterns = vec![
            r"(?i)\bunion\s+select\b",
            // Note: a plain `select ... from` pattern used to be included
            // here, but this app's primary input is natural-language
            // scraping descriptions, and phrases like "select the price
            // from the page" or "select images from the gallery" are
            // completely ordinary things a user would type - that pattern
            // had a very real false-positive rate on legitimate input.
            // The more specific SQL patterns below (actual SQL verbs
            // combined with SQL-only syntax) still catch real injection
            // attempts without that collision.
            r"(?i)\bdrop\s+table\b",
            r"(?i)\bdelete\s+from\b",
            r"(?i)\binsert\s+into\b",
            r"(?i)\bupdate\s+.*\bset\b",
            r"(?i)\balter\s+table\b",
            r"(?i)\bcreate\s+table\b",
            r"(?i)\bexec\s*\(",
            r"(?i)\bexecute\s*\(",
            r"(?i)'\s*or\s*'1'\s*=\s*'1",
            r"(?i)'\s*or\s*1\s*=\s*1",
            r"(?i)--\s*$",
            r"/\*.*\*/",
            r"(?i)\bxp_cmdshell\b",
            r"(?i)\bsp_executesql\b",
        ];
        
        Self::compile_patterns(&patterns)
    }
    
    /// Compile XSS patterns
    fn compile_xss_patterns() -> Result<Vec<Regex>> {
        let patterns = vec![
            r"(?i)<script[^>]*>",
            r"(?i)</script>",
            r"(?i)javascript:",
            r"(?i)vbscript:",
            r"(?i)data:text/html",
            // Inline event-handler attributes (onerror=, onclick=, ...).
            // Requires an enclosing `<...>` tag and whitespace before
            // "on" so this only matches an actual HTML attribute, not any
            // text containing "on" followed by "=" - the previous pattern
            // (`on\w+\s*=` with no tag requirement) matched ordinary
            // phrases like "condition=new" or "pagination=true", which
            // are completely normal in a scraping field description.
            r"(?i)<[^>]+\son\w+\s*=",
            r"(?i)<iframe[^>]*>",
            r"(?i)<object[^>]*>",
            r"(?i)<embed[^>]*>",
            r"(?i)<link[^>]*>",
            r"(?i)<meta[^>]*>",
            r"(?i)eval\s*\(",
            r"(?i)alert\s*\(",
            r"(?i)confirm\s*\(",
            r"(?i)prompt\s*\(",
            r"(?i)document\.cookie",
            r"(?i)document\.write",
            r"(?i)window\.location",
        ];
        
        Self::compile_patterns(&patterns)
    }
    
    /// Compile command injection patterns
    fn compile_command_patterns() -> Result<Vec<Regex>> {
        let patterns = vec![
            r";\s*rm\s",
            r";\s*cat\s",
            r";\s*ls\s",
            r";\s*pwd\s",
            r";\s*id\s",
            r";\s*whoami\s",
            r";\s*ps\s",
            r";\s*kill\s",
            r"&&\s*rm\s",
            r"&&\s*cat\s",
            r"&&\s*ls\s",
            r"\|\s*rm\s",
            r"\|\s*cat\s",
            // Backtick command substitution. Narrowed to require at least
            // two space-separated tokens inside the backticks (looking
            // like an actual shell command with an argument, e.g.
            // `` `cat /etc/passwd` ``) rather than any backtick-quoted
            // text - a bare single token in backticks is a common way to
            // reference a CSS selector or field name in a description
            // (e.g. "extract text from `.product-title`"), which the
            // previous unrestricted `` `.*` `` pattern would have blocked.
            r"`[^`\n]*\s+[^`\n]*`",
            r"\$\(.*\)",
            r">\s*/dev/null",
            r"2>&1",
            r"/bin/sh",
            r"/bin/bash",
            r"cmd\.exe",
            r"powershell",
        ];
        
        Self::compile_patterns(&patterns)
    }
    
    /// Compile path traversal patterns
    fn compile_path_patterns() -> Result<Vec<Regex>> {
        let patterns = vec![
            r"\.\./",
            r"\.\.\\",
            r"%2e%2e%2f",
            r"%2e%2e%5c",
            r"\.\.%2f",
            r"\.\.%5c",
            r"%252e%252e%252f",
            r"%252e%252e%255c",
            r"..%c0%af",
            r"..%c1%9c",
        ];
        
        Self::compile_patterns(&patterns)
    }
    
    /// Compile regex patterns
    fn compile_patterns(patterns: &[&str]) -> Result<Vec<Regex>> {
        let mut compiled = Vec::new();
        
        for pattern in patterns {
            match Regex::new(pattern) {
                Ok(regex) => compiled.push(regex),
                Err(e) => {
                    warn!("Failed to compile security pattern '{}': {}", pattern, e);
                    // Continue with other patterns
                }
            }
        }
        
        Ok(compiled)
    }
    
    /// Sanitize input by removing dangerous characters
    pub fn sanitize_input(&self, input: &str) -> String {
        let mut sanitized = input.to_string();
        
        // Remove null bytes
        sanitized = sanitized.replace('\0', "");
        
        // Remove control characters except newlines and tabs
        sanitized = sanitized.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();
        
        // Normalize whitespace
        sanitized = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
        
        // Truncate if too long
        if sanitized.len() > self.config.max_input_length {
            sanitized.truncate(self.config.max_input_length);
        }
        
        sanitized
    }
    
    /// Check if input is safe (non-blocking validation)
    pub fn is_safe(&self, input: &str) -> bool {
        self.validate(input).is_ok()
    }
    
    /// Get validation statistics
    pub fn get_stats(&self) -> ValidationStats {
        ValidationStats {
            sql_patterns: self.sql_injection_patterns.len(),
            xss_patterns: self.xss_patterns.len(),
            command_patterns: self.command_injection_patterns.len(),
            path_patterns: self.path_traversal_patterns.len(),
            max_input_length: self.config.max_input_length,
            validation_enabled: self.config.enable_input_validation,
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone)]
pub struct ValidationStats {
    pub sql_patterns: usize,
    pub xss_patterns: usize,
    pub command_patterns: usize,
    pub path_patterns: usize,
    pub max_input_length: usize,
    pub validation_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_config() -> SecurityConfig {
        SecurityConfig {
            enable_input_validation: true,
            enable_output_filtering: true,
            max_input_length: 10000,
            blocked_domains: vec![],
            allowed_schemes: vec!["http".to_string(), "https".to_string()],
            enable_rate_limiting: true,
            rate_limit_requests_per_minute: 60,
        }
    }
    
    #[test]
    fn test_sql_injection_detection() {
        let config = create_test_config();
        let validator = InputValidator::new(&config).unwrap();
        
        // Should detect SQL injection
        assert!(validator.validate("'; DROP TABLE users; --").is_err());
        assert!(validator.validate("1' OR '1'='1").is_err());
        assert!(validator.validate("UNION SELECT password FROM users").is_err());
        
        // Should allow safe input
        assert!(validator.validate("search for products").is_ok());
        assert!(validator.validate("user@example.com").is_ok());
    }
    
    #[test]
    fn test_xss_detection() {
        let config = create_test_config();
        let validator = InputValidator::new(&config).unwrap();
        
        // Should detect XSS
        assert!(validator.validate("<script>alert('xss')</script>").is_err());
        assert!(validator.validate("javascript:alert(1)").is_err());
        assert!(validator.validate("<img src=x onerror=alert(1)>").is_err());
        
        // Should allow safe HTML-like content
        assert!(validator.validate("Price: $10 <-- great deal").is_ok());
    }
    
    #[test]
    fn test_command_injection_detection() {
        let config = create_test_config();
        let validator = InputValidator::new(&config).unwrap();
        
        // Should detect command injection
        assert!(validator.validate("test; rm -rf /").is_err());
        assert!(validator.validate("data && cat /etc/passwd").is_err());
        assert!(validator.validate("$(whoami)").is_err());
        
        // Should allow safe input
        assert!(validator.validate("test data").is_ok());
    }
    
    #[test]
    fn test_path_traversal_detection() {
        let config = create_test_config();
        let validator = InputValidator::new(&config).unwrap();
        
        // Should detect path traversal
        assert!(validator.validate("../../../etc/passwd").is_err());
        assert!(validator.validate("..\\..\\windows\\system32").is_err());
        assert!(validator.validate("%2e%2e%2f").is_err());
        
        // Should allow safe paths
        assert!(validator.validate("./data/file.txt").is_ok());
        assert!(validator.validate("folder/subfolder").is_ok());
    }
    
    #[test]
    fn test_input_sanitization() {
        let config = create_test_config();
        let validator = InputValidator::new(&config).unwrap();
        
        let dirty_input = "test\0input\x01with\x02control\x03chars";
        let sanitized = validator.sanitize_input(dirty_input);
        
        assert!(!sanitized.contains('\0'));
        assert!(!sanitized.contains('\x01'));
        assert_eq!(sanitized, "testinputwithcontrolchars");
    }
    
    /// Regression tests for false positives that used to block ordinary
    /// natural-language scraping descriptions. This app's main input is
    /// plain-English descriptions of what to scrape, so patterns that
    /// collide with common phrasing are a real usability bug, not just a
    /// theoretical concern.
    #[test]
    fn test_natural_language_descriptions_are_not_blocked() {
        let config = create_test_config();
        let validator = InputValidator::new(&config).unwrap();
        
        // "select ... from" is completely ordinary phrasing when
        // describing what to scrape - it used to be flagged as SQL
        // injection.
        assert!(validator.validate("select the price from the page").is_ok());
        assert!(validator.validate("select images from the gallery").is_ok());
        
        // Field/filter descriptions using "key=value" style, outside of
        // any HTML tag, used to be flagged as XSS via a bare `on\w+=`
        // pattern.
        assert!(validator.validate("only include items where condition=new").is_ok());
        assert!(validator.validate("filter by pagination=true").is_ok());
        
        // Describing several fields with "select" repeated 3+ times used
        // to trip the repeated-injection-pattern check.
        assert!(validator.validate(
            "select the name, select the price, and select the rating for each item"
        ).is_ok());
        
        // Referencing a CSS selector in backticks used to be flagged as
        // command substitution.
        assert!(validator.validate("extract text from `.product-title`").is_ok());
        
        // Genuine attacks should still be caught after these fixes.
        assert!(validator.validate("<img src=x onerror=alert(1)>").is_err());
        assert!(validator.validate("`cat /etc/passwd`").is_err());
    }
}
