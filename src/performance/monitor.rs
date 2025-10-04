use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, debug};
use sysinfo::{System, Pid};

use super::metrics::PerformanceMetrics;

/// Performance monitor that collects system and application metrics
pub struct PerformanceMonitor {
    enabled: bool,
    system: Arc<RwLock<System>>,
    start_time: Instant,
    last_collection: Arc<RwLock<Instant>>,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub async fn new(enabled: bool) -> Result<Self> {
        let mut system = System::new_all();
        system.refresh_all();
        
        Ok(Self {
            enabled,
            system: Arc::new(RwLock::new(system)),
            start_time: Instant::now(),
            last_collection: Arc::new(RwLock::new(Instant::now())),
        })
    }
    
    /// Collect current performance metrics
    pub async fn collect_metrics(&self) -> Result<PerformanceMetrics> {
        if !self.enabled {
            return Ok(PerformanceMetrics::new());
        }
        
        let mut system = self.system.write().await;
        system.refresh_all();
        
        let mut metrics = PerformanceMetrics::new();
        
        // System metrics
        metrics.cpu_usage_percent = system.global_cpu_info().cpu_usage() as f64;
        metrics.memory_usage_mb = (system.used_memory() as f64) / (1024.0 * 1024.0);
        metrics.total_memory_mb = (system.total_memory() as f64) / (1024.0 * 1024.0);
        metrics.disk_usage_percent = self.calculate_disk_usage().await?;
        
        // Process-specific metrics
        if let Some(process) = self.find_current_process(&system).await {
            metrics.process_memory_mb = (process.memory() as f64) / (1024.0 * 1024.0);
            metrics.process_cpu_usage = process.cpu_usage() as f64;
        }
        
        // Network metrics - simplified for now
        metrics.network_bytes_sent = 0;
        metrics.network_bytes_received = 0;
        
        // Application-specific metrics
        metrics.uptime_seconds = self.start_time.elapsed().as_secs() as f64;
        
        // Update last collection time
        *self.last_collection.write().await = Instant::now();
        
        debug!("Collected performance metrics: CPU: {:.1}%, Memory: {:.1}MB", 
               metrics.cpu_usage_percent, metrics.memory_usage_mb);
        
        Ok(metrics)
    }
    
    /// Find the current process in the system
    async fn find_current_process<'a>(&self, system: &'a System) -> Option<&'a sysinfo::Process> {
        let current_pid = Pid::from_u32(std::process::id());
        system.process(current_pid)
    }
    
    /// Calculate disk usage percentage
    async fn calculate_disk_usage(&self) -> Result<f64> {
        // This is a simplified implementation
        // In a real implementation, you'd use a proper disk usage library
        Ok(0.0)
    }
    
    /// Get system information
    pub async fn get_system_info(&self) -> SystemInfo {
        let system = self.system.read().await;
        
        SystemInfo {
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
            host_name: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            cpu_count: system.cpus().len(),
            total_memory: system.total_memory(),
            uptime: System::uptime(),
        }
    }
    
    /// Check if monitoring is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Enable or disable monitoring
    pub async fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            info!("Performance monitoring enabled");
        } else {
            info!("Performance monitoring disabled");
        }
    }
    
    /// Get monitoring statistics
    pub async fn get_monitoring_stats(&self) -> MonitoringStats {
        let last_collection = *self.last_collection.read().await;
        let time_since_last = last_collection.elapsed();
        
        MonitoringStats {
            enabled: self.enabled,
            start_time: self.start_time,
            last_collection,
            time_since_last_collection: time_since_last,
            collection_count: 0, // This would be tracked in a real implementation
        }
    }
}

/// System information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub host_name: String,
    pub cpu_count: usize,
    pub total_memory: u64,
    pub uptime: u64,
}

/// Monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub enabled: bool,
    pub start_time: Instant,
    pub last_collection: Instant,
    pub time_since_last_collection: Duration,
    pub collection_count: u64,
}
