use anyhow::Result;
use scraper::{Html, Selector, ElementRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{info, warn, error, debug};
use url::Url;

pub mod http_client;
pub mod browser;
pub mod robots;
pub mod rate_limiter;
pub mod user_agent;

use crate::config::ScrapingConfig;
use crate::dsl::{ScrapePlan, Field, SelectorType, ExtractionMethod, Transform};

/// Main scraping engine
pub struct ScrapingEngine {
    config: ScrapingConfig,
    http_client: Arc<http_client::HttpClient>,
    #[cfg(feature = "browser")]
    browser_client: Option<Arc<browser::BrowserClient>>,
    robots_checker: Arc<robots::RobotsChecker>,
    rate_limiter: Arc<rate_limiter::RateLimiter>,
    user_agent_rotator: Arc<user_agent::UserAgentRotator>,
    semaphore: Arc<Semaphore>,
}

/// Scraping result for a single item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedItem {
    pub data: HashMap<String, serde_json::Value>,
    pub metadata: ItemMetadata,
}

/// Metadata for scraped items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub source_url: String,
    pub scraped_at: chrono::DateTime<chrono::Utc>,
    pub method: ScrapingMethod,
    pub response_time_ms: u64,
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScrapingMethod {
    Http,
    Browser,
}

impl ScrapingEngine {
    /// Create new scraping engine
    pub async fn new(config: &ScrapingConfig) -> Result<Self> {
        info!("Initializing scraping engine");
        
        let http_client = Arc::new(http_client::HttpClient::new(config).await?);
        
        #[cfg(feature = "browser")]
        let browser_client = if config.enable_browser_fallback {
            Some(Arc::new(browser::BrowserClient::new(config).await?))
        } else {
            None
        };
        
        let robots_checker = Arc::new(robots::RobotsChecker::new(http_client.clone()));
        let rate_limiter = Arc::new(rate_limiter::RateLimiter::new());
        let user_agent_rotator = Arc::new(user_agent::UserAgentRotator::new(&config.user_agents));
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));
        
        Ok(Self {
            config: config.clone(),
            http_client,
            #[cfg(feature = "browser")]
            browser_client,
            robots_checker,
            rate_limiter,
            user_agent_rotator,
            semaphore,
        })
    }
    
    /// Execute scraping based on DSL plan
    pub async fn execute_scraping(&self, plan: &ScrapePlan) -> Result<Vec<serde_json::Value>> {
        info!("Starting scraping execution for domain: {}", plan.target.domain);
        
        // Check robots.txt if required
        if plan.anti_blocking.respect_robots_txt {
            self.check_robots_compliance(plan).await?;
        }
        
        // Get all URLs to scrape
        let urls = plan.get_all_urls()?;
        info!("Found {} URLs to scrape", urls.len());
        
        let mut all_results = Vec::new();
        
        // Process URLs with concurrency control
        let mut tasks = Vec::new();
        
        for url in urls {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let engine = self.clone_for_task();
            let plan = plan.clone();
            
            // Create a future without spawning to avoid Send trait bound issues with scraper crate
            let task = async move {
                let _permit = permit; // Keep permit alive
                engine.scrape_single_url(&url, &plan).await
            };
            
            tasks.push(task);
        }
        
        // Execute all tasks concurrently using join_all
        use futures::future::join_all;
        let results = join_all(tasks).await;
        
        // Collect results
        for result in results {
            match result {
                Ok(mut data) => {
                    all_results.append(&mut data);
                }
                Err(e) => {
                    error!("Failed to scrape URL: {}", e);
                    // Continue with other URLs
                }
            }
        }
        
        info!("Scraping completed. Total items: {}", all_results.len());
        
        // Apply output limits if specified
        if let Some(limit) = plan.output.limit {
            all_results.truncate(limit);
        }
        
        Ok(all_results)
    }
    
    /// Generate preview with limited results
    pub async fn generate_preview(&self, plan: &ScrapePlan, limit: usize) -> Result<Vec<serde_json::Value>> {
        info!("Generating preview with {} items", limit);
        
        // Use only the first start URL for preview
        if let Some(first_url) = plan.target.start_urls.first() {
            let url = Url::parse(first_url)?;
            let mut results = self.scrape_single_url(&url, plan).await?;
            results.truncate(limit);
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Scrape a single URL
    async fn scrape_single_url(&self, url: &Url, plan: &ScrapePlan) -> Result<Vec<serde_json::Value>> {
        debug!("Scraping URL: {}", url);
        
        // Apply rate limiting
        self.rate_limiter.wait_for_domain(url.host_str().unwrap_or("")).await;
        
        // Apply delay
        let delay = self.calculate_delay(&plan.anti_blocking.randomized_delays);
        tokio::time::sleep(Duration::from_millis(delay)).await;
        
        // Try HTTP first
        match self.scrape_with_http(url, plan).await {
            Ok(results) => {
                debug!("HTTP scraping successful for {}", url);
                Ok(results)
            }
            Err(e) => {
                warn!("HTTP scraping failed for {}: {}", url, e);
                
                // Fallback to browser if enabled
                #[cfg(feature = "browser")]
                if self.config.enable_browser_fallback {
                    if let Some(browser) = &self.browser_client {
                        info!("Falling back to browser for {}", url);
                        return browser.scrape_url(url, plan).await;
                    }
                }
                
                Err(e)
            }
        }
    }
    
    /// Scrape using HTTP client
    async fn scrape_with_http(&self, url: &Url, plan: &ScrapePlan) -> Result<Vec<serde_json::Value>> {
        let start_time = std::time::Instant::now();
        
        // Get user agent
        let user_agent = self.user_agent_rotator.get_random_user_agent();
        
        // Make HTTP request
        let response = self.http_client.get(url, &user_agent, &plan.anti_blocking.headers).await?;
        let status_code = response.status().as_u16();
        let response_time = start_time.elapsed().as_millis() as u64;
        
        // Get response body
        let html_content = response.text().await?;
        
        // Parse HTML
        let document = Html::parse_document(&html_content);
        
        // Extract items
        let items = self.extract_items(&document, plan, url, status_code, response_time).await?;
        
        Ok(items)
    }
    
    /// Extract items from HTML document
    async fn extract_items(
        &self,
        document: &Html,
        plan: &ScrapePlan,
        source_url: &Url,
        status_code: u16,
        response_time: u64,
    ) -> Result<Vec<serde_json::Value>> {
        let item_selector = Selector::parse(&plan.rules.item_selector)
            .map_err(|e| anyhow::anyhow!("Invalid item selector: {}", e))?;
        
        let mut items = Vec::new();
        
        for element in document.select(&item_selector) {
            let mut item_data = HashMap::new();
            
            // Extract each field
            for field in &plan.rules.fields {
                match self.extract_field_value(&element, field, source_url).await {
                    Ok(Some(value)) => {
                        item_data.insert(field.name.clone(), value);
                    }
                    Ok(None) => {
                        if field.required {
                            debug!("Required field '{}' not found, skipping item", field.name);
                            continue;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to extract field '{}': {}", field.name, e);
                        if field.required {
                            continue;
                        }
                    }
                }
            }
            
            // Add metadata
            item_data.insert("_source_url".to_string(), serde_json::Value::String(source_url.to_string()));
            item_data.insert("_scraped_at".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            item_data.insert("_method".to_string(), serde_json::Value::String("http".to_string()));
            item_data.insert("_response_time_ms".to_string(), serde_json::Value::Number(response_time.into()));
            item_data.insert("_status_code".to_string(), serde_json::Value::Number(status_code.into()));
            
            // Apply filters
            if self.passes_filters(&item_data, &plan.rules.filters) {
                items.push(serde_json::Value::Object(
                    item_data.into_iter().map(|(k, v)| (k, v)).collect()
                ));
            }
        }
        
        debug!("Extracted {} items from {}", items.len(), source_url);
        Ok(items)
    }
    
    /// Extract value for a single field
    async fn extract_field_value(
        &self,
        element: &ElementRef<'_>,
        field: &Field,
        source_url: &Url,
    ) -> Result<Option<serde_json::Value>> {
        // Parse selector based on type
        let value = match field.selector_type {
            SelectorType::CSS => {
                let selector = Selector::parse(&field.selector)
                    .map_err(|e| anyhow::anyhow!("Invalid CSS selector '{}': {}", field.selector, e))?;
                
                if let Some(selected_element) = element.select(&selector).next() {
                    self.extract_value_by_method(&selected_element, &field.extraction, source_url)?
                } else {
                    return Ok(None);
                }
            }
            SelectorType::XPath => {
                // XPath support would require additional dependencies
                // For now, return an error
                return Err(anyhow::anyhow!("XPath selectors not yet implemented"));
            }
        };
        
        // Apply transformations
        let transformed_value = if let Some(transforms) = &field.transform {
            self.apply_transforms(value, transforms)?
        } else {
            value
        };
        
        Ok(Some(transformed_value))
    }
    
    /// Extract value using specified method
    fn extract_value_by_method(
        &self,
        element: &ElementRef<'_>,
        method: &ExtractionMethod,
        source_url: &Url,
    ) -> Result<serde_json::Value> {
        let value = match method {
            ExtractionMethod::Text => {
                element.text().collect::<Vec<_>>().join(" ").trim().to_string()
            }
            ExtractionMethod::Html => {
                element.html()
            }
            ExtractionMethod::Attribute { name } => {
                element.value().attr(name).unwrap_or("").to_string()
            }
            ExtractionMethod::Href => {
                let href = element.value().attr("href").unwrap_or("");
                // Convert relative URLs to absolute
                if let Ok(base_url) = Url::parse(&source_url.to_string()) {
                    if let Ok(absolute_url) = base_url.join(href) {
                        absolute_url.to_string()
                    } else {
                        href.to_string()
                    }
                } else {
                    href.to_string()
                }
            }
            ExtractionMethod::Src => {
                let src = element.value().attr("src").unwrap_or("");
                // Convert relative URLs to absolute
                if let Ok(base_url) = Url::parse(&source_url.to_string()) {
                    if let Ok(absolute_url) = base_url.join(src) {
                        absolute_url.to_string()
                    } else {
                        src.to_string()
                    }
                } else {
                    src.to_string()
                }
            }
        };
        
        Ok(serde_json::Value::String(value))
    }
    
    /// Apply transformations to a value
    fn apply_transforms(&self, mut value: serde_json::Value, transforms: &[Transform]) -> Result<serde_json::Value> {
        for transform in transforms {
            value = self.apply_single_transform(value, transform)?;
        }
        Ok(value)
    }
    
    /// Apply a single transformation
    fn apply_single_transform(&self, value: serde_json::Value, transform: &Transform) -> Result<serde_json::Value> {
        if let Some(text) = value.as_str() {
            let transformed = match transform {
                Transform::Trim => text.trim().to_string(),
                Transform::Lowercase => text.to_lowercase(),
                Transform::Uppercase => text.to_uppercase(),
                Transform::Regex { pattern, replacement } => {
                    let regex = regex::Regex::new(pattern)?;
                    regex.replace_all(text, replacement).to_string()
                }
                Transform::ParseNumber => {
                    // Try to parse as number
                    if let Ok(num) = text.parse::<f64>() {
                        return Ok(serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0))));
                    } else {
                        text.to_string()
                    }
                }
                Transform::ParseDate { format: _ } => {
                    // Date parsing would require chrono parsing
                    // For now, keep as string
                    text.to_string()
                }
                Transform::RemoveHtml => {
                    // Simple HTML tag removal
                    let regex = regex::Regex::new(r"<[^>]*>").unwrap();
                    regex.replace_all(text, "").to_string()
                }
                Transform::ExtractDomain => {
                    if let Ok(url) = Url::parse(text) {
                        url.host_str().unwrap_or(text).to_string()
                    } else {
                        text.to_string()
                    }
                }
            };
            
            Ok(serde_json::Value::String(transformed))
        } else {
            // Non-string values pass through unchanged
            Ok(value)
        }
    }
    
    /// Check if item passes all filters
    fn passes_filters(&self, item: &HashMap<String, serde_json::Value>, filters: &Option<Vec<crate::dsl::Filter>>) -> bool {
        if let Some(filters) = filters {
            for filter in filters {
                if let Some(field_value) = item.get(&filter.field) {
                    if !self.check_filter_condition(field_value, &filter.condition) {
                        return false;
                    }
                } else {
                    // Field doesn't exist, filter fails
                    return false;
                }
            }
        }
        true
    }
    
    /// Check if a value passes a filter condition
    fn check_filter_condition(&self, value: &serde_json::Value, condition: &crate::dsl::FilterCondition) -> bool {
        use crate::dsl::FilterCondition;
        
        match condition {
            FilterCondition::Contains { value: search } => {
                if let Some(text) = value.as_str() {
                    text.contains(search)
                } else {
                    false
                }
            }
            FilterCondition::NotContains { value: search } => {
                if let Some(text) = value.as_str() {
                    !text.contains(search)
                } else {
                    true
                }
            }
            FilterCondition::Equals { value: expected } => {
                if let Some(text) = value.as_str() {
                    text == expected
                } else {
                    false
                }
            }
            FilterCondition::NotEquals { value: expected } => {
                if let Some(text) = value.as_str() {
                    text != expected
                } else {
                    true
                }
            }
            FilterCondition::Regex { pattern } => {
                if let Some(text) = value.as_str() {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        regex.is_match(text)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            FilterCondition::LengthMin { min } => {
                if let Some(text) = value.as_str() {
                    text.len() >= *min
                } else {
                    false
                }
            }
            FilterCondition::LengthMax { max } => {
                if let Some(text) = value.as_str() {
                    text.len() <= *max
                } else {
                    false
                }
            }
            FilterCondition::NotEmpty => {
                if let Some(text) = value.as_str() {
                    !text.trim().is_empty()
                } else {
                    !value.is_null()
                }
            }
        }
    }
    
    /// Check robots.txt compliance
    async fn check_robots_compliance(&self, plan: &ScrapePlan) -> Result<()> {
        for url_str in &plan.target.start_urls {
            let url = Url::parse(url_str)?;
            if !self.robots_checker.is_allowed(&url, "*").await? {
                return Err(anyhow::anyhow!("Robots.txt disallows access to: {}", url));
            }
        }
        Ok(())
    }
    
    /// Calculate delay based on configuration
    fn calculate_delay(&self, delay_config: &crate::dsl::DelayConfig) -> u64 {
        use crate::dsl::DelayDistribution;
        use rand::Rng;
        
        let mut rng = rand::thread_rng();
        
        match delay_config.distribution {
            DelayDistribution::Uniform => {
                rng.gen_range(delay_config.min_ms..=delay_config.max_ms)
            }
            DelayDistribution::Exponential => {
                // Simple exponential distribution approximation
                let lambda = 1.0 / ((delay_config.min_ms + delay_config.max_ms) as f64 / 2.0);
                let exp_value = (-rng.gen::<f64>().ln() / lambda) as u64;
                exp_value.clamp(delay_config.min_ms, delay_config.max_ms)
            }
            DelayDistribution::Normal => {
                // Simple normal distribution approximation
                let mean = (delay_config.min_ms + delay_config.max_ms) as f64 / 2.0;
                let std_dev = (delay_config.max_ms - delay_config.min_ms) as f64 / 6.0;
                let normal_value = mean + std_dev * rng.gen::<f64>();
                (normal_value as u64).clamp(delay_config.min_ms, delay_config.max_ms)
            }
        }
    }
    
    /// Clone for async task usage
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            http_client: self.http_client.clone(),
            #[cfg(feature = "browser")]
            browser_client: self.browser_client.clone(),
            robots_checker: self.robots_checker.clone(),
            rate_limiter: self.rate_limiter.clone(),
            user_agent_rotator: self.user_agent_rotator.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 5,
            request_timeout_seconds: 30,
            max_retries: 3,
            retry_delay_seconds: 2,
            respect_robots_txt: true,
            default_delay_ms: 1000,
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            ],
            enable_browser_fallback: false,
            browser_timeout_seconds: 60,
        }
    }
}
