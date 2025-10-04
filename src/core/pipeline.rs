use anyhow::Result;
use tracing::{info, warn, debug};
use serde::{Serialize, Deserialize};

/// Pipeline for processing scraping data through multiple stages
pub struct ScrapingPipeline {
    stages: Vec<Box<dyn PipelineStage>>,
    config: PipelineConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub batch_size: usize,
    pub max_concurrent_stages: usize,
    pub enable_deduplication: bool,
    pub enable_validation: bool,
    pub enable_transformation: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_concurrent_stages: 3,
            enable_deduplication: true,
            enable_validation: true,
            enable_transformation: true,
        }
    }
}

/// Trait for pipeline stages
#[async_trait::async_trait]
pub trait PipelineStage: Send + Sync {
    async fn process(&self, data: PipelineData) -> Result<PipelineData>;
    fn name(&self) -> &str;
    fn is_enabled(&self, config: &PipelineConfig) -> bool;
}

/// Data flowing through the pipeline
#[derive(Debug, Clone)]
pub struct PipelineData {
    pub items: Vec<serde_json::Value>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub stage_info: Vec<StageInfo>,
}

#[derive(Debug, Clone)]
pub struct StageInfo {
    pub stage_name: String,
    pub processed_at: chrono::DateTime<chrono::Utc>,
    pub items_in: usize,
    pub items_out: usize,
    pub processing_time_ms: u64,
}

impl ScrapingPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        let mut pipeline = Self {
            stages: Vec::new(),
            config,
        };
        
        // Add default stages
        pipeline.add_stage(Box::new(ValidationStage::new()));
        pipeline.add_stage(Box::new(DeduplicationStage::new()));
        pipeline.add_stage(Box::new(TransformationStage::new()));
        pipeline.add_stage(Box::new(NormalizationStage::new()));
        
        pipeline
    }
    
    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }
    
    /// Process data through all pipeline stages
    pub async fn process(&self, mut data: PipelineData) -> Result<PipelineData> {
        info!("Starting pipeline processing with {} items", data.items.len());
        
        for stage in &self.stages {
            if !stage.is_enabled(&self.config) {
                debug!("Skipping disabled stage: {}", stage.name());
                continue;
            }
            
            let start_time = std::time::Instant::now();
            let items_in = data.items.len();
            
            debug!("Processing stage: {} with {} items", stage.name(), items_in);
            
            data = stage.process(data).await?;
            
            let processing_time = start_time.elapsed();
            let items_out = data.items.len();
            
            data.stage_info.push(StageInfo {
                stage_name: stage.name().to_string(),
                processed_at: chrono::Utc::now(),
                items_in,
                items_out,
                processing_time_ms: processing_time.as_millis() as u64,
            });
            
            info!(
                "Stage {} completed: {} -> {} items in {}ms",
                stage.name(),
                items_in,
                items_out,
                processing_time.as_millis()
            );
        }
        
        info!("Pipeline processing completed with {} items", data.items.len());
        Ok(data)
    }
    
    /// Process data in batches for memory efficiency
    pub async fn process_batched(&self, items: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>> {
        let mut all_results = Vec::new();
        
        for chunk in items.chunks(self.config.batch_size) {
            let data = PipelineData {
                items: chunk.to_vec(),
                metadata: std::collections::HashMap::new(),
                stage_info: Vec::new(),
            };
            
            let processed = self.process(data).await?;
            all_results.extend(processed.items);
        }
        
        Ok(all_results)
    }
}

/// Validation stage - ensures data quality
pub struct ValidationStage {
    rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: ValidationRuleType,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    NotEmpty,
    Url,
    Email,
    Number,
    Date,
    Length { min: usize, max: usize },
    Regex(String),
}

impl ValidationStage {
    pub fn new() -> Self {
        Self {
            rules: vec![
                ValidationRule {
                    field: "url".to_string(),
                    rule_type: ValidationRuleType::Url,
                    required: false,
                },
                ValidationRule {
                    field: "title".to_string(),
                    rule_type: ValidationRuleType::NotEmpty,
                    required: false,
                },
            ],
        }
    }
    
    fn validate_item(&self, item: &serde_json::Value) -> bool {
        for rule in &self.rules {
            if let Some(field_value) = item.get(&rule.field) {
                if !self.validate_field(field_value, &rule.rule_type) {
                    return false;
                }
            } else if rule.required {
                return false;
            }
        }
        true
    }
    
    fn validate_field(&self, value: &serde_json::Value, rule_type: &ValidationRuleType) -> bool {
        match rule_type {
            ValidationRuleType::NotEmpty => {
                !value.as_str().unwrap_or("").trim().is_empty()
            }
            ValidationRuleType::Url => {
                if let Some(url_str) = value.as_str() {
                    url::Url::parse(url_str).is_ok()
                } else {
                    false
                }
            }
            ValidationRuleType::Email => {
                if let Some(email_str) = value.as_str() {
                    email_str.contains('@') && email_str.contains('.')
                } else {
                    false
                }
            }
            ValidationRuleType::Number => {
                value.is_number()
            }
            ValidationRuleType::Length { min, max } => {
                if let Some(str_val) = value.as_str() {
                    str_val.len() >= *min && str_val.len() <= *max
                } else {
                    false
                }
            }
            ValidationRuleType::Regex(pattern) => {
                if let Some(str_val) = value.as_str() {
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(str_val))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            _ => true,
        }
    }
}

#[async_trait::async_trait]
impl PipelineStage for ValidationStage {
    async fn process(&self, mut data: PipelineData) -> Result<PipelineData> {
        let original_count = data.items.len();
        
        data.items.retain(|item| self.validate_item(item));
        
        let filtered_count = original_count - data.items.len();
        if filtered_count > 0 {
            warn!("Validation stage filtered out {} invalid items", filtered_count);
        }
        
        Ok(data)
    }
    
