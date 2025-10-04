use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;
use url::Url;

pub mod validator;
pub mod parser;
pub mod generator;

pub use validator::DSLValidator;

/// Scrape-Plan DSL structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapePlan {
    pub version: String,
    pub target: Target,
    pub rules: Rules,
    pub anti_blocking: AntiBlocking,
    pub output: Output,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Target configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub domain: String,
    pub start_urls: Vec<String>,
    pub url_patterns: Option<Vec<String>>,
    pub max_pages: Option<usize>,
}

/// Scraping rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rules {
    pub pagination: Option<Pagination>,
    pub item_selector: String,
    pub fields: Vec<Field>,
    pub filters: Option<Vec<Filter>>,
}

/// Pagination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub method: PaginationMethod,
    pub selector: Option<String>,
    pub max_pages: Option<usize>,
    pub wait_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaginationMethod {
    #[serde(rename = "link")]
    Link { next_selector: String },
    #[serde(rename = "button")]
    Button { button_selector: String },
    #[serde(rename = "scroll")]
    Scroll { scroll_pause_ms: u64 },
    #[serde(rename = "url_pattern")]
    UrlPattern { pattern: String, start: usize, end: usize },
}

/// Field extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub selector: String,
    pub selector_type: SelectorType,
    pub extraction: ExtractionMethod,
    pub required: bool,
    pub transform: Option<Vec<Transform>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectorType {
    #[serde(rename = "css")]
    CSS,
    #[serde(rename = "xpath")]
    XPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionMethod {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "html")]
    Html,
    #[serde(rename = "attr")]
    Attribute { name: String },
    #[serde(rename = "href")]
    Href,
    #[serde(rename = "src")]
    Src,
}

/// Data transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Transform {
    #[serde(rename = "trim")]
    Trim,
    #[serde(rename = "lowercase")]
    Lowercase,
    #[serde(rename = "uppercase")]
    Uppercase,
    #[serde(rename = "regex")]
    Regex { pattern: String, replacement: String },
    #[serde(rename = "parse_number")]
    ParseNumber,
    #[serde(rename = "parse_date")]
    ParseDate { format: Option<String> },
    #[serde(rename = "remove_html")]
    RemoveHtml,
    #[serde(rename = "extract_domain")]
    ExtractDomain,
}

/// Content filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub condition: FilterCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FilterCondition {
    #[serde(rename = "contains")]
    Contains { value: String },
    #[serde(rename = "not_contains")]
    NotContains { value: String },
    #[serde(rename = "equals")]
    Equals { value: String },
    #[serde(rename = "not_equals")]
    NotEquals { value: String },
    #[serde(rename = "regex")]
    Regex { pattern: String },
    #[serde(rename = "length_min")]
    LengthMin { min: usize },
    #[serde(rename = "length_max")]
    LengthMax { max: usize },
    #[serde(rename = "not_empty")]
    NotEmpty,
}

/// Anti-blocking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiBlocking {
    pub randomized_delays: DelayConfig,
    pub user_agent_rotation: bool,
    pub respect_robots_txt: bool,
    pub proxy: Option<ProxyConfig>,
    pub headers: Option<HashMap<String, String>>,
}

/// Delay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayConfig {
    pub min_ms: u64,
    pub max_ms: u64,
    pub distribution: DelayDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DelayDistribution {
    #[serde(rename = "uniform")]
    Uniform,
    #[serde(rename = "exponential")]
    Exponential,
    #[serde(rename = "normal")]
    Normal,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub proxies: Vec<String>,
    pub rotation: ProxyRotation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProxyRotation {
    #[serde(rename = "round_robin")]
    RoundRobin,
    #[serde(rename = "random")]
    Random,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub format: Vec<OutputFormat>,
    pub limit: Option<usize>,
    pub dedupe_keys: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    #[serde(rename = "csv")]
    CSV,
    #[serde(rename = "json")]
    JSON,
    #[serde(rename = "xlsx")]
    XLSX,
    #[serde(rename = "parquet")]
    Parquet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

impl Default for ScrapePlan {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            target: Target {
                domain: "example.com".to_string(),
                start_urls: vec!["https://example.com".to_string()],
                url_patterns: None,
                max_pages: Some(10),
            },
            rules: Rules {
                pagination: None,
                item_selector: "article".to_string(),
                fields: vec![
                    Field {
                        name: "title".to_string(),
                        selector: "h1, h2, .title".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![Transform::Trim]),
                    },
                    Field {
                        name: "url".to_string(),
                        selector: "a".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Href,
                        required: false,
                        transform: None,
                    },
                ],
                filters: None,
            },
            anti_blocking: AntiBlocking {
                randomized_delays: DelayConfig {
                    min_ms: 1000,
                    max_ms: 3000,
                    distribution: DelayDistribution::Uniform,
                },
                user_agent_rotation: true,
                respect_robots_txt: true,
                proxy: None,
                headers: None,
            },
            output: Output {
                format: vec![OutputFormat::CSV],
                limit: None,
                dedupe_keys: Some(vec!["url".to_string()]),
                sort_by: None,
                sort_order: None,
            },
            metadata: None,
        }
    }
}

