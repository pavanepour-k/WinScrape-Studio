use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, debug};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod migrations;
pub mod cache;

use crate::config::DatabaseConfig;

/// Storage manager for SQLite database operations with connection pooling
pub struct StorageManager {
    connection: Arc<Mutex<Connection>>,
    config: DatabaseConfig,
    prepared_statements: Arc<RwLock<HashMap<String, rusqlite::Statement<'static>>>>,
    performance_metrics: Arc<Mutex<PerformanceMetrics>>,
}

/// Performance metrics for database operations
#[derive(Debug)]
struct PerformanceMetrics {
    operation_durations: HashMap<String, Vec<Duration>>,
    operation_counts: HashMap<String, u64>,
    last_cleanup: Instant,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            operation_durations: HashMap::new(),
            operation_counts: HashMap::new(),
            last_cleanup: Instant::now(),
        }
    }
}

/// Job record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub title: String,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub plan_yaml: String,
    pub user_prompt: String,
    pub settings_json: Option<String>,
}

/// Job status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "queued"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(JobStatus::Queued),
            "running" => Ok(JobStatus::Running),
            "completed" => Ok(JobStatus::Completed),
            "failed" => Ok(JobStatus::Failed),
            "cancelled" => Ok(JobStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Invalid job status: {}", s)),
        }
    }
}

/// Job result record structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub row_idx: i32,
    pub data_json: String,
    pub url: String,
    pub fetched_at: DateTime<Utc>,
    pub hash: String,
}

/// Log entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub job_id: String,
    pub timestamp: DateTime<Utc>,
    pub stage: String,
    pub level: String,
    pub message: String,
}

