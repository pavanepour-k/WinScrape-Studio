use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub llm: LLMConfig,
    pub scraping: ScrapingConfig,
    pub export: ExportConfig,
    pub security: SecurityConfig,
    #[cfg(feature = "api")]
    pub api: ApiConfig,
    pub ui: UIConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub max_connections: u32,
    pub enable_wal: bool,
    pub cache_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub model_path: PathBuf,
    pub context_size: usize,
    pub temperature: f32,
    pub max_tokens: usize,
    pub threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingConfig {
    pub max_concurrent_requests: usize,
    pub request_timeout_seconds: u64,
    pub max_retries: usize,
    pub retry_delay_seconds: u64,
    pub respect_robots_txt: bool,
    pub default_delay_ms: u64,
    pub user_agents: Vec<String>,
    pub enable_browser_fallback: bool,
    pub browser_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub default_format: String,
    pub max_file_size_mb: usize,
    pub compression_enabled: bool,
    pub output_directory: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_input_validation: bool,
    pub enable_output_filtering: bool,
    pub max_input_length: usize,
    pub blocked_domains: Vec<String>,
    pub allowed_schemes: Vec<String>,
    pub enable_rate_limiting: bool,
    pub rate_limit_requests_per_minute: usize,
}

#[cfg(feature = "api")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub max_request_size_mb: usize,
    pub enable_auth: bool,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: String,
    pub font_size: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub enable_dark_mode: bool,
    pub chat_history_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_enabled: bool,
    pub console_enabled: bool,
    pub max_file_size_mb: usize,
    pub max_files: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = get_data_directory();
        
        Self {
            database: DatabaseConfig {
                path: data_dir.join("winscrape.db"),
                max_connections: 10,
                enable_wal: true,
                cache_size_mb: 64,
            },
            llm: LLMConfig {
                model_path: data_dir.join("models").join("llama-2-7b-chat.q4_0.gguf"),
                context_size: 2048,
                temperature: 0.1,
                max_tokens: 512,
                threads: 4,
            },
            scraping: ScrapingConfig {
                max_concurrent_requests: 5,
                request_timeout_seconds: 30,
                max_retries: 3,
                retry_delay_seconds: 2,
                respect_robots_txt: true,
                default_delay_ms: 1000,
                user_agents: vec![
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/120.0.0.0 Safari/537.36".to_string(),
                ],
                enable_browser_fallback: false,
                browser_timeout_seconds: 60,
            },
            export: ExportConfig {
                default_format: "csv".to_string(),
                max_file_size_mb: 100,
                compression_enabled: true,
                output_directory: data_dir.join("exports"),
            },
            security: SecurityConfig {
                enable_input_validation: true,
                enable_output_filtering: true,
                max_input_length: 10000,
                blocked_domains: vec![
                    "localhost".to_string(),
                    "127.0.0.1".to_string(),
                    "0.0.0.0".to_string(),
                ],
                allowed_schemes: vec![
                    "http".to_string(),
                    "https".to_string(),
                ],
                enable_rate_limiting: true,
                rate_limit_requests_per_minute: 60,
            },
            #[cfg(feature = "api")]
            api: ApiConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                enable_cors: true,
                max_request_size_mb: 10,
                enable_auth: false,
                auth_token: None,
            },
            ui: UIConfig {
                theme: "dark".to_string(),
                font_size: 14.0,
                window_width: 1200.0,
                window_height: 800.0,
                enable_dark_mode: true,
                chat_history_limit: 100,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_enabled: true,
                console_enabled: true,
                max_file_size_mb: 10,
                max_files: 5,
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from default locations
    pub async fn load() -> Result<Self> {
        let config_path = get_config_path();
        
        if config_path.exists() {
            Self::load_from_file(&config_path).await
        } else {
            info!("No configuration file found, using defaults");
            let config = Self::default();
            config.save().await?;
            Ok(config)
        }
    }
    
    /// Load configuration from specific file
    pub async fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: AppConfig = toml::from_str(&content)?;
        
        // Validate configuration
        config.validate()?;
        
        info!("Configuration loaded successfully");
        Ok(config)
    }
    
    /// Save configuration to default location
    pub async fn save(&self) -> Result<()> {
        let config_path = get_config_path();
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(&config_path, content).await?;
        
        info!("Configuration saved to: {}", config_path.display());
        Ok(())
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate database configuration
        if self.database.max_connections == 0 {
            return Err(anyhow::anyhow!("Database max_connections must be > 0"));
        }
        
        // Validate LLM configuration
        if self.llm.context_size == 0 {
            return Err(anyhow::anyhow!("LLM context_size must be > 0"));
        }
        
        if !(0.0..=2.0).contains(&self.llm.temperature) {
            return Err(anyhow::anyhow!("LLM temperature must be between 0.0 and 2.0"));
        }
        
        // Validate scraping configuration
        if self.scraping.max_concurrent_requests == 0 {
            return Err(anyhow::anyhow!("Scraping max_concurrent_requests must be > 0"));
        }
        
        if self.scraping.user_agents.is_empty() {
            return Err(anyhow::anyhow!("At least one user agent must be configured"));
        }
        
        // Validate security configuration
        if self.security.max_input_length == 0 {
            return Err(anyhow::anyhow!("Security max_input_length must be > 0"));
        }
        
        // Validate export configuration
        if self.export.max_file_size_mb == 0 {
            return Err(anyhow::anyhow!("Export max_file_size_mb must be > 0"));
        }
        
        #[cfg(feature = "api")]
        {
            if self.api.port == 0 {
                return Err(anyhow::anyhow!("API port must be > 0"));
            }
        }
        
        info!("Configuration validation passed");
        Ok(())
    }
    
    /// Get data directory path
    pub fn get_data_dir(&self) -> PathBuf {
        self.database.path.parent()
            .unwrap_or(&std::path::Path::new("."))
            .to_path_buf()
    }
    
    /// Ensure all required directories exist
    pub async fn ensure_directories(&self) -> Result<()> {
        let dirs_to_create = vec![
            self.get_data_dir(),
            self.export.output_directory.clone(),
            self.llm.model_path.parent().unwrap_or(&std::path::Path::new(".")).to_path_buf(),
        ];
        
        for dir in dirs_to_create {
            if !dir.exists() {
                tokio::fs::create_dir_all(&dir).await?;
                info!("Created directory: {}", dir.display());
            }
        }
        
        Ok(())
    }
}