impl ScrapePlan {
    /// Create a new scrape plan from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let plan: ScrapePlan = serde_yaml::from_str(yaml)?;
        Ok(plan)
    }
    
    /// Convert scrape plan to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
    
    /// Create a new scrape plan from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        let plan: ScrapePlan = serde_json::from_str(json)?;
        Ok(plan)
    }
    
    /// Convert scrape plan to JSON string
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    
    /// Validate the scrape plan
    pub fn validate(&self) -> Result<()> {
        let validator = DSLValidator::new();
        validator.validate(self)
    }
    
    /// Get all URLs that should be scraped
    pub fn get_all_urls(&self) -> Result<Vec<Url>> {
        let mut urls = Vec::new();
        
        // Add start URLs
        for url_str in &self.target.start_urls {
            let url = Url::parse(url_str)?;
            urls.push(url);
        }
        
        // Generate URLs from patterns if specified
        if let Some(patterns) = &self.target.url_patterns {
            for pattern in patterns {
                let generated_urls = self.generate_urls_from_pattern(pattern)?;
                urls.extend(generated_urls);
            }
        }
        
        Ok(urls)
    }
    
    /// Generate URLs from a pattern
    fn generate_urls_from_pattern(&self, pattern: &str) -> Result<Vec<Url>> {
        let mut urls = Vec::new();
        
        // Simple pattern matching for numbered pages
        if pattern.contains("{page}") {
            let max_pages = self.target.max_pages.unwrap_or(10);
            for page in 1..=max_pages {
                let url_str = pattern.replace("{page}", &page.to_string());
                if let Ok(url) = Url::parse(&url_str) {
                    urls.push(url);
                }
            }
        } else {
            // Direct URL
            if let Ok(url) = Url::parse(pattern) {
                urls.push(url);
            }
        }
        
        Ok(urls)
    }
    
    /// Get the domain from the target
    pub fn get_domain(&self) -> &str {
        &self.target.domain
    }
    
    /// Check if robots.txt should be respected
    pub fn should_respect_robots(&self) -> bool {
        self.anti_blocking.respect_robots_txt
    }
    
    /// Get delay configuration
    pub fn get_delay_config(&self) -> &DelayConfig {
        &self.anti_blocking.randomized_delays
    }
    
    /// Get required fields
    pub fn get_required_fields(&self) -> Vec<&Field> {
        self.rules.fields.iter().filter(|f| f.required).collect()
    }
    
    /// Get optional fields
    pub fn get_optional_fields(&self) -> Vec<&Field> {
        self.rules.fields.iter().filter(|f| !f.required).collect()
    }
    
    /// Check if field exists
    pub fn has_field(&self, name: &str) -> bool {
        self.rules.fields.iter().any(|f| f.name == name)
    }
    
    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.rules.fields.iter().find(|f| f.name == name)
    }
    
    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        if self.metadata.is_none() {
            self.metadata = Some(HashMap::new());
        }
        self.metadata.as_mut().unwrap().insert(key, value);
    }
    
    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.as_ref()?.get(key)
    }
}

/// DSL parsing errors
#[derive(Debug, thiserror::Error)]
pub enum DSLError {
    #[error("Invalid YAML format: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),
    
