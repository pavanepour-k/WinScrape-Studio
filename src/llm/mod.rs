use anyhow::Result;
use tracing::{info, warn, debug};

pub mod prompts;
pub mod processor;
#[cfg(feature = "local-llm")]
pub mod candle_backend;

use crate::config::LLMConfig;
use crate::dsl::ScrapePlan;

/// LLM processor for natural language to DSL conversion
pub struct LLMProcessor {
    config: LLMConfig,
    http_client: reqwest::Client,
}

/// Ollama's /api/generate request body (see
/// https://github.com/ollama/ollama/blob/main/docs/api.md).
#[derive(serde::Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    options: OllamaOptions,
}

#[derive(serde::Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: usize,
}

/// Ollama's /api/generate response body (with stream: false, this is a
/// single complete JSON object rather than a stream of chunks).
#[derive(serde::Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

impl LLMProcessor {
    /// Create new LLM processor
    pub async fn new(config: &LLMConfig) -> Result<Self> {
        info!("Initializing LLM processor");
        
        #[cfg(feature = "local-llm")]
        if config.enable_candle {
            info!(
                "Local GGUF inference via candle is enabled (model: {}). Tried before Ollama; \
                 falls back automatically if the model file is missing/unsupported.",
                config.model_path.display()
            );
        }
        #[cfg(not(feature = "local-llm"))]
        if config.enable_candle {
            warn!("llm.enable_candle is set but this build doesn't include the 'local-llm' feature - ignoring.");
        }
        
        if config.enable_ollama {
            info!(
                "Real LLM generation via Ollama is enabled (server: {}, model: {}). \
                 Falls back to rule-based generation if Ollama is unreachable.",
                config.ollama_url, config.ollama_model
            );
        } else {
            info!("Ollama integration disabled");
        }
        
        if !config.enable_candle && !config.enable_ollama {
            info!("No real LLM backend enabled - using rule-based DSL generation only");
        }
        
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.ollama_timeout_seconds))
            .build()?;
        
        info!("LLM processor initialized successfully");
        Ok(Self {
            config: config.clone(),
            http_client,
        })
    }
    
    /// Ask a local Ollama server to generate a DSL scraping plan for the
    /// given natural-language description. Returns `Ok(None)` (not an
    /// error) when Ollama integration is disabled, so callers can treat
    /// "disabled" and "unavailable" uniformly as "fall back to rules".
    async fn try_ollama_generate_dsl(&self, description: &str) -> Result<Option<ScrapePlan>> {
        if !self.config.enable_ollama {
            return Ok(None);
        }
        
        let prompt = prompts::build_dsl_generation_prompt(description);
        let url = format!("{}/api/generate", self.config.ollama_url.trim_end_matches('/'));
        
        debug!("Requesting DSL generation from Ollama at {}", url);
        
        let response = self.http_client
            .post(&url)
            .json(&OllamaGenerateRequest {
                model: &self.config.ollama_model,
                prompt: &prompt,
                stream: false,
                options: OllamaOptions {
                    temperature: self.config.temperature,
                    num_predict: self.config.max_tokens,
                },
            })
            .send()
            .await;
        
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                // Ollama not installed/running, or unreachable - this is
                // an expected, common case (most users won't have it
                // set up), so this is a debug/warn, not an error.
                warn!("Could not reach Ollama at {}: {}. Falling back to rule-based generation.", self.config.ollama_url, e);
                return Ok(None);
            }
        };
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                "Ollama returned an error (status {}): {}. Falling back to rule-based generation.",
                status, body
            );
            return Ok(None);
        }
        
        let body: OllamaGenerateResponse = match response.json().await {
            Ok(b) => b,
            Err(e) => {
                warn!("Could not parse Ollama's response: {}. Falling back to rule-based generation.", e);
                return Ok(None);
            }
        };
        
        let yaml = match self.extract_dsl_from_response(&body.response) {
            Ok(y) => y,
            Err(e) => {
                warn!("Could not extract a DSL plan from Ollama's response: {}. Falling back to rule-based generation.", e);
                return Ok(None);
            }
        };
        
        match crate::dsl::parser::DSLParser::parse_yaml(&yaml) {
            Ok(plan) => {
                info!("Ollama generated a usable DSL plan");
                Ok(Some(plan))
            }
            Err(e) => {
                warn!("Ollama's generated DSL didn't parse as a valid plan: {}. Falling back to rule-based generation.", e);
                Ok(None)
            }
        }
    }
    
    /// Attempt DSL generation using an in-process candle GGUF model
    /// (only compiled in with `--features local-llm`). Returns `Ok(None)`
    /// (not an error) when disabled or unavailable, same convention as
    /// `try_ollama_generate_dsl`, so callers can fall through uniformly.
    #[cfg(feature = "local-llm")]
    async fn try_candle_generate_dsl(&self, description: &str) -> Result<Option<ScrapePlan>> {
        if !self.config.enable_candle {
            return Ok(None);
        }
        
        let prompt = prompts::build_dsl_generation_prompt(description);
        let config = self.config.clone();
        
        // candle's inference loop is synchronous and CPU-bound (no .await
        // points inside it), so it has to run on a blocking thread rather
        // than the async runtime, same as the xlsx/parquet exporters.
        let generation = tokio::task::spawn_blocking(move || {
            candle_backend::generate(&config, &prompt)
        })
        .await;
        
        let text = match generation {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                warn!("Local GGUF generation failed: {}. Falling back.", e);
                return Ok(None);
            }
            Err(e) => {
                warn!("Local GGUF generation task panicked: {}. Falling back.", e);
                return Ok(None);
            }
        };
        
        let yaml = match self.extract_dsl_from_response(&text) {
            Ok(y) => y,
            Err(e) => {
                warn!("Could not extract a DSL plan from the local model's output: {}. Falling back.", e);
                return Ok(None);
            }
        };
        
        match crate::dsl::parser::DSLParser::parse_yaml(&yaml) {
            Ok(plan) => {
                info!("Local GGUF model generated a usable DSL plan");
                Ok(Some(plan))
            }
            Err(e) => {
                warn!("Local model's generated DSL didn't parse as a valid plan: {}. Falling back.", e);
                Ok(None)
            }
        }
    }
    
    /// No-op stand-in when built without the local-llm feature, so
    /// generate_dsl's call site doesn't need its own cfg-gating.
    #[cfg(not(feature = "local-llm"))]
    async fn try_candle_generate_dsl(&self, _description: &str) -> Result<Option<ScrapePlan>> {
        Ok(None)
    }
    
    /// Generate DSL from natural language description
    pub async fn generate_dsl(&self, description: &str) -> Result<ScrapePlan> {
        info!("Generating DSL from description: {}", description);
        
        // Try local, in-process GGUF inference first (free, no network
        // call at all, but requires the user to have built with
        // --features local-llm and pointed llm.model_path at a
        // compatible GGUF file - see candle_backend.rs). Then a local
        // Ollama server. Either produces None (not an error) when
        // unavailable/disabled/failed, so we fall through to the
        // rule-based generator - the safety net that's always available.
        match self.try_candle_generate_dsl(description).await {
            Ok(Some(mut plan)) => {
                plan.add_metadata("generated_by".to_string(), serde_json::Value::String("candle".to_string()));
                plan.add_metadata("user_prompt".to_string(), serde_json::Value::String(description.to_string()));
                info!("DSL generated by local GGUF model with {} fields", plan.rules.fields.len());
                return Ok(plan);
            }
            Ok(None) => {
                debug!("Local GGUF inference unavailable or declined to produce a usable plan; trying Ollama");
            }
            Err(e) => {
                warn!("Unexpected error during local GGUF inference: {}. Trying Ollama.", e);
            }
        }
        
        match self.try_ollama_generate_dsl(description).await {
            Ok(Some(mut plan)) => {
                plan.add_metadata("generated_by".to_string(), serde_json::Value::String("ollama".to_string()));
                plan.add_metadata("user_prompt".to_string(), serde_json::Value::String(description.to_string()));
                info!("DSL generated by Ollama with {} fields", plan.rules.fields.len());
                return Ok(plan);
            }
            Ok(None) => {
                debug!("Ollama unavailable or declined to produce a usable plan; using rule-based generation");
            }
            Err(e) => {
                warn!("Unexpected error calling Ollama: {}. Using rule-based generation.", e);
            }
        }
        
        // Enhanced rule-based approach with better pattern matching
        let analysis = processor::LLMProcessor::extract_intent(description);
        
        let mut plan = ScrapePlan::default();
        
        // Set domain if found
        if let Some(domain) = analysis.domain {
            plan.target.domain = domain.clone();
            plan.target.start_urls = vec![format!("https://{}", domain)];
        }
        
        // Enhanced field generation based on scraping type
        plan.rules.fields.clear();
        match analysis.scraping_type {
            processor::ScrapingType::Ecommerce => {
                plan.rules.fields.extend(vec![
                    crate::dsl::Field {
                        name: "title".to_string(),
                        selector: "h1, .product-title, .title, [data-testid='product-title']".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![crate::dsl::Transform::Trim]),
                    },
                    crate::dsl::Field {
                        name: "price".to_string(),
                        selector: ".price, .cost, [data-testid='price'], .amount".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![crate::dsl::Transform::ParseNumber]),
                    },
                    crate::dsl::Field {
                        name: "description".to_string(),
                        selector: ".description, .product-description, .summary".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![crate::dsl::Transform::RemoveHtml]),
                    },
                    crate::dsl::Field {
                        name: "image".to_string(),
                        selector: "img.product-image, .product-img img, [data-testid='product-image']".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Src,
                        required: false,
                        transform: None,
                    },
                ]);
            },
            processor::ScrapingType::News => {
                plan.rules.fields.extend(vec![
                    crate::dsl::Field {
                        name: "headline".to_string(),
                        selector: "h1, .headline, .title, .article-title".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![crate::dsl::Transform::Trim]),
                    },
                    crate::dsl::Field {
                        name: "content".to_string(),
                        selector: ".article-content, .story-body, .content, .article-text".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![crate::dsl::Transform::RemoveHtml]),
                    },
                    crate::dsl::Field {
                        name: "author".to_string(),
                        selector: ".author, .byline, .writer, [rel='author']".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![crate::dsl::Transform::Trim]),
                    },
                    crate::dsl::Field {
                        name: "date".to_string(),
                        selector: ".date, .published, .timestamp, time".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![crate::dsl::Transform::ParseDate { format: None }]),
                    },
                ]);
            },
            processor::ScrapingType::Directory => {
                plan.rules.fields.extend(vec![
                    crate::dsl::Field {
                        name: "name".to_string(),
                        selector: ".name, .title, h3, .listing-name".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![crate::dsl::Transform::Trim]),
                    },
                    crate::dsl::Field {
                        name: "description".to_string(),
                        selector: ".description, .summary, .details".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![crate::dsl::Transform::RemoveHtml]),
                    },
                    crate::dsl::Field {
                        name: "contact".to_string(),
                        selector: ".contact, .phone, .email, .address".to_string(),
                        selector_type: crate::dsl::SelectorType::CSS,
                        extraction: crate::dsl::ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![crate::dsl::Transform::Trim]),
                    },
                ]);
            },
            _ => {
                // Generic scraping - use extracted fields or common patterns
                if !analysis.fields.is_empty() {
                    for field_name in analysis.fields {
                        plan.rules.fields.push(crate::dsl::Field {
                            name: field_name.clone(),
                            selector: format!(".{}, #{}, [class*='{}'], [id*='{}']", 
                                field_name, field_name, field_name, field_name),
                            selector_type: crate::dsl::SelectorType::CSS,
                            extraction: crate::dsl::ExtractionMethod::Text,
                            required: false,
                            transform: Some(vec![crate::dsl::Transform::Trim]),
                        });
                    }
                } else {
                    // Default generic fields
                    plan.rules.fields.extend(vec![
                        crate::dsl::Field {
                            name: "title".to_string(),
                            selector: "h1, h2, .title, .heading".to_string(),
                            selector_type: crate::dsl::SelectorType::CSS,
                            extraction: crate::dsl::ExtractionMethod::Text,
                            required: false,
                            transform: Some(vec![crate::dsl::Transform::Trim]),
                        },
                        crate::dsl::Field {
                            name: "content".to_string(),
                            selector: ".content, .text, .body, p".to_string(),
                            selector_type: crate::dsl::SelectorType::CSS,
                            extraction: crate::dsl::ExtractionMethod::Text,
                            required: false,
                            transform: Some(vec![crate::dsl::Transform::RemoveHtml]),
                        },
                    ]);
                }
            }
        }
        
        // Set item selector based on scraping type
        match analysis.scraping_type {
            processor::ScrapingType::Ecommerce => {
                plan.rules.item_selector = ".product, .item, .listing, [data-testid='product']".to_string();
            },
            processor::ScrapingType::News => {
                plan.rules.item_selector = ".article, .story, .post, .news-item".to_string();
            },
            processor::ScrapingType::Directory => {
                plan.rules.item_selector = ".listing, .entry, .item, .result".to_string();
            },
            _ => {
                plan.rules.item_selector = ".item, .entry, .result, .listing".to_string();
            }
        }
        
        // Add user prompt as metadata
        plan.add_metadata("user_prompt".to_string(), serde_json::Value::String(description.to_string()));
        plan.add_metadata("generated_by".to_string(), serde_json::Value::String("rule_based".to_string()));
        plan.add_metadata("scraping_type".to_string(), serde_json::Value::String(format!("{:?}", analysis.scraping_type)));
        plan.add_metadata("confidence".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(analysis.confidence as f64).unwrap()));
        
        info!("DSL generated successfully with {} fields", plan.rules.fields.len());
        Ok(plan)
    }
    
    /// Extract DSL YAML from LLM response (simplified)
    fn extract_dsl_from_response(&self, response: &str) -> Result<String> {
        // Look for YAML code blocks
        if let Some(start) = response.find("```yaml") {
            if let Some(end) = response[start + 7..].find("```") {
                let yaml_content = &response[start + 7..start + 7 + end];
                return Ok(yaml_content.trim().to_string());
            }
        }
        
        // Look for generic code blocks
        if let Some(start) = response.find("```") {
            if let Some(end) = response[start + 3..].find("```") {
                let yaml_content = &response[start + 3..start + 3 + end];
                // Try to parse as YAML to validate
                if serde_yaml::from_str::<serde_yaml::Value>(yaml_content.trim()).is_ok() {
                    return Ok(yaml_content.trim().to_string());
                }
            }
        }
        
        // If no code blocks found, try to find YAML-like content
        let lines: Vec<&str> = response.lines().collect();
        let mut yaml_lines = Vec::new();
        let mut in_yaml = false;
        
        for line in lines {
            let trimmed = line.trim();
            
            // Start of YAML (version line or target line)
            if trimmed.starts_with("version:") || trimmed.starts_with("target:") {
                in_yaml = true;
                yaml_lines.push(line);
            } else if in_yaml {
                // Continue collecting YAML lines
                if trimmed.is_empty() || trimmed.starts_with(' ') || trimmed.starts_with('-') || trimmed.contains(':') {
                    yaml_lines.push(line);
                } else {
                    // End of YAML-like content
                    break;
                }
            }
        }
        
        if !yaml_lines.is_empty() {
            let yaml_content = yaml_lines.join("\n");
            // Validate YAML
            if serde_yaml::from_str::<serde_yaml::Value>(&yaml_content).is_ok() {
                return Ok(yaml_content);
            }
        }
        
        // Fallback: return a default DSL with user input as metadata
        warn!("Could not extract valid DSL from LLM response, using fallback");
        self.create_fallback_dsl(response)
    }
    
    /// Create fallback DSL when parsing fails
    fn create_fallback_dsl(&self, original_response: &str) -> Result<String> {
        let mut plan = crate::dsl::ScrapePlan::default();
        
        // Try to extract domain from response
        if let Some(domain) = self.extract_domain_from_text(original_response) {
            plan.target.domain = domain.clone();
            plan.target.start_urls = vec![format!("https://{}", domain)];
        }
        
        // Add original response as metadata
        plan.add_metadata("llm_response".to_string(), serde_json::Value::String(original_response.to_string()));
        plan.add_metadata("fallback".to_string(), serde_json::Value::Bool(true));
        
        Ok(plan.to_yaml()?)
    }
    
    /// Extract domain from text using simple heuristics
    fn extract_domain_from_text(&self, text: &str) -> Option<String> {
        // Look for URLs
        let url_regex = regex::Regex::new(r"https?://([a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,})").ok()?;
        if let Some(captures) = url_regex.captures(text) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
        
        // Look for domain-like patterns
        let domain_regex = regex::Regex::new(r"\b([a-zA-Z0-9\-]+\.[a-zA-Z]{2,})\b").ok()?;
        if let Some(captures) = domain_regex.captures(text) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }
        
        None
    }
    
    /// Validate generated DSL
    pub async fn validate_generated_dsl(&self, dsl: &ScrapePlan) -> Result<f32> {
        // Simple validation scoring
        let mut score = 0.0;
        let mut max_score = 0.0;
        
        // Check if domain is valid
        max_score += 1.0;
        if !dsl.target.domain.is_empty() && dsl.target.domain.contains('.') {
            score += 1.0;
        }
        
        // Check if start URLs are valid
        max_score += 1.0;
        if !dsl.target.start_urls.is_empty() {
            let valid_urls = dsl.target.start_urls.iter()
                .filter(|url| url::Url::parse(url).is_ok())
                .count();
            score += valid_urls as f32 / dsl.target.start_urls.len() as f32;
        }
        
        // Check if fields are defined
        max_score += 1.0;
        if !dsl.rules.fields.is_empty() {
            score += 1.0;
        }
        
        // Check if selectors look valid
        max_score += 1.0;
        let valid_selectors = dsl.rules.fields.iter()
            .filter(|field| !field.selector.is_empty() && field.selector.len() > 1)
            .count();
        if !dsl.rules.fields.is_empty() {
            score += valid_selectors as f32 / dsl.rules.fields.len() as f32;
        }
        
        // Return confidence score (0.0 to 1.0)
        if max_score > 0.0 {
            Ok(score / max_score)
        } else {
            Ok(0.0)
        }
    }
    
    /// Get model information
    pub fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            model_path: self.config.model_path.clone(),
            context_size: self.config.context_size,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            threads: self.config.threads,
        }
    }
}

/// Model information structure
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub model_path: std::path::PathBuf,
    pub context_size: usize,
    pub temperature: f32,
    pub max_tokens: usize,
    pub threads: usize,
}

/// LLM processing errors
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Model loading failed: {0}")]
    ModelLoadError(String),
    
    #[error("Context creation failed: {0}")]
    ContextError(String),
    
    #[error("Text generation failed: {0}")]
    GenerationError(String),
    
    #[error("DSL parsing failed: {0}")]
    DSLParsingError(String),
    
    #[error("Invalid model configuration: {0}")]
    ConfigError(String),
}