/// Get the default data directory
fn get_data_directory() -> PathBuf {
    directories::ProjectDirs::from("com", "winscrape", "studio")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("data"))
}

/// Get the configuration file path
fn get_config_path() -> PathBuf {
    directories::ProjectDirs::from("com", "winscrape", "studio")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("config.toml"))
}

/// Environment-based configuration overrides
pub struct ConfigOverrides;

impl ConfigOverrides {
    /// Apply environment variable overrides to configuration
    pub fn apply(config: &mut AppConfig) {
        // Database overrides
        if let Ok(db_path) = std::env::var("WSS_DB_PATH") {
            config.database.path = PathBuf::from(db_path);
        }
        
        // LLM overrides
        if let Ok(model_path) = std::env::var("WSS_LLM_MODEL_PATH") {
            config.llm.model_path = PathBuf::from(model_path);
        }
        
        if let Ok(temp_str) = std::env::var("WSS_LLM_TEMPERATURE") {
            if let Ok(temp) = temp_str.parse::<f32>() {
                config.llm.temperature = temp;
            }
        }
        
        // Scraping overrides
        if let Ok(concurrent_str) = std::env::var("WSS_SCRAPING_CONCURRENT") {
            if let Ok(concurrent) = concurrent_str.parse::<usize>() {
                config.scraping.max_concurrent_requests = concurrent;
            }
        }
        
        if let Ok(robots_str) = std::env::var("WSS_RESPECT_ROBOTS") {
            config.scraping.respect_robots_txt = robots_str.to_lowercase() == "true";
        }
        
        // Security overrides
        if let Ok(rate_limit_str) = std::env::var("WSS_RATE_LIMIT") {
            if let Ok(rate_limit) = rate_limit_str.parse::<usize>() {
                config.security.rate_limit_requests_per_minute = rate_limit;
            }
        }
        
        // API overrides
        #[cfg(feature = "api")]
        {
            if let Ok(api_host) = std::env::var("WSS_API_HOST") {
                config.api.host = api_host;
            }
            
            if let Ok(api_port_str) = std::env::var("WSS_API_PORT") {
                if let Ok(api_port) = api_port_str.parse::<u16>() {
                    config.api.port = api_port;
                }
            }
        }
        
        // Logging overrides
        if let Ok(log_level) = std::env::var("WSS_LOG_LEVEL") {
            config.logging.level = log_level;
        }
        
        info!("Applied environment variable overrides");
    }
}
