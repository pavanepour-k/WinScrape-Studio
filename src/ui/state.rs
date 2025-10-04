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
    pub settings: UISettings,
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
            settings: UISettings::default(),
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

/// UI settings
#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    pub max_concurrent_requests: usize,
    pub request_timeout: u64,
    pub respect_robots_txt: bool,
    pub enable_browser_fallback: bool,
    pub default_export_format: String,
    pub enable_input_validation: bool,
    pub enable_output_filtering: bool,
}

#[cfg(feature = "ui")]
impl Default for UISettings {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 5,
            request_timeout: 30,
            respect_robots_txt: true,
            enable_browser_fallback: false,
            default_export_format: "csv".to_string(),
            enable_input_validation: true,
            enable_output_filtering: true,
        }
    }
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
