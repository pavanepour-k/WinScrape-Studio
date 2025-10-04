use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

pub mod orchestrator;
pub mod job_manager;
pub mod pipeline;

use crate::config::AppConfig;
use crate::storage::{StorageManager, Job, JobStatus};
use crate::scraper::ScrapingEngine;
use crate::llm::LLMProcessor;
use crate::dsl::{ScrapePlan, DSLValidator};
use crate::export::{ExportManager, ExportFormat};
use crate::security::SecurityManager;

/// Core application state and orchestrator
pub struct WinScrapeStudio {
    config: AppConfig,
    storage: Arc<StorageManager>,
    scraper: Arc<ScrapingEngine>,
    llm: Arc<LLMProcessor>,
    dsl_validator: Arc<DSLValidator>,
    export_manager: Arc<ExportManager>,
    security_manager: Arc<SecurityManager>,
    job_manager: Arc<RwLock<job_manager::JobManager>>,
}

impl WinScrapeStudio {
    /// Initialize the core application with all subsystems
    pub async fn new(config: AppConfig) -> Result<Self> {
        info!("Initializing WinScrape Studio core");
        
        // Initialize storage layer
        let storage = Arc::new(StorageManager::new(&config.database).await?);
        info!("Storage manager initialized");
        
        // Initialize LLM processor
        let llm = Arc::new(LLMProcessor::new(&config.llm).await?);
        info!("LLM processor initialized");
        
        // Initialize scraping engine
        let scraper = Arc::new(ScrapingEngine::new(&config.scraping).await?);
        info!("Scraping engine initialized");
        
        // Initialize DSL validator
        let dsl_validator = Arc::new(DSLValidator::new());
        info!("DSL validator initialized");
        
        // Initialize export manager
        let export_manager = Arc::new(ExportManager::new(&config.export)?);
        info!("Export manager initialized");
        
        // Initialize security manager
        let security_manager = Arc::new(SecurityManager::new(&config.security)?);
        info!("Security manager initialized");
        
        // Initialize job manager
        let job_manager = Arc::new(RwLock::new(
            job_manager::JobManager::new(storage.clone())
        ));
        info!("Job manager initialized");
        
        Ok(Self {
            config,
            storage,
            scraper,
            llm,
            dsl_validator,
            export_manager,
            security_manager,
            job_manager,
        })
    }
    
    /// Generate DSL from natural language description
    pub async fn generate_dsl(&self, description: &str) -> Result<ScrapePlan> {
        info!("Generating DSL from description: {}", description);
        
        // Security check on input
        self.security_manager.validate_input(description)?;
        
        // Generate DSL using LLM
        let dsl = self.llm.generate_dsl(description).await?;
        
        // Validate generated DSL
        self.dsl_validator.validate(&dsl)?;
        
        info!("DSL generated and validated successfully");
        Ok(dsl)
    }
    
    /// Validate DSL and generate preview
    pub async fn validate_and_preview(&self, dsl: &ScrapePlan) -> Result<Vec<serde_json::Value>> {
        info!("Validating DSL and generating preview");
        
        // Validate DSL structure
        self.dsl_validator.validate(dsl)?;
        
        // Security validation
        self.security_manager.validate_dsl(dsl)?;
        
        // Generate preview (limited to 10 rows)
        let preview = self.scraper.generate_preview(dsl, 10).await?;
        
        info!("Preview generated with {} rows", preview.len());
        Ok(preview)
    }
    
    /// Validate DSL without preview
    pub async fn validate_dsl(&self, dsl: &ScrapePlan) -> Result<()> {
        self.dsl_validator.validate(dsl)?;
        self.security_manager.validate_dsl(dsl)?;
        Ok(())
    }
    
