use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, error};

mod core;
mod config;
mod storage;
mod scraper;
mod llm;
mod dsl;
mod export;
mod security;
mod utils;

use crate::core::WinScrapeStudio;
use crate::config::AppConfig;

#[derive(Parser)]
#[command(name = "wss-cli")]
#[command(about = "WinScrape Studio Command Line Interface")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
    
    #[arg(short, long, help = "Configuration file path")]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a scraping job from natural language description
    Scrape {
        #[arg(help = "Natural language description of what to scrape")]
        description: String,
        
        #[arg(short, long, help = "Output file path")]
        output: Option<String>,
        
        #[arg(short, long, help = "Output format", value_enum)]
        format: Option<OutputFormat>,
        
        #[arg(long, help = "Skip preview and approval (dangerous)")]
        auto_approve: bool,
    },
    
    /// List previous scraping jobs
    List {
        #[arg(short, long, help = "Number of jobs to show")]
        limit: Option<usize>,
    },
    
    /// Show details of a specific job
    Show {
        #[arg(help = "Job ID")]
        job_id: String,
    },
    
    /// Re-run a previous job
    Rerun {
        #[arg(help = "Job ID to re-run")]
        job_id: String,
        
        #[arg(short, long, help = "Output file path")]
        output: Option<String>,
    },
    
    /// Validate a DSL file
    Validate {
        #[arg(help = "Path to DSL file")]
        dsl_file: String,
    },
    
    /// Export job results
    Export {
        #[arg(help = "Job ID")]
        job_id: String,
        
        #[arg(short, long, help = "Output file path")]
        output: String,
        
        #[arg(short, long, help = "Output format", value_enum)]
        format: OutputFormat,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Csv,
    Json,
    Xlsx,
    Parquet,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging based on verbosity
    let log_level = if cli.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", format!("winscrape_studio={}", log_level));
    
    tracing_subscriber::fmt::init();
    
    info!("WinScrape Studio CLI v{}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config = if let Some(config_path) = cli.config {
        AppConfig::load_from_file(&config_path).await?
    } else {
        AppConfig::load().await?
    };
    
    // Initialize core application
    let app = WinScrapeStudio::new(config).await?;
    
    // Execute command
    match cli.command {
        Commands::Scrape { description, output, format, auto_approve } => {
            execute_scrape(&app, description, output, format, auto_approve).await?;
        }
        Commands::List { limit } => {
            list_jobs(&app, limit).await?;
        }
        Commands::Show { job_id } => {
            show_job(&app, job_id).await?;
        }
        Commands::Rerun { job_id, output } => {
            rerun_job(&app, job_id, output).await?;
        }
        Commands::Validate { dsl_file } => {
            validate_dsl(&app, dsl_file).await?;
        }
        Commands::Export { job_id, output, format } => {
            export_job(&app, job_id, output, format).await?;
        }
    }
    
    Ok(())
}

async fn execute_scrape(
    app: &WinScrapeStudio,
    description: String,
    output: Option<String>,
    format: Option<OutputFormat>,
    auto_approve: bool,
) -> Result<()> {
    info!("Processing scraping request: {}", description);
    
    // Generate DSL from natural language
    let dsl = app.generate_dsl(&description).await?;
    println!("Generated DSL:");
    println!("{}", serde_yaml::to_string(&dsl)?);
    
    // Validate and preview
    let preview = app.validate_and_preview(&dsl).await?;
    println!("\nPreview (first 10 rows):");
    for (i, row) in preview.iter().enumerate().take(10) {
        println!("{}: {:?}", i + 1, row);
    }
    
    if !auto_approve {
        println!("\nProceed with full scraping? (y/N): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().to_lowercase().starts_with('y') {
            println!("Scraping cancelled.");
            return Ok(());
        }
    }
    
    // Execute full scraping
    let job_id = app.execute_scraping(&dsl).await?;
    println!("Scraping completed. Job ID: {}", job_id);
    
    // Export if requested
    if let Some(output_path) = output {
        let export_format = format.unwrap_or(OutputFormat::Csv);
        app.export_job(&job_id, &output_path, convert_format(export_format)).await?;
        println!("Results exported to: {}", output_path);
    }
    
    Ok(())
}

async fn list_jobs(app: &WinScrapeStudio, limit: Option<usize>) -> Result<()> {
    let jobs = app.list_jobs(limit.unwrap_or(20)).await?;
    
    println!("Recent scraping jobs:");
    println!("{:<36} {:<20} {:<15} {:<20}", "Job ID", "Title", "Status", "Created");
    println!("{}", "-".repeat(91));
    
    for job in jobs {
        println!(
            "{:<36} {:<20} {:<15} {:<20}",
            job.id,
            job.title.chars().take(20).collect::<String>(),
            job.status,
            job.created_at.format("%Y-%m-%d %H:%M:%S")
        );
    }
    
    Ok(())
}

async fn show_job(app: &WinScrapeStudio, job_id: String) -> Result<()> {
    let job = app.get_job(&job_id).await?;
    
    println!("Job Details:");
    println!("ID: {}", job.id);
    println!("Title: {}", job.title);
    println!("Status: {}", job.status);
    println!("Created: {}", job.created_at);
    println!("User Prompt: {}", job.user_prompt);
    println!("\nDSL Plan:");
    println!("{}", job.plan_yaml);
    
    if let Some(settings) = job.settings_json {
        println!("\nSettings:");
        println!("{}", settings);
    }
    
    Ok(())
}

async fn rerun_job(app: &WinScrapeStudio, job_id: String, output: Option<String>) -> Result<()> {
    info!("Re-running job: {}", job_id);
    
    let new_job_id = app.rerun_job(&job_id).await?;
    println!("Job re-run completed. New Job ID: {}", new_job_id);
    
    if let Some(output_path) = output {
        app.export_job(&new_job_id, &output_path, crate::export::ExportFormat::Csv).await?;
        println!("Results exported to: {}", output_path);
    }
    
    Ok(())
}

async fn validate_dsl(app: &WinScrapeStudio, dsl_file: String) -> Result<()> {
    let dsl_content = std::fs::read_to_string(&dsl_file)?;
    let dsl: crate::dsl::ScrapePlan = serde_yaml::from_str(&dsl_content)?;
    
    match app.validate_dsl(&dsl).await {
        Ok(_) => println!("DSL file is valid."),
        Err(e) => {
            error!("DSL validation failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

async fn export_job(
    app: &WinScrapeStudio,
    job_id: String,
    output: String,
    format: OutputFormat,
) -> Result<()> {
    app.export_job(&job_id, &output, convert_format(format)).await?;
    println!("Job {} exported to: {}", job_id, output);
    Ok(())
}

fn convert_format(format: OutputFormat) -> crate::export::ExportFormat {
    match format {
        OutputFormat::Csv => crate::export::ExportFormat::Csv,
        OutputFormat::Json => crate::export::ExportFormat::Json,
        OutputFormat::Xlsx => crate::export::ExportFormat::Xlsx,
        OutputFormat::Parquet => crate::export::ExportFormat::Parquet,
    }
}
