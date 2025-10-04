use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error, debug};

use crate::core::WinScrapeStudio;
use crate::dsl::ScrapePlan;
use crate::storage::JobStatus;

/// High-level orchestration logic for complex workflows
pub struct Orchestrator {
    app: Arc<WinScrapeStudio>,
}

impl Orchestrator {
    pub fn new(app: Arc<WinScrapeStudio>) -> Self {
        Self { app }
    }
    
    /// Execute complete workflow from natural language to results
    pub async fn execute_complete_workflow(
        &self,
        user_input: &str,
        auto_approve: bool,
    ) -> Result<WorkflowResult> {
        info!("Starting complete workflow for input: {}", user_input);
        
        let mut workflow = WorkflowExecution::new(user_input.to_string());
        
        // Stage 1: Natural Language Processing
        workflow.set_stage(WorkflowStage::NLProcessing);
        let dsl = match self.app.generate_dsl(user_input).await {
            Ok(dsl) => {
                workflow.add_log("DSL generated successfully".to_string());
                dsl
            }
            Err(e) => {
                workflow.add_error(format!("DSL generation failed: {}", e));
                return Ok(workflow.into_result());
            }
        };
        
        // Stage 2: Validation and Preview
        workflow.set_stage(WorkflowStage::Validation);
        let preview = match self.app.validate_and_preview(&dsl).await {
            Ok(preview) => {
                workflow.add_log(format!("Validation successful, {} preview rows", preview.len()));
                preview
            }
            Err(e) => {
                workflow.add_error(format!("Validation failed: {}", e));
                return Ok(workflow.into_result());
            }
        };
        
        // Stage 3: Approval Gate
        workflow.set_stage(WorkflowStage::Approval);
        if !auto_approve {
            workflow.add_log("Waiting for user approval".to_string());
            workflow.set_pending_approval(dsl.clone(), preview);
            return Ok(workflow.into_result());
        }
        
        // Stage 4: Execution
        workflow.set_stage(WorkflowStage::Execution);
        let job_id = match self.app.execute_scraping(&dsl).await {
            Ok(job_id) => {
                workflow.add_log(format!("Scraping job started: {}", job_id));
                job_id
            }
            Err(e) => {
                workflow.add_error(format!("Execution failed: {}", e));
                return Ok(workflow.into_result());
            }
        };
        
        // Stage 5: Monitoring
        workflow.set_stage(WorkflowStage::Monitoring);
        let final_status = self.monitor_job_completion(&job_id, &mut workflow).await?;
        
        // Stage 6: Completion
        workflow.set_stage(WorkflowStage::Completed);
        workflow.set_job_id(job_id);
        workflow.set_final_status(final_status);
        
        Ok(workflow.into_result())
    }
    
    /// Monitor job completion
    async fn monitor_job_completion(
        &self,
        job_id: &str,
        workflow: &mut WorkflowExecution,
    ) -> Result<JobStatus> {
        let mut attempts = 0;
        let max_attempts = 300; // 5 minutes with 1-second intervals
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            attempts += 1;
            
            let job = self.app.get_job(job_id).await?;
            
            match job.status {
                JobStatus::Completed => {
                    workflow.add_log("Job completed successfully".to_string());
                    return Ok(JobStatus::Completed);
                }
                JobStatus::Failed => {
                    workflow.add_error("Job failed during execution".to_string());
                    return Ok(JobStatus::Failed);
                }
                JobStatus::Cancelled => {
                    workflow.add_log("Job was cancelled".to_string());
                    return Ok(JobStatus::Cancelled);
                }
                JobStatus::Running | JobStatus::Queued => {
                    if attempts % 30 == 0 {
                        workflow.add_log(format!("Job still running... ({}s)", attempts));
                    }
                }
            }
            
            if attempts >= max_attempts {
                workflow.add_error("Job monitoring timeout".to_string());
                return Ok(JobStatus::Failed);
            }
        }
    }
    
    /// Execute batch workflow for multiple inputs
    pub async fn execute_batch_workflow(
        &self,
        inputs: Vec<String>,
        auto_approve: bool,
    ) -> Result<Vec<WorkflowResult>> {
        let input_count = inputs.len();
        info!("Starting batch workflow for {} inputs", input_count);
        
        let mut results = Vec::new();
        
        for (idx, input) in inputs.into_iter().enumerate() {
            info!("Processing batch item {}: {}", idx + 1, input);
            
            let result = self.execute_complete_workflow(&input, auto_approve).await?;
            results.push(result);
            
            // Add delay between batch items to avoid overwhelming the system
            if idx < input_count - 1 {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
        
        info!("Batch workflow completed");
        Ok(results)
    }
}

/// Workflow execution state tracker
pub struct WorkflowExecution {
    pub user_input: String,
    pub stage: WorkflowStage,
    pub logs: Vec<String>,
    pub errors: Vec<String>,
    pub job_id: Option<String>,
    pub final_status: Option<JobStatus>,
    pub pending_approval: Option<PendingApproval>,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum WorkflowStage {
    NLProcessing,
    Validation,
    Approval,
    Execution,
    Monitoring,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub dsl: ScrapePlan,
    pub preview: Vec<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub user_input: String,
    pub stage: WorkflowStage,
    pub logs: Vec<String>,
    pub errors: Vec<String>,
    pub job_id: Option<String>,
    pub final_status: Option<JobStatus>,
    pub pending_approval: Option<PendingApproval>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub success: bool,
}

impl WorkflowExecution {
    pub fn new(user_input: String) -> Self {
        Self {
            user_input,
            stage: WorkflowStage::NLProcessing,
            logs: Vec::new(),
            errors: Vec::new(),
            job_id: None,
            final_status: None,
            pending_approval: None,
            started_at: chrono::Utc::now(),
        }
    }
    
    pub fn set_stage(&mut self, stage: WorkflowStage) {
        self.stage = stage;
    }
    
    pub fn add_log(&mut self, message: String) {
        debug!("Workflow log: {}", message);
        self.logs.push(message);
    }
    
    pub fn add_error(&mut self, error: String) {
        error!("Workflow error: {}", error);
        self.errors.push(error);
        self.stage = WorkflowStage::Failed;
    }
    
    pub fn set_job_id(&mut self, job_id: String) {
        self.job_id = Some(job_id);
    }
    
    pub fn set_final_status(&mut self, status: JobStatus) {
        self.final_status = Some(status);
    }
    
    pub fn set_pending_approval(&mut self, dsl: ScrapePlan, preview: Vec<serde_json::Value>) {
        self.pending_approval = Some(PendingApproval { dsl, preview });
    }
    
    pub fn into_result(self) -> WorkflowResult {
        let success = self.errors.is_empty() && 
                     matches!(self.stage, WorkflowStage::Completed | WorkflowStage::Approval);
        
        WorkflowResult {
            user_input: self.user_input,
            stage: self.stage,
            logs: self.logs,
            errors: self.errors,
            job_id: self.job_id,
            final_status: self.final_status,
            pending_approval: self.pending_approval,
            started_at: self.started_at,
            completed_at: chrono::Utc::now(),
            success,
        }
    }
}

impl std::fmt::Display for WorkflowStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStage::NLProcessing => write!(f, "Natural Language Processing"),
            WorkflowStage::Validation => write!(f, "Validation & Preview"),
            WorkflowStage::Approval => write!(f, "Pending Approval"),
            WorkflowStage::Execution => write!(f, "Execution"),
            WorkflowStage::Monitoring => write!(f, "Monitoring"),
            WorkflowStage::Completed => write!(f, "Completed"),
            WorkflowStage::Failed => write!(f, "Failed"),
        }
    }
}
