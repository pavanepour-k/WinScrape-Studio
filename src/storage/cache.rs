use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tracing::{debug, warn};

use super::{StorageManager, CacheEntry};

/// High-level cache interface
pub struct CacheManager {
    storage: Arc<StorageManager>,
    default_ttl: Duration,
}

impl CacheManager {
    pub fn new(storage: Arc<StorageManager>, default_ttl_hours: i64) -> Self {
        Self {
            storage,
            default_ttl: Duration::hours(default_ttl_hours),
        }
    }
    
    /// Store a value in cache with default TTL
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, Some(self.default_ttl)).await
    }
    
    /// Store a value in cache with custom TTL
    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let serialized = bincode::serialize(value)?;
        let ttl_timestamp = ttl.map(|d| Utc::now() + d);
        
        let entry = CacheEntry {
            key: key.to_string(),
            value_blob: serialized,
            ttl: ttl_timestamp,
            created_at: Utc::now(),
        };
        
        self.storage.store_cache(&entry).await?;
        debug!("Cached value for key: {}", key);
        Ok(())
    }
    
    /// Get a value from cache
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        if let Some(entry) = self.storage.get_cache(key).await? {
            match bincode::deserialize(&entry.value_blob) {
                Ok(value) => {
                    debug!("Cache hit for key: {}", key);
                    Ok(Some(value))
                }
                Err(e) => {
                    warn!("Failed to deserialize cached value for key {}: {}", key, e);
                    Ok(None)
                }
            }
        } else {
            debug!("Cache miss for key: {}", key);
            Ok(None)
        }
    }
    
    /// Check if a key exists in cache
    pub async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.storage.get_cache(key).await?.is_some())
    }
    
    /// Remove a key from cache
    pub async fn remove(&self, key: &str) -> Result<()> {
        // This would require adding a delete method to StorageManager
        // For now, we'll set an expired entry
        let entry = CacheEntry {
            key: key.to_string(),
            value_blob: vec![],
            ttl: Some(Utc::now() - Duration::seconds(1)), // Already expired
            created_at: Utc::now(),
        };
        
        self.storage.store_cache(&entry).await?;
        debug!("Removed cache key: {}", key);
        Ok(())
    }
    
    /// Clean expired entries
    pub async fn clean_expired(&self) -> Result<usize> {
        self.storage.clean_expired_cache().await
    }
}

/// Specialized cache for HTTP responses
pub struct HttpCache {
    cache: CacheManager,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedHttpResponse {
    pub url: String,
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub cached_at: DateTime<Utc>,
}

impl HttpCache {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        Self {
            cache: CacheManager::new(storage, 24), // 24 hour default TTL
        }
    }
    
    /// Cache an HTTP response
    pub async fn cache_response(
        &self,
        url: &str,
        status_code: u16,
        headers: std::collections::HashMap<String, String>,
        body: String,
        ttl_hours: Option<i64>,
    ) -> Result<()> {
        let response = CachedHttpResponse {
            url: url.to_string(),
            status_code,
            headers,
            body,
            cached_at: Utc::now(),
        };
        
        let cache_key = self.generate_cache_key(url);
        let ttl = ttl_hours.map(Duration::hours);
        
        self.cache.set_with_ttl(&cache_key, &response, ttl).await
    }
    
    /// Get cached HTTP response
    pub async fn get_cached_response(&self, url: &str) -> Result<Option<CachedHttpResponse>> {
        let cache_key = self.generate_cache_key(url);
        self.cache.get(&cache_key).await
    }
    
    /// Generate cache key for URL
    fn generate_cache_key(&self, url: &str) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        format!("http:{:x}", hasher.finalize())
    }
}

/// Specialized cache for robots.txt files
pub struct RobotsCache {
    cache: CacheManager,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRobots {
    pub domain: String,
    pub robots_txt: String,
    pub parsed_rules: Vec<RobotRule>,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotRule {
    pub user_agent: String,
    pub disallow: Vec<String>,
    pub allow: Vec<String>,
    pub crawl_delay: Option<u64>,
}

impl RobotsCache {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        Self {
            cache: CacheManager::new(storage, 168), // 1 week default TTL
        }
    }
    
    /// Cache robots.txt for a domain
    pub async fn cache_robots(&self, domain: &str, robots_txt: String) -> Result<()> {
        let parsed_rules = self.parse_robots_txt(&robots_txt);
        
        let cached_robots = CachedRobots {
            domain: domain.to_string(),
            robots_txt,
            parsed_rules,
            cached_at: Utc::now(),
        };
        
        let cache_key = format!("robots:{}", domain);
        self.cache.set(&cache_key, &cached_robots).await
    }
    
    /// Get cached robots.txt for a domain
    pub async fn get_cached_robots(&self, domain: &str) -> Result<Option<CachedRobots>> {
        let cache_key = format!("robots:{}", domain);
        self.cache.get(&cache_key).await
    }
    
    /// Parse robots.txt content into structured rules
    fn parse_robots_txt(&self, robots_txt: &str) -> Vec<RobotRule> {
        let mut rules = Vec::new();
        let mut current_user_agent = String::new();
        let mut current_disallow = Vec::new();
        let mut current_allow = Vec::new();
        let mut current_crawl_delay = None;
        
        for line in robots_txt.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim();
                
                match key.as_str() {
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
                        current_disallow.push(value.to_string());
                    }
                    "allow" => {
                        current_allow.push(value.to_string());
                    }
                    "crawl-delay" => {
                        current_crawl_delay = value.parse().ok();
                    }
                    _ => {}
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
        
        rules
    }
}

/// Specialized cache for DSL generation results
pub struct DSLCache {
    cache: CacheManager,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDSL {
    pub user_prompt: String,
    pub generated_dsl: String,
    pub confidence_score: f32,
    pub cached_at: DateTime<Utc>,
}

impl DSLCache {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        Self {
            cache: CacheManager::new(storage, 72), // 3 days default TTL
        }
    }
    
    /// Cache DSL generation result
    pub async fn cache_dsl(
        &self,
        user_prompt: &str,
        generated_dsl: String,
        confidence_score: f32,
    ) -> Result<()> {
        let cached_dsl = CachedDSL {
            user_prompt: user_prompt.to_string(),
            generated_dsl,
            confidence_score,
            cached_at: Utc::now(),
        };
        
        let cache_key = self.generate_prompt_key(user_prompt);
        self.cache.set(&cache_key, &cached_dsl).await
    }
    
    /// Get cached DSL for a prompt
    pub async fn get_cached_dsl(&self, user_prompt: &str) -> Result<Option<CachedDSL>> {
        let cache_key = self.generate_prompt_key(user_prompt);
        self.cache.get(&cache_key).await
    }
    
    /// Generate cache key for user prompt
    fn generate_prompt_key(&self, prompt: &str) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(prompt.trim().to_lowercase().as_bytes());
        format!("dsl:{:x}", hasher.finalize())
    }
}