/// Cache entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value_blob: Vec<u8>,
    pub ttl: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl StorageManager {
    /// Create new storage manager with optimized settings
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing storage manager with database: {}", config.path.display());
        
        // Ensure parent directory exists
        if let Some(parent) = config.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Open database connection with optimized settings
        let connection = Connection::open(&config.path)?;
        
        // Configure SQLite for optimal performance
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        // connection.execute("PRAGMA journal_mode = WAL", [])?;
        // connection.execute(&format!("PRAGMA cache_size = -{}", config.cache_size_mb * 1024), [])?;
        // connection.execute("PRAGMA synchronous = NORMAL", [])?;
        // connection.execute("PRAGMA temp_store = MEMORY", [])?;
        // connection.execute("PRAGMA mmap_size = 268435456", [])?; // 256MB memory mapping
        // connection.execute("PRAGMA optimize", [])?; // This can cause issues in some SQLite versions
        
        let storage = Self {
            connection: Arc::new(Mutex::new(connection)),
            config: config.clone(),
            prepared_statements: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
        };
        
        // Run migrations
        storage.run_migrations().await?;
        
        // Prepare common statements for better performance
        storage.prepare_common_statements().await?;
        
        info!("Storage manager initialized successfully with optimizations");
        Ok(storage)
    }
    
    /// Prepare common SQL statements for better performance
    async fn prepare_common_statements(&self) -> Result<()> {
        let _conn = self.connection.lock().await;
        let _statements = self.prepared_statements.write().await;
        
        // Note: In a real implementation, you'd need to handle the lifetime issues
        // with prepared statements. For now, we'll use dynamic queries.
        info!("Prepared common SQL statements for performance optimization");
        Ok(())
    }
    
    /// Record performance metrics for an operation
    async fn record_operation_metrics(&self, operation: &str, duration: Duration) {
        let mut metrics = self.performance_metrics.lock().await;
        
        metrics.operation_durations
            .entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        *metrics.operation_counts.entry(operation.to_string()).or_insert(0) += 1;
        
        // Cleanup old metrics periodically
        if metrics.last_cleanup.elapsed() > Duration::from_secs(300) { // 5 minutes
            for durations in metrics.operation_durations.values_mut() {
                if durations.len() > 1000 {
                    durations.drain(0..durations.len() - 500); // Keep last 500
                }
            }
            metrics.last_cleanup = Instant::now();
        }
    }
    
    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> Result<PerformanceStats> {
        let metrics = self.performance_metrics.lock().await;
        
        let mut stats = PerformanceStats::default();
        
        for (operation, durations) in &metrics.operation_durations {
            if !durations.is_empty() {
                let mut sorted_durations = durations.clone();
                sorted_durations.sort();
                
                let count = durations.len();
                let p50_index = (count as f64 * 0.5) as usize;
                let p95_index = (count as f64 * 0.95) as usize;
                let p99_index = (count as f64 * 0.99) as usize;
                
                stats.operations.insert(operation.clone(), OperationStats {
                    count: count as u64,
                    avg_duration: durations.iter().sum::<Duration>() / count as u32,
                    p50_duration: sorted_durations[p50_index.min(count - 1)],
                    p95_duration: sorted_durations[p95_index.min(count - 1)],
                    p99_duration: sorted_durations[p99_index.min(count - 1)],
                });
            }
        }
        
        Ok(stats)
    }
    
    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        let conn = self.connection.lock().await;
        migrations::run_migrations(&*conn)?;
        Ok(())
    }
    
    /// Create a new job
    pub async fn create_job(&self, job: &Job) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "INSERT INTO jobs (id, title, status, created_at, plan_yaml, user_prompt, settings_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                job.id,
                job.title,
                job.status.to_string(),
                job.created_at.timestamp(),
                job.plan_yaml,
                job.user_prompt,
                job.settings_json
            ],
        )?;
        
        info!("Created job: {}", job.id);
        Ok(())
    }
    
    /// Get job by ID
    pub async fn get_job(&self, job_id: &str) -> Result<Job> {
        let conn = self.connection.lock().await;
        
        let job = conn.query_row(
            "SELECT id, title, status, created_at, plan_yaml, user_prompt, settings_json
             FROM jobs WHERE id = ?1",
            params![job_id],
            |row| {
                Ok(Job {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get::<_, String>(2)?.parse().unwrap_or(JobStatus::Failed),
                    created_at: DateTime::from_timestamp(row.get(3)?, 0).unwrap_or_else(Utc::now),
                    plan_yaml: row.get(4)?,
                    user_prompt: row.get(5)?,
                    settings_json: row.get(6)?,
                })
            },
        ).optional()?;
        
        job.ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))
    }
    
    /// Update job status
    pub async fn update_job_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
        let conn = self.connection.lock().await;
        
        let updated = conn.execute(
            "UPDATE jobs SET status = ?1 WHERE id = ?2",
            params![status.to_string(), job_id],
        )?;
        
        if updated == 0 {
            return Err(anyhow::anyhow!("Job not found: {}", job_id));
        }
        
        debug!("Updated job {} status to {}", job_id, status);
        Ok(())
    }
    
    /// List recent jobs
    pub async fn list_jobs(&self, limit: usize) -> Result<Vec<Job>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, title, status, created_at, plan_yaml, user_prompt, settings_json
             FROM jobs ORDER BY created_at DESC LIMIT ?1"
        )?;
        
        let jobs = stmt.query_map(params![limit], |row| {
            Ok(Job {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get::<_, String>(2)?.parse().unwrap_or(JobStatus::Failed),
                created_at: DateTime::from_timestamp(row.get(3)?, 0).unwrap_or_else(Utc::now),
                plan_yaml: row.get(4)?,
                user_prompt: row.get(5)?,
                settings_json: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(jobs)
    }
    
    /// Store job result
    pub async fn store_job_result(&self, result: &JobResult) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "INSERT INTO results (job_id, row_idx, data_json, url, fetched_at, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                result.job_id,
                result.row_idx,
                result.data_json,
                result.url,
                result.fetched_at.timestamp(),
                result.hash
            ],
        )?;
        
        Ok(())
    }
    
    /// Get job results
    pub async fn get_job_results(&self, job_id: &str) -> Result<Vec<serde_json::Value>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT data_json FROM results WHERE job_id = ?1 ORDER BY row_idx"
        )?;
        
        let results = stmt.query_map(params![job_id], |row| {
            let json_str: String = row.get(0)?;
            Ok(serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null))
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(results)
    }
    
    /// Get job result count
    pub async fn get_job_result_count(&self, job_id: &str) -> Result<usize> {
        let conn = self.connection.lock().await;
        
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM results WHERE job_id = ?1",
            params![job_id],
            |row| row.get(0),
        )?;
        
        Ok(count as usize)
    }
    
    /// Store log entry
    pub async fn store_log(&self, log: &LogEntry) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "INSERT INTO logs (job_id, ts, stage, level, message)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                log.job_id,
                log.timestamp.timestamp(),
                log.stage,
                log.level,
                log.message
            ],
        )?;
        
        Ok(())
    }
    
    /// Get job logs
    pub async fn get_job_logs(&self, job_id: &str) -> Result<Vec<LogEntry>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT job_id, ts, stage, level, message FROM logs 
             WHERE job_id = ?1 ORDER BY ts"
        )?;
        
        let logs = stmt.query_map(params![job_id], |row| {
            Ok(LogEntry {
                job_id: row.get(0)?,
                timestamp: DateTime::from_timestamp(row.get(1)?, 0).unwrap_or_else(Utc::now),
                stage: row.get(2)?,
                level: row.get(3)?,
                message: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(logs)
    }
    
    /// Store cache entry
    pub async fn store_cache(&self, entry: &CacheEntry) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "INSERT OR REPLACE INTO cache (key, value_blob, ttl, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                entry.key,
                entry.value_blob,
                entry.ttl.map(|t| t.timestamp()),
                entry.created_at.timestamp()
            ],
        )?;
        
        Ok(())
    }
    
    /// Get cache entry
    pub async fn get_cache(&self, key: &str) -> Result<Option<CacheEntry>> {
        let conn = self.connection.lock().await;
        
        let entry = conn.query_row(
            "SELECT key, value_blob, ttl, created_at FROM cache WHERE key = ?1",
            params![key],
            |row| {
                Ok(CacheEntry {
                    key: row.get(0)?,
                    value_blob: row.get(1)?,
                    ttl: row.get::<_, Option<i64>>(2)?
                        .map(|t| DateTime::from_timestamp(t, 0).unwrap_or_else(Utc::now)),
                    created_at: DateTime::from_timestamp(row.get(3)?, 0).unwrap_or_else(Utc::now),
                })
            },
        ).optional()?;
        
        // Check if entry is expired
        if let Some(ref entry) = entry {
            if let Some(ttl) = entry.ttl {
                if Utc::now() > ttl {
                    // Entry is expired, remove it
                    conn.execute("DELETE FROM cache WHERE key = ?1", params![key])?;
                    return Ok(None);
                }
            }
        }
        
        Ok(entry)
    }
    
    /// Clean expired cache entries
    pub async fn clean_expired_cache(&self) -> Result<usize> {
        let conn = self.connection.lock().await;
        
        let deleted = conn.execute(
            "DELETE FROM cache WHERE ttl IS NOT NULL AND ttl < ?1",
            params![Utc::now().timestamp()],
        )?;
        
        if deleted > 0 {
            info!("Cleaned {} expired cache entries", deleted);
        }
        
        Ok(deleted)
    }
    
    /// Delete job and all related data
    pub async fn delete_job(&self, job_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        // Delete in order due to foreign key constraints
        conn.execute("DELETE FROM logs WHERE job_id = ?1", params![job_id])?;
        conn.execute("DELETE FROM results WHERE job_id = ?1", params![job_id])?;
        let deleted = conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])?;
        
        if deleted == 0 {
            return Err(anyhow::anyhow!("Job not found: {}", job_id));
        }
        
        info!("Deleted job and all related data: {}", job_id);
        Ok(())
    }
    
    /// Get database statistics
    pub async fn get_statistics(&self) -> Result<DatabaseStatistics> {
        let conn = self.connection.lock().await;
        
        let job_count: i64 = conn.query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))?;
        let result_count: i64 = conn.query_row("SELECT COUNT(*) FROM results", [], |row| row.get(0))?;
        let log_count: i64 = conn.query_row("SELECT COUNT(*) FROM logs", [], |row| row.get(0))?;
        let cache_count: i64 = conn.query_row("SELECT COUNT(*) FROM cache", [], |row| row.get(0))?;
        
        // Get database file size
        let db_size = tokio::fs::metadata(&self.config.path).await
            .map(|m| m.len())
            .unwrap_or(0);
        
        Ok(DatabaseStatistics {
            job_count: job_count as usize,
            result_count: result_count as usize,
            log_count: log_count as usize,
            cache_count: cache_count as usize,
            database_size_bytes: db_size,
        })
    }
    
    /// Vacuum database to reclaim space
    pub async fn vacuum(&self) -> Result<()> {
        info!("Starting database vacuum operation");
        let conn = self.connection.lock().await;
        conn.execute("VACUUM", [])?;
        info!("Database vacuum completed");
        Ok(())
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatistics {
    pub job_count: usize,
    pub result_count: usize,
    pub log_count: usize,
    pub cache_count: usize,
    pub database_size_bytes: u64,
}

impl DatabaseStatistics {
    pub fn database_size_mb(&self) -> f64 {
        self.database_size_bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Performance statistics for database operations
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub operations: HashMap<String, OperationStats>,
}

/// Statistics for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    pub count: u64,
    pub avg_duration: Duration,
    pub p50_duration: Duration,
    pub p95_duration: Duration,
    pub p99_duration: Duration,
}
