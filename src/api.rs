use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::core::WinScrapeStudio;
use crate::dsl::ScrapePlan;
use crate::export::ExportFormat;

/// API request for DSL generation
#[derive(Debug, Deserialize)]
pub struct GenerateDSLRequest {
    pub description: String,
}

/// API response for DSL generation
#[derive(Debug, Serialize)]
pub struct GenerateDSLResponse {
    pub dsl: ScrapePlan,
    pub success: bool,
    pub message: String,
}

/// API request for scraping execution
#[derive(Debug, Deserialize)]
pub struct ExecuteScrapingRequest {
    pub dsl: ScrapePlan,
}

/// API response for scraping execution
#[derive(Debug, Serialize)]
pub struct ExecuteScrapingResponse {
    pub job_id: String,
    pub success: bool,
    pub message: String,
}

/// API request for job export
#[derive(Debug, Deserialize)]
pub struct ExportJobRequest {
    pub job_id: String,
    pub format: String,
}

/// Configure API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/generate-dsl", web::post().to(generate_dsl))
            .route("/execute-scraping", web::post().to(execute_scraping))
            .route("/jobs", web::get().to(list_jobs))
            .route("/jobs/{job_id}", web::get().to(get_job))
            .route("/jobs/{job_id}/export", web::post().to(export_job))
            .route("/health", web::get().to(health_check))
    );
}

/// Generate DSL from natural language description
async fn generate_dsl(
    app: web::Data<Arc<WinScrapeStudio>>,
    req: web::Json<GenerateDSLRequest>,
) -> ActixResult<HttpResponse> {
    info!("API: Generating DSL for description: {}", req.description);
    
    match app.generate_dsl(&req.description).await {
        Ok(dsl) => {
            let response = GenerateDSLResponse {
                dsl,
                success: true,
                message: "DSL generated successfully".to_string(),
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("API: Failed to generate DSL: {}", e);
            let response = GenerateDSLResponse {
                dsl: ScrapePlan::default(),
                success: false,
                message: format!("Failed to generate DSL: {}", e),
            };
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/// Execute scraping job
async fn execute_scraping(
    app: web::Data<Arc<WinScrapeStudio>>,
    req: web::Json<ExecuteScrapingRequest>,
) -> ActixResult<HttpResponse> {
    info!("API: Executing scraping job");
    
    match app.execute_scraping(&req.dsl).await {
        Ok(job_id) => {
            let response = ExecuteScrapingResponse {
                job_id,
                success: true,
                message: "Scraping job started successfully".to_string(),
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("API: Failed to execute scraping: {}", e);
            let response = ExecuteScrapingResponse {
                job_id: String::new(),
                success: false,
                message: format!("Failed to execute scraping: {}", e),
            };
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/// List recent jobs
async fn list_jobs(
    app: web::Data<Arc<WinScrapeStudio>>,
) -> ActixResult<HttpResponse> {
    info!("API: Listing jobs");
    
    match app.list_jobs(50).await {
        Ok(jobs) => Ok(HttpResponse::Ok().json(jobs)),
        Err(e) => {
            error!("API: Failed to list jobs: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to list jobs: {}", e)
            })))
        }
    }
}

/// Get job details
async fn get_job(
    app: web::Data<Arc<WinScrapeStudio>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();
    info!("API: Getting job details for: {}", job_id);
    
    match app.get_job(&job_id).await {
        Ok(job) => Ok(HttpResponse::Ok().json(job)),
        Err(e) => {
            error!("API: Failed to get job {}: {}", job_id, e);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "message": format!("Job not found: {}", e)
            })))
        }
    }
}

/// Export job results
async fn export_job(
    app: web::Data<Arc<WinScrapeStudio>>,
    path: web::Path<String>,
    req: web::Json<ExportJobRequest>,
) -> ActixResult<HttpResponse> {
    let job_id = path.into_inner();
    info!("API: Exporting job {} in format {}", job_id, req.format);
    
    let format = match req.format.as_str() {
        "csv" => ExportFormat::Csv,
        "json" => ExportFormat::Json,
        "xlsx" => ExportFormat::Xlsx,
        "parquet" => ExportFormat::Parquet,
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "message": "Unsupported export format"
            })));
        }
    };
    
    let output_path = format!("exports/job_{}_{}.{}", job_id, 
        chrono::Utc::now().format("%Y%m%d_%H%M%S"), 
        req.format);
    
    match app.export_job(&job_id, &output_path, format).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Export completed successfully",
            "file_path": output_path
        }))),
        Err(e) => {
            error!("API: Failed to export job {}: {}", job_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to export job: {}", e)
            })))
        }
    }
}

/// Health check endpoint
async fn health_check() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
