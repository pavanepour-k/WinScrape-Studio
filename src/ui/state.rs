#[cfg(feature = "ui")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "ui")]
use crate::core::orchestrator::{WorkflowResult, PendingApproval};

/// UI application state
#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIState {
    pub current_view: View,
    pub jobs: Vec<JobInfo>,
    #[serde(skip)]
    pub current_workflow: Option<WorkflowState>,
    #[serde(skip)]
    pub pending_approval: Option<PendingApproval>,
    #[serde(skip)]
    pub status_message: Option<String>,
    #[serde(skip, default = "std::time::Instant::now")]
    pub last_job_refresh: std::time::Instant,
}

#[cfg(feature = "ui")]
impl UIState {
    pub fn new() -> Self {
        Self {
            current_view: View::Chat,
            jobs: Vec::new(),
            current_workflow: None,
            pending_approval: None,
            status_message: None,
            last_job_refresh: std::time::Instant::now(),
        }
    }
}

/// UI views
#[cfg(feature = "ui")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    Chat,
    Jobs,
    Settings,
    Help,
}

/// Job information for UI display
#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result_count: Option<usize>,
}

/// Job status for UI
#[cfg(feature = "ui")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow execution state
#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub enum WorkflowState {
    Processing,
    Completed(WorkflowResult),
    Failed(String),
}

// Stub implementations when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct UIState;

#[cfg(not(feature = "ui"))]
impl UIState {
    pub fn new() -> Self {
        Self
    }
}
