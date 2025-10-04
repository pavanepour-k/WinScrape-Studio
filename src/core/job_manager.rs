use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use chrono::{DateTime, Utc};

use crate::storage::{StorageManager, JobStatus, JobResult};
use crate::dsl::ScrapePlan;

/// Manages job execution and lifecycle
pub struct JobManager {
    storage: Arc<StorageManager>,
    active_jobs: HashMap<String, JobHandle>,
    job_queue: Vec<QueuedJob>,
    max_concurrent_jobs: usize,
}

/// Handle for an active job
pub struct JobHandle {
    pub job_id: String,
    pub status: JobStatus,
    pub started_at: DateTime<Utc>,
    pub cancel_tx: mpsc::Sender<()>,
}

/// Queued job waiting for execution
pub struct QueuedJob {
    pub job_id: String,
    pub dsl: ScrapePlan,
    pub priority: JobPriority,
    pub queued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl JobManager {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        Self {
            storage,
            active_jobs: HashMap::new(),
            job_queue: Vec::new(),
            max_concurrent_jobs: 3, // Configurable limit
        }
    }
    
    /// Execute a scraping job
    pub async fn execute_job(&mut self, job_id: &str, dsl: ScrapePlan) -> Result<()> {
        info!("Executing job: {}", job_id);
        
        // Check if we can start immediately or need to queue
        if self.active_jobs.len() >= self.max_concurrent_jobs {
            self.queue_job(job_id, dsl, JobPriority::Normal).await?;
            return Ok(());
        }
        
        self.start_job(job_id, dsl).await
    }
    
    /// Queue a job for later execution
    async fn queue_job(&mut self, job_id: &str, dsl: ScrapePlan, priority: JobPriority) -> Result<()> {
        info!("Queueing job: {} with priority: {:?}", job_id, priority);
        
        let queued_job = QueuedJob {
            job_id: job_id.to_string(),
            dsl,
            priority,
            queued_at: Utc::now(),
        };
        
        // Insert in priority order
        let insert_pos = self.job_queue
            .binary_search_by(|job| job.priority.cmp(&queued_job.priority).reverse())
            .unwrap_or_else(|pos| pos);
        
        self.job_queue.insert(insert_pos, queued_job);
        
        // Update job status in storage
        self.storage.update_job_status(job_id, JobStatus::Queued).await?;
        
        Ok(())
    }
    