    #[error("Invalid JSON format: {0}")]
    InvalidJson(#[from] serde_json::Error),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid selector: {0}")]
    InvalidSelector(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// DSL examples for testing and documentation
pub struct DSLExamples;

impl DSLExamples {
    /// E-commerce product scraping example
    pub fn ecommerce_products() -> ScrapePlan {
        ScrapePlan {
            version: "1.0".to_string(),
            target: Target {
                domain: "shop.example.com".to_string(),
                start_urls: vec!["https://shop.example.com/products".to_string()],
                url_patterns: Some(vec!["https://shop.example.com/products?page={page}".to_string()]),
                max_pages: Some(50),
            },
            rules: Rules {
                pagination: Some(Pagination {
                    method: PaginationMethod::Link {
                        next_selector: "a.next-page".to_string(),
                    },
                    selector: None,
                    max_pages: Some(50),
                    wait_time_ms: Some(2000),
                }),
                item_selector: ".product-item".to_string(),
                fields: vec![
                    Field {
                        name: "title".to_string(),
                        selector: "h3.product-title".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![Transform::Trim]),
                    },
                    Field {
                        name: "price".to_string(),
                        selector: ".price".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![
                            Transform::Trim,
                            Transform::Regex {
                                pattern: r"[^\d.]".to_string(),
                                replacement: "".to_string(),
                            },
                            Transform::ParseNumber,
                        ]),
                    },
                    Field {
                        name: "image_url".to_string(),
                        selector: "img.product-image".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Src,
                        required: false,
                        transform: None,
                    },
                    Field {
                        name: "product_url".to_string(),
                        selector: "a.product-link".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Href,
                        required: true,
                        transform: None,
                    },
                ],
                filters: Some(vec![
                    Filter {
                        field: "price".to_string(),
                        condition: FilterCondition::NotEmpty,
                    },
                    Filter {
                        field: "title".to_string(),
                        condition: FilterCondition::LengthMin { min: 3 },
                    },
                ]),
            },
            anti_blocking: AntiBlocking {
                randomized_delays: DelayConfig {
                    min_ms: 1000,
                    max_ms: 3000,
                    distribution: DelayDistribution::Normal,
                },
                user_agent_rotation: true,
                respect_robots_txt: true,
                proxy: None,
                headers: Some({
                    let mut headers = HashMap::new();
                    headers.insert("Accept-Language".to_string(), "en-US,en;q=0.9".to_string());
                    headers
                }),
            },
            output: Output {
                format: vec![OutputFormat::CSV, OutputFormat::JSON],
                limit: Some(1000),
                dedupe_keys: Some(vec!["product_url".to_string()]),
                sort_by: Some("price".to_string()),
                sort_order: Some(SortOrder::Ascending),
            },
            metadata: Some({
                let mut metadata = HashMap::new();
                metadata.insert("description".to_string(), serde_json::Value::String("E-commerce product scraping".to_string()));
                metadata.insert("category".to_string(), serde_json::Value::String("products".to_string()));
                metadata
            }),
        }
    }
    
    /// News article scraping example
    pub fn news_articles() -> ScrapePlan {
        ScrapePlan {
            version: "1.0".to_string(),
            target: Target {
                domain: "news.example.com".to_string(),
                start_urls: vec!["https://news.example.com/latest".to_string()],
                url_patterns: None,
                max_pages: Some(20),
            },
            rules: Rules {
                pagination: Some(Pagination {
                    method: PaginationMethod::Button {
                        button_selector: "button.load-more".to_string(),
                    },
                    selector: None,
                    max_pages: Some(20),
                    wait_time_ms: Some(3000),
                }),
                item_selector: "article.news-item".to_string(),
                fields: vec![
                    Field {
                        name: "headline".to_string(),
                        selector: "h2.headline".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Text,
                        required: true,
                        transform: Some(vec![Transform::Trim]),
                    },
                    Field {
                        name: "summary".to_string(),
                        selector: ".summary".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![Transform::Trim, Transform::RemoveHtml]),
                    },
                    Field {
                        name: "author".to_string(),
                        selector: ".author".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Text,
                        required: false,
                        transform: Some(vec![Transform::Trim]),
                    },
                    Field {
                        name: "published_date".to_string(),
                        selector: "time".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Attribute { name: "datetime".to_string() },
                        required: false,
                        transform: Some(vec![Transform::ParseDate { format: None }]),
                    },
                    Field {
                        name: "article_url".to_string(),
                        selector: "a.read-more".to_string(),
                        selector_type: SelectorType::CSS,
                        extraction: ExtractionMethod::Href,
                        required: true,
                        transform: None,
                    },
                ],
                filters: Some(vec![
                    Filter {
                        field: "headline".to_string(),
                        condition: FilterCondition::NotEmpty,
                    },
                ]),
            },
            anti_blocking: AntiBlocking {
                randomized_delays: DelayConfig {
                    min_ms: 2000,
                    max_ms: 5000,
                    distribution: DelayDistribution::Exponential,
                },
                user_agent_rotation: true,
                respect_robots_txt: true,
                proxy: None,
                headers: None,
            },
            output: Output {
                format: vec![OutputFormat::JSON, OutputFormat::XLSX],
                limit: Some(500),
                dedupe_keys: Some(vec!["article_url".to_string()]),
                sort_by: Some("published_date".to_string()),
                sort_order: Some(SortOrder::Descending),
            },
            metadata: Some({
                let mut metadata = HashMap::new();
                metadata.insert("description".to_string(), serde_json::Value::String("News article scraping".to_string()));
                metadata.insert("category".to_string(), serde_json::Value::String("news".to_string()));
                metadata
            }),
        }
    }
}
