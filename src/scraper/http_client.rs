use anyhow::Result;
use reqwest::{Client, Response, header::{HeaderMap, HeaderName, HeaderValue}};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, warn, info};
use url::Url;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::config::ScrapingConfig;

/// HTTP client wrapper with retry logic, connection pooling, and performance monitoring
pub struct HttpClient {
    client: Client,
    config: ScrapingConfig,
    performance_metrics: Arc<RwLock<HttpPerformanceMetrics>>,
    domain_limits: Arc<RwLock<HashMap<String, DomainLimits>>>,
}

/// Performance metrics for HTTP operations
#[derive(Debug)]
struct HttpPerformanceMetrics {
    request_durations: Vec<Duration>,
    success_count: u64,
    error_count: u64,
    total_bytes_transferred: u64,
    last_reset: Instant,
}

impl Default for HttpPerformanceMetrics {
    fn default() -> Self {
        Self {
            request_durations: Vec::new(),
            success_count: 0,
            error_count: 0,
            total_bytes_transferred: 0,
            last_reset: Instant::now(),
        }
    }
}

/// Rate limiting information for a domain
#[derive(Debug, Clone)]
struct DomainLimits {
    last_request: Instant,
    request_count: u32,
    window_start: Instant,
}

impl HttpClient {
    /// Create new HTTP client with optimized settings
    pub async fn new(config: &ScrapingConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"));
        headers.insert("Accept-Language", HeaderValue::from_static("en-US,en;q=0.5"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip, deflate, br"));
        headers.insert("DNT", HeaderValue::from_static("1"));
        headers.insert("Connection", HeaderValue::from_static("keep-alive"));
        headers.insert("Upgrade-Insecure-Requests", HeaderValue::from_static("1"));
        
        // Create optimized HTTP client with connection pooling
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .connect_timeout(Duration::from_secs(10))
            .default_headers(headers)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .http2_prior_knowledge() // Enable HTTP/2 for better performance
            .pool_max_idle_per_host(config.max_concurrent_requests as usize)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()?;
        
        info!("HTTP client initialized with connection pooling and HTTP/2 support");
        
        Ok(Self {
            client,
            config: config.clone(),
            performance_metrics: Arc::new(RwLock::new(HttpPerformanceMetrics::default())),
            domain_limits: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Check and enforce rate limiting for a domain
    async fn check_rate_limit(&self, domain: &str) -> Result<()> {
        let mut limits = self.domain_limits.write().await;
        let now = Instant::now();
        
        if let Some(domain_limit) = limits.get_mut(domain) {
            // Reset window if needed
            if now.duration_since(domain_limit.window_start) > Duration::from_secs(60) {
                domain_limit.window_start = now;
                domain_limit.request_count = 0;
            }
            
            // Check if we're within rate limits
            if domain_limit.request_count >= self.config.max_concurrent_requests as u32 {
                let wait_time = Duration::from_secs(60) - now.duration_since(domain_limit.window_start);
                if wait_time > Duration::from_secs(0) {
                    warn!("Rate limit exceeded for domain {}, waiting {}ms", domain, wait_time.as_millis());
                    tokio::time::sleep(wait_time).await;
                    domain_limit.window_start = Instant::now();
                    domain_limit.request_count = 0;
                }
            }
            
            domain_limit.request_count += 1;
            domain_limit.last_request = now;
        } else {
            // First request to this domain
            limits.insert(domain.to_string(), DomainLimits {
                last_request: now,
                request_count: 1,
                window_start: now,
            });
        }
        
        Ok(())
    }
    
    /// Record performance metrics for a request
    async fn record_request_metrics(&self, duration: Duration, success: bool, bytes: u64) {
        let mut metrics = self.performance_metrics.write().await;
        
        metrics.request_durations.push(duration);
        if success {
            metrics.success_count += 1;
        } else {
            metrics.error_count += 1;
        }
        metrics.total_bytes_transferred += bytes;
        
        // Keep only last 1000 durations to prevent memory growth
        if metrics.request_durations.len() > 1000 {
            metrics.request_durations.drain(0..500);
        }
    }
    
    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> HttpPerformanceStats {
        let metrics = self.performance_metrics.read().await;
        
        let avg_duration = if !metrics.request_durations.is_empty() {
            metrics.request_durations.iter().sum::<Duration>() / metrics.request_durations.len() as u32
        } else {
            Duration::from_secs(0)
        };
        
        let success_rate = if metrics.success_count + metrics.error_count > 0 {
            metrics.success_count as f64 / (metrics.success_count + metrics.error_count) as f64
        } else {
            0.0
        };
        
        HttpPerformanceStats {
            total_requests: metrics.success_count + metrics.error_count,
            success_count: metrics.success_count,
            error_count: metrics.error_count,
            success_rate,
            avg_response_time: avg_duration,
            total_bytes_transferred: metrics.total_bytes_transferred,
        }
    }
    
    /// Make GET request with retry logic and performance monitoring
    pub async fn get(
        &self,
        url: &Url,
        user_agent: &str,
        custom_headers: &Option<HashMap<String, String>>,
    ) -> Result<Response> {
        let start_time = Instant::now();
        let domain = url.host_str().unwrap_or("unknown");
        
        // Check rate limiting
        self.check_rate_limit(domain).await?;
        
        let mut last_error = None;
        let mut total_bytes = 0u64;
        
        for attempt in 1..=self.config.max_retries {
            debug!("HTTP GET attempt {} for: {}", attempt, url);
            
            match self.make_request(url, user_agent, custom_headers).await {
                Ok(response) => {
                    // Get content length for metrics
                    if let Some(content_length) = response.headers().get("content-length") {
                        if let Ok(length_str) = content_length.to_str() {
                            total_bytes = length_str.parse().unwrap_or(0);
                        }
                    }
                    
                    if response.status().is_success() {
                        debug!("HTTP GET successful for: {}", url);
                        let duration = start_time.elapsed();
                        self.record_request_metrics(duration, true, total_bytes).await;
                        return Ok(response);
                    } else if response.status().is_server_error() && attempt < self.config.max_retries {
                        warn!("Server error {} for {}, retrying...", response.status(), url);
                        last_error = Some(anyhow::anyhow!("Server error: {}", response.status()));
                    } else {
                        let duration = start_time.elapsed();
                        self.record_request_metrics(duration, false, total_bytes).await;
                        return Ok(response); // Return non-success responses for handling upstream
                    }
                }
                Err(e) => {
                    warn!("HTTP request failed for {} (attempt {}): {}", url, attempt, e);
                    last_error = Some(e);
                    
                    if attempt < self.config.max_retries {
                        let delay = Duration::from_secs(self.config.retry_delay_seconds * attempt as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        let duration = start_time.elapsed();
        self.record_request_metrics(duration, false, total_bytes).await;
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }
    
    /// Make single HTTP request
    async fn make_request(
        &self,
        url: &Url,
        user_agent: &str,
        custom_headers: &Option<HashMap<String, String>>,
    ) -> Result<Response> {
        let mut request = self.client.get(url.as_str())
            .header("User-Agent", user_agent);
        
        // Add custom headers if provided
        if let Some(headers) = custom_headers {
            for (name, value) in headers {
                if let (Ok(header_name), Ok(header_value)) = (
                    HeaderName::from_bytes(name.as_bytes()),
                    HeaderValue::from_str(value)
                ) {
                    request = request.header(header_name, header_value);
                }
            }
        }
        
        let response = request.send().await?;
        Ok(response)
    }
    
    /// Get robots.txt for a domain
    pub async fn get_robots_txt(&self, domain: &str, user_agent: &str) -> Result<String> {
        let robots_url = format!("https://{}/robots.txt", domain);
        let url = Url::parse(&robots_url)?;
        
        match self.get(&url, user_agent, &None).await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(response.text().await?)
                } else {
                    // If robots.txt doesn't exist, assume everything is allowed
                    Ok(String::new())
                }
            }
            Err(_) => {
                // If we can't fetch robots.txt, assume everything is allowed
                Ok(String::new())
            }
        }
    }
    
    /// Check if URL is reachable
    pub async fn check_url(&self, url: &Url, user_agent: &str) -> Result<bool> {
        match self.client.head(url.as_str())
            .header("User-Agent", user_agent)
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
    
    /// Get response headers for a URL
    pub async fn get_headers(&self, url: &Url, user_agent: &str) -> Result<HeaderMap> {
        let response = self.client.head(url.as_str())
            .header("User-Agent", user_agent)
            .send()
            .await?;
        
        Ok(response.headers().clone())
    }
}

/// HTTP performance statistics
#[derive(Debug, Clone)]
pub struct HttpPerformanceStats {
    pub total_requests: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub avg_response_time: Duration,
    pub total_bytes_transferred: u64,
}
