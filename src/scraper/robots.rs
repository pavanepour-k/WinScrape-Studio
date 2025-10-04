use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use url::Url;

use super::http_client::HttpClient;

/// Robots.txt checker and parser
pub struct RobotsChecker {
    http_client: Arc<HttpClient>,
    cache: Arc<RwLock<HashMap<String, RobotsRules>>>,
}

/// Parsed robots.txt rules for a domain
#[derive(Debug, Clone)]
pub struct RobotsRules {
    pub rules: Vec<RobotRule>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

/// Individual robot rule
#[derive(Debug, Clone)]
pub struct RobotRule {
    pub user_agent: String,
    pub disallow: Vec<String>,
    pub allow: Vec<String>,
    pub crawl_delay: Option<u64>,
}

impl RobotsChecker {
    pub fn new(http_client: Arc<HttpClient>) -> Self {
        Self {
            http_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Check if a URL is allowed for a given user agent
    pub async fn is_allowed(&self, url: &Url, user_agent: &str) -> Result<bool> {
        let domain = url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;
        
        // Get robots rules for domain
        let rules = self.get_robots_rules(domain).await?;
        
        // Check if URL is allowed
        Ok(self.check_url_allowed(url, user_agent, &rules))
    }
    
    /// Get crawl delay for a domain and user agent
    pub async fn get_crawl_delay(&self, domain: &str, user_agent: &str) -> Result<Option<u64>> {
        let rules = self.get_robots_rules(domain).await?;
        
        // Find matching rule
        for rule in &rules.rules {
            if self.matches_user_agent(&rule.user_agent, user_agent) {
                if let Some(delay) = rule.crawl_delay {
                    return Ok(Some(delay));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get robots rules for a domain (with caching)
    async fn get_robots_rules(&self, domain: &str) -> Result<RobotsRules> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(rules) = cache.get(domain) {
                // Check if cache is still valid (24 hours)
                let cache_age = chrono::Utc::now() - rules.cached_at;
                if cache_age.num_hours() < 24 {
                    debug!("Using cached robots.txt for domain: {}", domain);
                    return Ok(rules.clone());
                }
            }
        }
        
        // Fetch and parse robots.txt
        debug!("Fetching robots.txt for domain: {}", domain);
        let robots_txt = self.http_client.get_robots_txt(domain, "WinScrape-Studio/1.0").await?;
        let rules = self.parse_robots_txt(&robots_txt);
        
        // Cache the rules
        {
            let mut cache = self.cache.write().await;
            cache.insert(domain.to_string(), rules.clone());
        }
        
        Ok(rules)
    }
    
    /// Parse robots.txt content
    fn parse_robots_txt(&self, content: &str) -> RobotsRules {
        let mut rules = Vec::new();
        let mut current_user_agent = String::new();
        let mut current_disallow = Vec::new();
        let mut current_allow = Vec::new();
        let mut current_crawl_delay = None;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse directive
            if let Some((directive, value)) = line.split_once(':') {
                let directive = directive.trim().to_lowercase();
                let value = value.trim();
                
                match directive.as_str() {
                    "user-agent" => {
                        // Save previous rule if exists
                        if !current_user_agent.is_empty() {
                            rules.push(RobotRule {
                                user_agent: current_user_agent.clone(),
                                disallow: current_disallow.clone(),
                                allow: current_allow.clone(),
                                crawl_delay: current_crawl_delay,
                            });
                        }
                        
                        // Start new rule
                        current_user_agent = value.to_string();
                        current_disallow.clear();
                        current_allow.clear();
                        current_crawl_delay = None;
                    }
                    "disallow" => {
                        if !value.is_empty() {
                            current_disallow.push(value.to_string());
                        }
                    }
                    "allow" => {
                        if !value.is_empty() {
                            current_allow.push(value.to_string());
                        }
                    }
                    "crawl-delay" => {
                        current_crawl_delay = value.parse().ok();
                    }
                    _ => {
                        // Ignore unknown directives
                    }
                }
            }
        }
        
        // Save final rule
        if !current_user_agent.is_empty() {
            rules.push(RobotRule {
                user_agent: current_user_agent,
                disallow: current_disallow,
                allow: current_allow,
                crawl_delay: current_crawl_delay,
            });
        }
        
        RobotsRules {
            rules,
            cached_at: chrono::Utc::now(),
        }
    }
    
    /// Check if URL is allowed based on rules
    fn check_url_allowed(&self, url: &Url, user_agent: &str, rules: &RobotsRules) -> bool {
        let path = url.path();
        
        // Find matching rules for user agent
        let mut applicable_rules = Vec::new();
        
        for rule in &rules.rules {
            if self.matches_user_agent(&rule.user_agent, user_agent) {
                applicable_rules.push(rule);
            }
        }
        
        // If no specific rules found, check for wildcard rules
        if applicable_rules.is_empty() {
            for rule in &rules.rules {
                if rule.user_agent == "*" {
                    applicable_rules.push(rule);
                }
            }
        }
        
        // If still no rules, allow by default
        if applicable_rules.is_empty() {
            return true;
        }
        
        // Check rules in order (allow rules take precedence over disallow)
        for rule in &applicable_rules {
            // Check allow rules first
            for allow_pattern in &rule.allow {
                if self.matches_path(path, allow_pattern) {
                    debug!("URL {} allowed by rule: {}", url, allow_pattern);
                    return true;
                }
            }
            
            // Check disallow rules
            for disallow_pattern in &rule.disallow {
                if self.matches_path(path, disallow_pattern) {
                    debug!("URL {} disallowed by rule: {}", url, disallow_pattern);
                    return false;
                }
            }
        }
        
        // Default to allow if no matching rules
        true
    }
    
    /// Check if user agent matches pattern
    fn matches_user_agent(&self, pattern: &str, user_agent: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        // Case-insensitive substring match
        user_agent.to_lowercase().contains(&pattern.to_lowercase())
    }
    
    /// Check if path matches robots.txt pattern
    fn matches_path(&self, path: &str, pattern: &str) -> bool {
        if pattern == "/" {
            return true; // Root pattern matches everything
        }
        
        if pattern.is_empty() {
            return false;
        }
        
        // Handle wildcard patterns
        if pattern.contains('*') {
            return self.matches_wildcard_pattern(path, pattern);
        }
        
        // Exact prefix match
        path.starts_with(pattern)
    }
    
    /// Match wildcard patterns in robots.txt
    fn matches_wildcard_pattern(&self, path: &str, pattern: &str) -> bool {
        // Convert robots.txt pattern to regex
        let regex_pattern = pattern
            .replace(".", r"\.")
            .replace("*", ".*")
            .replace("?", ".");
        
        if let Ok(regex) = regex::Regex::new(&format!("^{}", regex_pattern)) {
            regex.is_match(path)
        } else {
            // Fallback to simple prefix match if regex fails
            path.starts_with(&pattern.replace('*', ""))
        }
    }
    
    /// Clear cache (for testing or manual refresh)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, Vec<String>) {
        let cache = self.cache.read().await;
        let count = cache.len();
        let domains: Vec<String> = cache.keys().cloned().collect();
        (count, domains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock HTTP client for testing
    struct MockHttpClient;
    
    impl MockHttpClient {
        fn new() -> Self {
            Self
        }
    }
    
    // Create a simple mock that implements the same interface as HttpClient
    impl MockHttpClient {
        async fn get(&self, _url: &Url, _user_agent: &str, _custom_headers: &Option<HashMap<String, String>>) -> Result<reqwest::Response> {
            Err(anyhow::anyhow!("Mock HTTP client - not implemented"))
        }
        
        async fn get_robots_txt(&self, _domain: &str, _user_agent: &str) -> Result<String> {
            Err(anyhow::anyhow!("Mock HTTP client - not implemented"))
        }
        
        async fn check_url(&self, _url: &Url, _user_agent: &str) -> Result<bool> {
            Err(anyhow::anyhow!("Mock HTTP client - not implemented"))
        }
        
        async fn get_headers(&self, _url: &Url, _user_agent: &str) -> Result<reqwest::header::HeaderMap> {
            Err(anyhow::anyhow!("Mock HTTP client - not implemented"))
        }
    }
    
    // Create a mock HttpClient that can be used in tests
    async fn create_mock_http_client() -> Result<HttpClient> {
        use crate::config::ScrapingConfig;
        
        let config = ScrapingConfig {
            max_retries: 3,
            retry_delay_seconds: 1,
            request_timeout_seconds: 30,
            ..Default::default()
        };
        
        HttpClient::new(&config).await
    }
    
    #[tokio::test]
    async fn test_parse_robots_txt() {
        let robots_content = r#"
User-agent: *
Disallow: /private/
Disallow: /temp/
Allow: /public/
Crawl-delay: 1

User-agent: Googlebot
Disallow: /admin/
Allow: /api/
Crawl-delay: 2
"#;
        
        let http_client = create_mock_http_client().await.unwrap();
        let checker = RobotsChecker::new(Arc::new(http_client));
        
        let rules = checker.parse_robots_txt(robots_content);
        
        assert_eq!(rules.rules.len(), 2);
        
        // Check first rule (*)
        let first_rule = &rules.rules[0];
        assert_eq!(first_rule.user_agent, "*");
        assert_eq!(first_rule.disallow, vec!["/private/", "/temp/"]);
        assert_eq!(first_rule.allow, vec!["/public/"]);
        assert_eq!(first_rule.crawl_delay, Some(1));
        
        // Check second rule (Googlebot)
        let second_rule = &rules.rules[1];
        assert_eq!(second_rule.user_agent, "Googlebot");
        assert_eq!(second_rule.disallow, vec!["/admin/"]);
        assert_eq!(second_rule.allow, vec!["/api/"]);
        assert_eq!(second_rule.crawl_delay, Some(2));
    }
    
    #[tokio::test]
    async fn test_path_matching() {
        let http_client = create_mock_http_client().await.unwrap();
        let checker = RobotsChecker::new(Arc::new(http_client));
        
        // Test exact matches
        assert!(checker.matches_path("/admin/page", "/admin/"));
        assert!(!checker.matches_path("/public/page", "/admin/"));
        
        // Test wildcard matches
        assert!(checker.matches_wildcard_pattern("/admin/test.html", "/admin/*.html"));
        assert!(checker.matches_wildcard_pattern("/any/path", "/*"));
        assert!(!checker.matches_wildcard_pattern("/admin/test.php", "/admin/*.html"));
    }
}