    /// Execute full scraping job
    pub async fn execute_scraping(&self, dsl: &ScrapePlan) -> Result<String> {
        let job_id = Uuid::new_v4().to_string();
        info!("Starting scraping job: {}", job_id);
        
        // Create job record
        let job = Job {
            id: job_id.clone(),
            title: dsl.target.domain.clone(),
            status: JobStatus::Running,
            created_at: chrono::Utc::now(),
            plan_yaml: serde_yaml::to_string(dsl)?,
            user_prompt: dsl.metadata.as_ref()
                .and_then(|m| m.get("user_prompt"))
                .and_then(|v| v.as_str())
                .unwrap_or("Direct DSL execution")
                .to_string(),
            settings_json: Some(serde_json::to_string(&self.config)?),
        };
        
        self.storage.create_job(&job).await?;
        
        // Execute scraping
        let mut job_manager = self.job_manager.write().await;
        job_manager.execute_job(&job_id, dsl.clone()).await?;
        
        info!("Scraping job {} completed", job_id);
        Ok(job_id)
    }
    
    /// List recent jobs
    pub async fn list_jobs(&self, limit: usize) -> Result<Vec<Job>> {
        self.storage.list_jobs(limit).await
    }
    
    /// Get job details
    pub async fn get_job(&self, job_id: &str) -> Result<Job> {
        self.storage.get_job(job_id).await
    }
    
    /// Re-run existing job
    pub async fn rerun_job(&self, job_id: &str) -> Result<String> {
        let original_job = self.storage.get_job(job_id).await?;
        let dsl: ScrapePlan = serde_yaml::from_str(&original_job.plan_yaml)?;
        
        self.execute_scraping(&dsl).await
    }
    
    /// Export job results
    pub async fn export_job(&self, job_id: &str, output_path: &str, format: ExportFormat) -> Result<()> {
        info!("Exporting job {} to {}", job_id, output_path);
        
        let results = self.storage.get_job_results(job_id).await?;
        self.export_manager.export(&results, output_path, format).await?;
        
        info!("Export completed");
        Ok(())
    }
    
    /// Run GUI interface
    #[cfg(feature = "ui")]
    pub async fn run_gui(&mut self) -> Result<()> {
        // GUI implementation should be handled in main.rs
        // This is just a placeholder to maintain the API
        info!("GUI interface requested - implementation should be in main.rs");
        Ok(())
    }
    
    /// Run in headless mode
    pub async fn run_headless(&self) -> Result<()> {
        info!("Running in headless mode - use CLI interface");
        
        // Keep the application running for potential API access
        #[cfg(feature = "api")]
        {
            self.start_api_server().await?;
        }
        
        #[cfg(not(feature = "api"))]
        {
            warn!("No API feature enabled - application will exit");
        }
        
        Ok(())
    }
    
    /// Start optional API server
    #[cfg(feature = "api")]
    async fn start_api_server(&self) -> Result<()> {
        // API server implementation should be handled in main.rs
        // This is just a placeholder to maintain the API
        info!("API server requested - implementation should be in main.rs");
        Ok(())
    }
    
    /// Clone for UI usage (simplified interface)
    #[cfg(feature = "ui")]
    async fn clone_for_ui(&self) -> Result<Arc<Self>> {
        // For UI, we need a shared reference
        // This is a simplified approach - in production, consider using channels
        Ok(Arc::new(Self {
            config: self.config.clone(),
            storage: self.storage.clone(),
            scraper: self.scraper.clone(),
            llm: self.llm.clone(),
            dsl_validator: self.dsl_validator.clone(),
            export_manager: self.export_manager.clone(),
            security_manager: self.security_manager.clone(),
            job_manager: self.job_manager.clone(),
        }))
    }
    
    /// Clone for API usage
    #[cfg(feature = "api")]
    async fn clone_for_api(&self) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            config: self.config.clone(),
            storage: self.storage.clone(),
            scraper: self.scraper.clone(),
            llm: self.llm.clone(),
            dsl_validator: self.dsl_validator.clone(),
            export_manager: self.export_manager.clone(),
            security_manager: self.security_manager.clone(),
            job_manager: self.job_manager.clone(),
        }))
    }

    /// Get the export manager for testing purposes
    pub fn get_export_manager(&self) -> &ExportManager {
        &self.export_manager
    }

    /// Get the security manager for testing purposes
    pub fn get_security_manager(&self) -> &SecurityManager {
        &self.security_manager
    }
}