    /// Start executing a job immediately
    async fn start_job(&mut self, job_id: &str, dsl: ScrapePlan) -> Result<()> {
        info!("Starting job execution: {}", job_id);
        
        // Create cancellation channel
        let (cancel_tx, cancel_rx) = mpsc::channel(1);
        
        // Create job handle
        let handle = JobHandle {
            job_id: job_id.to_string(),
            status: JobStatus::Running,
            started_at: Utc::now(),
            cancel_tx,
        };
        
        self.active_jobs.insert(job_id.to_string(), handle);
        
        // Update status in storage
        self.storage.update_job_status(job_id, JobStatus::Running).await?;
        
        // Spawn job execution task
        let job_id_clone = job_id.to_string();
        let storage_clone = self.storage.clone();
        
        // Execute job in a separate task
        // Note: We need to handle the fact that scraper crate types are not Send.
        // We'll execute the job synchronously in this context instead of spawning.
        let result = execute_scraping_job(
            &job_id_clone,
            dsl,
            storage_clone.clone(),
            cancel_rx,
        ).await;
        
        match result {
            Ok(_) => {
                info!("Job {} completed successfully", job_id_clone);
                if let Err(e) = storage_clone.update_job_status(&job_id_clone, JobStatus::Completed).await {
                    error!("Failed to update job status: {}", e);
                }
            }
            Err(e) => {
                error!("Job {} failed: {}", job_id_clone, e);
                if let Err(e) = storage_clone.update_job_status(&job_id_clone, JobStatus::Failed).await {
                    error!("Failed to update job status: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Cancel a running job
    pub async fn cancel_job(&mut self, job_id: &str) -> Result<()> {
        info!("Cancelling job: {}", job_id);
        
        if let Some(handle) = self.active_jobs.get(job_id) {
            // Send cancellation signal
            if let Err(e) = handle.cancel_tx.send(()).await {
                warn!("Failed to send cancellation signal for job {}: {}", job_id, e);
            }
            
            // Update status
            self.storage.update_job_status(job_id, JobStatus::Cancelled).await?;
            
            // Remove from active jobs
            self.active_jobs.remove(job_id);
            
            // Try to start next queued job
            self.try_start_next_job().await?;
        } else {
            // Check if it's in the queue
            if let Some(pos) = self.job_queue.iter().position(|job| job.job_id == job_id) {
                self.job_queue.remove(pos);
                self.storage.update_job_status(job_id, JobStatus::Cancelled).await?;
            } else {
                warn!("Job {} not found in active jobs or queue", job_id);
            }
        }
        
        Ok(())
    }
    
    /// Try to start the next queued job if there's capacity
    async fn try_start_next_job(&mut self) -> Result<()> {
        if self.active_jobs.len() < self.max_concurrent_jobs && !self.job_queue.is_empty() {
            let next_job = self.job_queue.remove(0);
            self.start_job(&next_job.job_id, next_job.dsl).await?;
        }
        Ok(())
    }
    
    /// Get status of all jobs
    pub fn get_job_statuses(&self) -> HashMap<String, JobStatus> {
        let mut statuses = HashMap::new();
        
        // Active jobs
        for (job_id, handle) in &self.active_jobs {
            statuses.insert(job_id.clone(), handle.status.clone());
        }
        
        // Queued jobs
        for queued_job in &self.job_queue {
            statuses.insert(queued_job.job_id.clone(), JobStatus::Queued);
        }
        
        statuses
    }
    
    /// Clean up completed jobs
    pub async fn cleanup_completed_jobs(&mut self) -> Result<()> {
        let completed_jobs: Vec<String> = self.active_jobs
            .iter()
            .filter(|(_, handle)| handle.status == JobStatus::Completed || handle.status == JobStatus::Failed)
            .map(|(job_id, _)| job_id.clone())
            .collect();
        
        for job_id in completed_jobs {
            self.active_jobs.remove(&job_id);
            self.try_start_next_job().await?;
        }
        
        Ok(())
    }
}

/// Execute the actual scraping job
async fn execute_scraping_job(
    job_id: &str,
    dsl: ScrapePlan,
    storage: Arc<StorageManager>,
    mut cancel_rx: mpsc::Receiver<()>,
) -> Result<()> {
    info!("Executing scraping for job: {}", job_id);
    
    // Initialize scraping engine (this would normally be passed in)
    let scraping_config = crate::config::ScrapingConfig::default();
    let scraper = crate::scraper::ScrapingEngine::new(&scraping_config).await?;
    
    // Execute scraping with cancellation support
    let scraping_future = scraper.execute_scraping(&dsl);
    let cancellation_future = cancel_rx.recv();
    
    tokio::select! {
        result = scraping_future => {
            match result {
                Ok(results) => {
                    info!("Scraping completed for job: {}, {} results", job_id, results.len());
                    
                    // Store results
                    for (idx, result) in results.into_iter().enumerate() {
                        let job_result = JobResult {
                            job_id: job_id.to_string(),
                            row_idx: idx as i32,
                            data_json: serde_json::to_string(&result)?,
                            url: result.get("_source_url")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            fetched_at: Utc::now(),
                            hash: calculate_result_hash(&result),
                        };
                        
                        storage.store_job_result(&job_result).await?;
                    }
                    
                    info!("Results stored for job: {}", job_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Scraping failed for job {}: {}", job_id, e);
                    Err(e)
                }
            }
        }
        _ = cancellation_future => {
            warn!("Job {} was cancelled", job_id);
            Err(anyhow::anyhow!("Job was cancelled"))
        }
    }
}

/// Calculate hash for deduplication
fn calculate_result_hash(result: &serde_json::Value) -> String {
    use sha2::{Sha256, Digest};
    
    let serialized = serde_json::to_string(result).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    format!("{:x}", hasher.finalize())
}
