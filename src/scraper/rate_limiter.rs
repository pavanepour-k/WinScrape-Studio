use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// Rate limiter for controlling request frequency per domain
pub struct RateLimiter {
    domain_limits: Arc<RwLock<HashMap<String, DomainLimiter>>>,
    default_delay: Duration,
}

/// Per-domain rate limiting state
struct DomainLimiter {
    last_request: Instant,
    request_count: usize,
    window_start: Instant,
    delay: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            domain_limits: Arc::new(RwLock::new(HashMap::new())),
            default_delay: Duration::from_millis(1000),
        }
    }
    
    /// Wait for rate limit before making request to domain
    pub async fn wait_for_domain(&self, domain: &str) {
        let now = Instant::now();
        let required_delay = {
            let mut limits = self.domain_limits.write().await;
            let limiter = limits.entry(domain.to_string()).or_insert_with(|| {
                DomainLimiter {
                    last_request: now - self.default_delay,
                    request_count: 0,
                    window_start: now,
                    delay: self.default_delay,
                }
            });
            
            // Calculate required delay
            let time_since_last = now.duration_since(limiter.last_request);
            let required_delay = if time_since_last < limiter.delay {
                limiter.delay - time_since_last
            } else {
                Duration::from_millis(0)
            };
            
            // Update state
            limiter.last_request = now + required_delay;
            limiter.request_count += 1;
            
            // Reset window if needed (1 minute windows)
            if now.duration_since(limiter.window_start) > Duration::from_secs(60) {
                limiter.request_count = 1;
                limiter.window_start = now;
            }
            
            required_delay
        };
        
        if required_delay > Duration::from_millis(0) {
            debug!("Rate limiting: waiting {}ms for domain {}", required_delay.as_millis(), domain);
            tokio::time::sleep(required_delay).await;
        }
    }
    
    /// Set custom delay for a domain
    pub async fn set_domain_delay(&self, domain: &str, delay: Duration) {
        let mut limits = self.domain_limits.write().await;
        let limiter = limits.entry(domain.to_string()).or_insert_with(|| {
            DomainLimiter {
                last_request: Instant::now() - delay,
                request_count: 0,
                window_start: Instant::now(),
                delay,
            }
        });
        limiter.delay = delay;
    }
    
    /// Get current request count for domain in current window
    pub async fn get_domain_request_count(&self, domain: &str) -> usize {
        let limits = self.domain_limits.read().await;
        limits.get(domain).map(|l| l.request_count).unwrap_or(0)
    }
    
    /// Reset rate limiting state for a domain
    pub async fn reset_domain(&self, domain: &str) {
        let mut limits = self.domain_limits.write().await;
        limits.remove(domain);
    }
    
    /// Get statistics for all domains
    pub async fn get_stats(&self) -> HashMap<String, (usize, Duration)> {
        let limits = self.domain_limits.read().await;
        limits.iter()
            .map(|(domain, limiter)| {
                (domain.clone(), (limiter.request_count, limiter.delay))
            })
            .collect()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