    fn name(&self) -> &str {
        "validation"
    }
    
    fn is_enabled(&self, config: &PipelineConfig) -> bool {
        config.enable_validation
    }
}

/// Deduplication stage - removes duplicate entries
pub struct DeduplicationStage {
    hash_fields: Vec<String>,
}

impl DeduplicationStage {
    pub fn new() -> Self {
        Self {
            hash_fields: vec!["url".to_string(), "title".to_string()],
        }
    }
    
    fn calculate_hash(&self, item: &serde_json::Value) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        for field in &self.hash_fields {
            if let Some(value) = item.get(field) {
                hasher.update(value.to_string().as_bytes());
            }
        }
        
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait::async_trait]
impl PipelineStage for DeduplicationStage {
    async fn process(&self, mut data: PipelineData) -> Result<PipelineData> {
        let original_count = data.items.len();
        let mut seen_hashes = std::collections::HashSet::new();
        
        data.items.retain(|item| {
            let hash = self.calculate_hash(item);
            seen_hashes.insert(hash)
        });
        
        let duplicate_count = original_count - data.items.len();
        if duplicate_count > 0 {
            info!("Deduplication stage removed {} duplicate items", duplicate_count);
        }
        
        Ok(data)
    }
    
    fn name(&self) -> &str {
        "deduplication"
    }
    
    fn is_enabled(&self, config: &PipelineConfig) -> bool {
        config.enable_deduplication
    }
}

/// Transformation stage - applies data transformations
pub struct TransformationStage {
    transformations: Vec<DataTransformation>,
}

#[derive(Debug, Clone)]
pub enum DataTransformation {
    Trim(String),
    Lowercase(String),
    Uppercase(String),
    ParseNumber(String),
    ParseDate(String),
    ExtractDomain(String),
    RemoveHtml(String),
}

impl TransformationStage {
    pub fn new() -> Self {
        Self {
            transformations: vec![
                DataTransformation::Trim("title".to_string()),
                DataTransformation::Trim("description".to_string()),
                DataTransformation::RemoveHtml("description".to_string()),
            ],
        }
    }
    
    fn apply_transformation(&self, item: &mut serde_json::Value, transformation: &DataTransformation) {
        match transformation {
            DataTransformation::Trim(field) => {
                if let Some(value) = item.get_mut(field) {
                    if let Some(str_val) = value.as_str() {
                        *value = serde_json::Value::String(str_val.trim().to_string());
                    }
                }
            }
            DataTransformation::Lowercase(field) => {
                if let Some(value) = item.get_mut(field) {
                    if let Some(str_val) = value.as_str() {
                        *value = serde_json::Value::String(str_val.to_lowercase());
                    }
                }
            }
            DataTransformation::RemoveHtml(field) => {
                if let Some(value) = item.get_mut(field) {
                    if let Some(str_val) = value.as_str() {
                        // Simple HTML tag removal (in production, use a proper HTML parser)
                        let cleaned = regex::Regex::new(r"<[^>]*>")
                            .unwrap()
                            .replace_all(str_val, "")
                            .to_string();
                        *value = serde_json::Value::String(cleaned);
                    }
                }
            }
            // Add more transformations as needed
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl PipelineStage for TransformationStage {
    async fn process(&self, mut data: PipelineData) -> Result<PipelineData> {
        for item in &mut data.items {
            for transformation in &self.transformations {
                self.apply_transformation(item, transformation);
            }
        }
        
        Ok(data)
    }
    
    fn name(&self) -> &str {
        "transformation"
    }
    
    fn is_enabled(&self, config: &PipelineConfig) -> bool {
        config.enable_transformation
    }
}

/// Normalization stage - standardizes data formats
pub struct NormalizationStage;

impl NormalizationStage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl PipelineStage for NormalizationStage {
    async fn process(&self, mut data: PipelineData) -> Result<PipelineData> {
        for item in &mut data.items {
            // Add metadata
            if !item.as_object().unwrap().contains_key("_processed_at") {
                item.as_object_mut().unwrap().insert(
                    "_processed_at".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
            }
            
            // Ensure consistent field types
            self.normalize_item(item);
        }
        
        Ok(data)
    }
    
    fn name(&self) -> &str {
        "normalization"
    }
    
    fn is_enabled(&self, _config: &PipelineConfig) -> bool {
        true // Always enabled
    }
}

impl NormalizationStage {
    fn normalize_item(&self, item: &mut serde_json::Value) {
        if let Some(obj) = item.as_object_mut() {
            // Normalize URLs
            if let Some(url_val) = obj.get_mut("url") {
                if let Some(url_str) = url_val.as_str() {
                    if let Ok(parsed_url) = url::Url::parse(url_str) {
                        *url_val = serde_json::Value::String(parsed_url.to_string());
                    }
                }
            }
            
            // Normalize prices (remove currency symbols, convert to numbers)
            if let Some(price_val) = obj.get_mut("price") {
                if let Some(price_str) = price_val.as_str() {
                    let cleaned_price = price_str
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '.')
                        .collect::<String>();
                    
                    if let Ok(price_num) = cleaned_price.parse::<f64>() {
                        *price_val = serde_json::json!(price_num);
                    }
                }
            }
        }
    }
}
