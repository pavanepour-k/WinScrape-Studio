use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::debug;

/// Performance metrics collected from various system components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // System metrics
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub total_memory_mb: f64,
    pub disk_usage_percent: f64,
    
    // Process metrics
    pub process_memory_mb: f64,
    pub process_cpu_usage: f64,
    
    // Network metrics
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    
    // Application metrics
    pub uptime_seconds: f64,
    pub request_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub avg_response_time: Duration,
    pub cache_hit_rate: f64,
    pub cache_miss_rate: f64,
    pub error_rate: f64,
    
    // Database metrics
    pub db_connection_count: u32,
    pub db_query_count: u64,
    pub db_avg_query_time: Duration,
    pub db_cache_hit_rate: f64,
    
    // HTTP client metrics
    pub http_request_count: u64,
    pub http_success_count: u64,
    pub http_error_count: u64,
    pub http_avg_response_time: Duration,
    pub http_total_bytes_transferred: u64,
    
    // Custom metrics
    pub custom_metrics: HashMap<String, f64>,
    
    // Timestamps
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub collection_duration: Duration,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            total_memory_mb: 0.0,
            disk_usage_percent: 0.0,
            process_memory_mb: 0.0,
            process_cpu_usage: 0.0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            uptime_seconds: 0.0,
            request_count: 0,
            success_count: 0,
            error_count: 0,
            avg_response_time: Duration::from_secs(0),
            cache_hit_rate: 0.0,
            cache_miss_rate: 0.0,
            error_rate: 0.0,
            db_connection_count: 0,
            db_query_count: 0,
            db_avg_query_time: Duration::from_secs(0),
            db_cache_hit_rate: 0.0,
            http_request_count: 0,
            http_success_count: 0,
            http_error_count: 0,
            http_avg_response_time: Duration::from_secs(0),
            http_total_bytes_transferred: 0,
            custom_metrics: HashMap::new(),
            timestamp: chrono::Utc::now(),
            collection_duration: Duration::from_secs(0),
        }
    }
    
    /// Update metrics with new data
    pub fn update(&mut self, new_metrics: PerformanceMetrics) {
        let start_time = Instant::now();
        
        // Update system metrics
        self.cpu_usage_percent = new_metrics.cpu_usage_percent;
        self.memory_usage_mb = new_metrics.memory_usage_mb;
        self.total_memory_mb = new_metrics.total_memory_mb;
        self.disk_usage_percent = new_metrics.disk_usage_percent;
        
        // Update process metrics
        self.process_memory_mb = new_metrics.process_memory_mb;
        self.process_cpu_usage = new_metrics.process_cpu_usage;
        
        // Update network metrics
        self.network_bytes_sent = new_metrics.network_bytes_sent;
        self.network_bytes_received = new_metrics.network_bytes_received;
        
        // Update application metrics
        self.uptime_seconds = new_metrics.uptime_seconds;
        
        // Update HTTP metrics
        self.http_request_count = new_metrics.http_request_count;
        self.http_success_count = new_metrics.http_success_count;
        self.http_error_count = new_metrics.http_error_count;
        self.http_avg_response_time = new_metrics.http_avg_response_time;
        self.http_total_bytes_transferred = new_metrics.http_total_bytes_transferred;
        
        // Calculate derived metrics
        self.calculate_derived_metrics();
        
        // Update timestamp
        self.timestamp = chrono::Utc::now();
        self.collection_duration = start_time.elapsed();
        
        debug!("Updated performance metrics in {:?}", self.collection_duration);
    }
    
    /// Calculate derived metrics from base metrics
    fn calculate_derived_metrics(&mut self) {
        // Calculate error rate
        let total_requests = self.http_success_count + self.http_error_count;
        if total_requests > 0 {
            self.error_rate = self.http_error_count as f64 / total_requests as f64;
        } else {
            self.error_rate = 0.0;
        }
        
        // Calculate cache hit rate
        let total_cache_requests = self.cache_hit_rate + self.cache_miss_rate;
        if total_cache_requests > 0.0 {
            self.cache_hit_rate = self.cache_hit_rate / total_cache_requests;
            self.cache_miss_rate = self.cache_miss_rate / total_cache_requests;
        }
        
        // Calculate overall request metrics
        self.request_count = self.http_request_count;
        self.success_count = self.http_success_count;
        self.error_count = self.http_error_count;
        self.avg_response_time = self.http_avg_response_time;
    }
    
    /// Calculate overall performance score (0.0 to 1.0)
    pub fn calculate_performance_score(&self) -> f64 {
        let mut score: f64 = 1.0;
        
        // CPU usage penalty
        if self.cpu_usage_percent > 80.0 {
            score -= 0.3;
        } else if self.cpu_usage_percent > 60.0 {
            score -= 0.2;
        } else if self.cpu_usage_percent > 40.0 {
            score -= 0.1;
        }
        
        // Memory usage penalty
        let memory_usage_percent = if self.total_memory_mb > 0.0 {
            self.memory_usage_mb / self.total_memory_mb
        } else {
            0.0
        };
        
        if memory_usage_percent > 0.9 {
            score -= 0.3;
        } else if memory_usage_percent > 0.8 {
            score -= 0.2;
        } else if memory_usage_percent > 0.7 {
            score -= 0.1;
        }
        
        // Error rate penalty
        if self.error_rate > 0.1 {
            score -= 0.3;
        } else if self.error_rate > 0.05 {
            score -= 0.2;
        } else if self.error_rate > 0.01 {
            score -= 0.1;
        }
        
        // Response time penalty
        if self.avg_response_time > Duration::from_secs(5) {
            score -= 0.2;
        } else if self.avg_response_time > Duration::from_secs(2) {
            score -= 0.1;
        }
        
        // Cache hit rate bonus
        if self.cache_hit_rate > 0.9 {
            score += 0.1;
        } else if self.cache_hit_rate > 0.8 {
            score += 0.05;
        }
        
        // Ensure score is between 0.0 and 1.0
        score.max(0.0_f64).min(1.0_f64)
    }
    
    /// Get memory usage percentage
    pub fn get_memory_usage_percent(&self) -> f64 {
        if self.total_memory_mb > 0.0 {
            self.memory_usage_mb / self.total_memory_mb
        } else {
            0.0
        }
    }
    
    /// Get network throughput in MB/s
    pub fn get_network_throughput_mbps(&self) -> f64 {
        let total_bytes = self.network_bytes_sent + self.network_bytes_received;
        (total_bytes as f64) / (1024.0 * 1024.0) / (self.uptime_seconds.max(1.0))
    }
    
    /// Check if metrics indicate performance issues
    pub fn has_performance_issues(&self) -> bool {
        self.cpu_usage_percent > 80.0 ||
        self.get_memory_usage_percent() > 0.9 ||
        self.error_rate > 0.1 ||
        self.avg_response_time > Duration::from_secs(5) ||
        self.cache_hit_rate < 0.5
    }
    
    /// Get performance issues as a list
    pub fn get_performance_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        
        if self.cpu_usage_percent > 80.0 {
            issues.push(format!("High CPU usage: {:.1}%", self.cpu_usage_percent));
        }
        
        let memory_percent = self.get_memory_usage_percent();
        if memory_percent > 0.9 {
            issues.push(format!("High memory usage: {:.1}%", memory_percent * 100.0));
        }
        
        if self.error_rate > 0.1 {
            issues.push(format!("High error rate: {:.1}%", self.error_rate * 100.0));
        }
        
        if self.avg_response_time > Duration::from_secs(5) {
            issues.push(format!("Slow response time: {:?}", self.avg_response_time));
        }
        
        if self.cache_hit_rate < 0.5 {
            issues.push(format!("Low cache hit rate: {:.1}%", self.cache_hit_rate * 100.0));
        }
        
        issues
    }
    
    /// Add custom metric
    pub fn add_custom_metric(&mut self, name: String, value: f64) {
        self.custom_metrics.insert(name, value);
    }
    
    /// Get custom metric
    pub fn get_custom_metric(&self, name: &str) -> Option<f64> {
        self.custom_metrics.get(name).copied()
    }
    
    /// Get summary of key metrics
    pub fn get_summary(&self) -> PerformanceSummary {
        PerformanceSummary {
            performance_score: self.calculate_performance_score(),
            cpu_usage: self.cpu_usage_percent,
            memory_usage_percent: self.get_memory_usage_percent(),
            error_rate: self.error_rate,
            avg_response_time: self.avg_response_time,
            cache_hit_rate: self.cache_hit_rate,
            has_issues: self.has_performance_issues(),
            issues: self.get_performance_issues(),
        }
    }
}

/// Performance summary for quick overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub performance_score: f64,
    pub cpu_usage: f64,
    pub memory_usage_percent: f64,
    pub error_rate: f64,
    pub avg_response_time: Duration,
    pub cache_hit_rate: f64,
    pub has_issues: bool,
    pub issues: Vec<String>,
}
