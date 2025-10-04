use anyhow::Result;
use regex::Regex;
use tracing::{debug, warn};

use crate::config::SecurityConfig;

/// Output filter for removing sensitive information
pub struct OutputFilter {
    config: SecurityConfig,
    sensitive_patterns: Vec<SensitivePattern>,
}

/// Pattern for detecting sensitive information
struct SensitivePattern {
    name: String,
    regex: Regex,
    replacement: String,
}

impl OutputFilter {
    /// Create new output filter
    pub fn new(config: &SecurityConfig) -> Result<Self> {
        let sensitive_patterns = Self::create_sensitive_patterns()?;
        
        Ok(Self {
            config: config.clone(),
            sensitive_patterns,
        })
    }
    
    /// Filter a single data item
    pub fn filter_item(&self, item: &mut serde_json::Value) -> Result<()> {
        if !self.config.enable_output_filtering {
            return Ok(());
        }
        
        match item {
            serde_json::Value::Object(obj) => {
                for (key, value) in obj.iter_mut() {
                    self.filter_value(key, value)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for value in arr.iter_mut() {
                    self.filter_item(value)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Filter a single value
    fn filter_value(&self, key: &str, value: &mut serde_json::Value) -> Result<()> {
        match value {
            serde_json::Value::String(s) => {
                let filtered = self.filter_string(s);
                if filtered != *s {
                    debug!("Filtered sensitive data in field: {}", key);
                    *s = filtered;
                }
            }
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                self.filter_item(value)?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Filter sensitive information from string
    fn filter_string(&self, input: &str) -> String {
        let mut filtered = input.to_string();
        
        for pattern in &self.sensitive_patterns {
            if pattern.regex.is_match(&filtered) {
                warn!("Found sensitive data pattern: {}", pattern.name);
                filtered = pattern.regex.replace_all(&filtered, &pattern.replacement).to_string();
            }
        }
        
        filtered
    }
    
    /// Create patterns for detecting sensitive information
    fn create_sensitive_patterns() -> Result<Vec<SensitivePattern>> {
        let mut patterns = Vec::new();
        
        // Credit card numbers
        patterns.push(SensitivePattern {
            name: "Credit Card".to_string(),
            regex: Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b")?,
            replacement: "[CREDIT_CARD_REDACTED]".to_string(),
        });
        
        // Social Security Numbers (US format)
        patterns.push(SensitivePattern {
            name: "SSN".to_string(),
            regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")?,
            replacement: "[SSN_REDACTED]".to_string(),
        });
        
        // Email addresses (optional filtering)
        patterns.push(SensitivePattern {
            name: "Email".to_string(),
            regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")?,
            replacement: "[EMAIL_REDACTED]".to_string(),
        });
        
        // Phone numbers
        patterns.push(SensitivePattern {
            name: "Phone".to_string(),
            regex: Regex::new(r"\b(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b")?,
            replacement: "[PHONE_REDACTED]".to_string(),
        });
        
        // IP addresses (internal ranges)
        patterns.push(SensitivePattern {
            name: "Internal IP".to_string(),
            regex: Regex::new(r"\b(?:192\.168\.|10\.|172\.(?:1[6-9]|2[0-9]|3[01])\.)\d{1,3}\.\d{1,3}\b")?,
            replacement: "[IP_REDACTED]".to_string(),
        });
        
        // API keys (common patterns)
        patterns.push(SensitivePattern {
            name: "API Key".to_string(),
            regex: Regex::new(r"\b[A-Za-z0-9]{32,}\b")?,
            replacement: "[API_KEY_REDACTED]".to_string(),
        });
        
        // Passwords in URLs
        patterns.push(SensitivePattern {
            name: "Password in URL".to_string(),
            regex: Regex::new(r"://[^:]+:([^@]+)@")?,
            replacement: "://[USER]:[PASSWORD_REDACTED]@".to_string(),
        });
        
        Ok(patterns)
    }
    
    /// Check if field should be completely removed
    fn should_remove_field(&self, key: &str) -> bool {
        let sensitive_fields = [
            "password", "passwd", "pwd", "secret", "token", "key",
            "auth", "authorization", "credential", "private",
            "ssn", "social_security", "credit_card", "cc_number",
        ];
        
        let key_lower = key.to_lowercase();
        sensitive_fields.iter().any(|&field| key_lower.contains(field))
    }
    
    /// Remove sensitive fields entirely
    pub fn remove_sensitive_fields(&self, item: &mut serde_json::Value) -> Result<()> {
        if let serde_json::Value::Object(obj) = item {
            let keys_to_remove: Vec<String> = obj.keys()
                .filter(|key| self.should_remove_field(key))
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                debug!("Removing sensitive field: {}", key);
                obj.remove(&key);
            }
        }
        
        Ok(())
    }
    
    /// Get filtering statistics
    pub fn get_stats(&self) -> FilterStats {
        FilterStats {
            pattern_count: self.sensitive_patterns.len(),
            filtering_enabled: self.config.enable_output_filtering,
        }
    }
}

/// Filtering statistics
#[derive(Debug, Clone)]
pub struct FilterStats {
    pub pattern_count: usize,
    pub filtering_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
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
    fn test_credit_card_filtering() {
        let config = create_test_config();
        let filter = OutputFilter::new(&config).unwrap();
        
        let mut data = json!({
            "name": "John Doe",
            "card": "4532 1234 5678 9012"
        });
        
        filter.filter_item(&mut data).unwrap();
        
        assert_eq!(data["card"], "[CREDIT_CARD_REDACTED]");
        assert_eq!(data["name"], "John Doe"); // Should remain unchanged
    }
    
    #[test]
    fn test_email_filtering() {
        let config = create_test_config();
        let filter = OutputFilter::new(&config).unwrap();
        
        let mut data = json!({
            "contact": "Please email john.doe@example.com for more info"
        });
        
        filter.filter_item(&mut data).unwrap();
        
        let contact_str = data["contact"].as_str().unwrap();
        assert!(contact_str.contains("[EMAIL_REDACTED]"));
        assert!(!contact_str.contains("john.doe@example.com"));
    }
    
    #[test]
    fn test_nested_filtering() {
        let config = create_test_config();
        let filter = OutputFilter::new(&config).unwrap();
        
        let mut data = json!({
            "user": {
                "name": "John",
                "phone": "555-123-4567",
                "details": {
                    "ssn": "123-45-6789"
                }
            }
        });
        
        filter.filter_item(&mut data).unwrap();
        
        assert_eq!(data["user"]["phone"], "[PHONE_REDACTED]");
        assert_eq!(data["user"]["details"]["ssn"], "[SSN_REDACTED]");
        assert_eq!(data["user"]["name"], "John"); // Should remain unchanged
    }
    
    #[test]
    fn test_sensitive_field_removal() {
        let config = create_test_config();
        let filter = OutputFilter::new(&config).unwrap();
        
        let mut data = json!({
            "username": "john",
            "password": "secret123",
            "email": "john@example.com",
            "api_key": "abc123xyz"
        });
        
        filter.remove_sensitive_fields(&mut data).unwrap();
        
        assert!(data.get("username").is_some());
        assert!(data.get("email").is_some());
        assert!(data.get("password").is_none());
        assert!(data.get("api_key").is_none());
    }
}
