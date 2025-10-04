/// Prompt templates for LLM interactions
use crate::dsl::{ScrapePlan, DSLExamples};

/// Build prompt for DSL generation from natural language
pub fn build_dsl_generation_prompt(user_description: &str) -> String {
    let system_prompt = get_system_prompt();
    let examples = get_dsl_examples();
    let user_prompt = format_user_request(user_description);
    
    format!(
        "{}\n\n{}\n\n{}\n\nPlease generate a scraping plan in YAML format:",
        system_prompt,
        examples,
        user_prompt
    )
}

/// System prompt defining the LLM's role and capabilities
fn get_system_prompt() -> &'static str {
    r#"You are an expert web scraping assistant. Your task is to convert natural language descriptions into structured scraping plans using a specific DSL (Domain Specific Language) format.

Key principles:
1. Always respect robots.txt by default
2. Use reasonable delays to avoid overwhelming servers
3. Extract meaningful field names and selectors
4. Provide fallback selectors when possible
5. Include proper data transformations
6. Generate valid YAML format

The scraping plan should include:
- Target domain and URLs
- CSS selectors for items and fields
- Data extraction methods
- Anti-blocking measures
- Output configuration

Always generate complete, valid YAML that follows the schema exactly."#
}

/// Get example DSL configurations for few-shot learning
fn get_dsl_examples() -> String {
    let ecommerce_example = DSLExamples::ecommerce_products();
    let news_example = DSLExamples::news_articles();
    
    format!(
        r#"Here are examples of valid scraping plans:

Example 1 - E-commerce Product Scraping:
```yaml
{}
```

Example 2 - News Article Scraping:
```yaml
{}
```"#,
        ecommerce_example.to_yaml().unwrap_or_default(),
        news_example.to_yaml().unwrap_or_default()
    )
}

/// Format user request with context
fn format_user_request(description: &str) -> String {
    format!(
        "User Request: \"{}\"\n\nBased on this description, create a scraping plan that:",
        description.trim()
    )
}

/// Build prompt for DSL validation and improvement
pub fn build_dsl_validation_prompt(dsl: &ScrapePlan, issues: &[String]) -> String {
    format!(
        r#"Please review and improve this scraping plan:

```yaml
{}
```

Issues found:
{}

Please provide an improved version that addresses these issues while maintaining the same scraping goals."#,
        dsl.to_yaml().unwrap_or_default(),
        issues.iter()
            .enumerate()
            .map(|(i, issue)| format!("{}. {}", i + 1, issue))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Build prompt for selector suggestion
pub fn build_selector_suggestion_prompt(html_snippet: &str, field_description: &str) -> String {
    format!(
        r#"Given this HTML snippet, suggest the best CSS selector for extracting "{}":

```html
{}
```

Provide:
1. Primary CSS selector
2. Fallback selector (if applicable)
3. Extraction method (text, attribute, etc.)
4. Any necessary transformations

Format your response as a JSON object with these fields."#,
        field_description,
        html_snippet
    )
}

/// Build prompt for domain analysis
pub fn build_domain_analysis_prompt(domain: &str, sample_urls: &[String]) -> String {
    format!(
        r#"Analyze the website structure for domain: {}

Sample URLs:
{}

Please provide:
1. Likely pagination patterns
2. Common CSS class/ID patterns
3. Recommended scraping approach
4. Potential anti-bot measures
5. Suggested delays and limits

Base your analysis on common patterns for this type of website."#,
        domain,
        sample_urls.iter()
            .enumerate()
            .map(|(i, url)| format!("{}. {}", i + 1, url))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Build prompt for error diagnosis
pub fn build_error_diagnosis_prompt(error_message: &str, dsl: &ScrapePlan) -> String {
    format!(
        r#"A scraping job failed with this error: "{}"

Scraping plan used:
```yaml
{}
```

Please diagnose the issue and suggest fixes:
1. What likely caused this error?
2. How can the scraping plan be modified to avoid this error?
3. Are there alternative approaches?
4. Should any anti-blocking measures be adjusted?

Provide specific, actionable recommendations."#,
        error_message,
        dsl.to_yaml().unwrap_or_default()
    )
}

/// Build prompt for field extraction optimization
pub fn build_field_optimization_prompt(field_name: &str, current_selector: &str, sample_html: &str) -> String {
    format!(
        r#"Optimize the CSS selector for extracting field "{}".

Current selector: "{}"

Sample HTML where extraction failed:
```html
{}
```

Please suggest:
1. Improved primary selector
2. Fallback selectors
3. Better extraction method if needed
4. Transformations to clean the data

Explain why your suggestions are better than the current approach."#,
        field_name,
        current_selector,
        sample_html
    )
}

/// Build prompt for pagination detection
pub fn build_pagination_detection_prompt(html_content: &str) -> String {
    format!(
        r#"Analyze this HTML content to detect pagination patterns:

```html
{}
```

Identify:
1. Pagination type (links, buttons, infinite scroll, URL patterns)
2. Specific selectors for navigation elements
3. Recommended wait times
4. Maximum pages to scrape safely
5. Any special handling needed

Provide a pagination configuration in the DSL format."#,
        html_content
    )
}

/// Build prompt for robots.txt interpretation
pub fn build_robots_interpretation_prompt(robots_txt: &str, target_paths: &[String]) -> String {
    format!(
        r#"Interpret this robots.txt file for scraping compliance:

```
{}
```

Target paths to scrape:
{}

Please analyze:
1. Which paths are allowed/disallowed for general crawlers?
2. Any specific crawl delays required?
3. Recommended user agent to use?
4. Paths that should be avoided?
5. Overall scraping strategy recommendations?

Provide clear guidance on compliant scraping for these paths."#,
        robots_txt,
        target_paths.iter()
            .enumerate()
            .map(|(i, path)| format!("{}. {}", i + 1, path))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Build prompt for data quality assessment
pub fn build_data_quality_prompt(sample_data: &[serde_json::Value]) -> String {
    let sample_json = serde_json::to_string_pretty(&sample_data).unwrap_or_default();
    
    format!(
        r#"Assess the quality of this scraped data:

```json
{}
```

Evaluate:
1. Data completeness (missing fields, empty values)
2. Data consistency (format variations, duplicates)
3. Data accuracy (obvious errors or anomalies)
4. Suggested improvements to extraction or transformation
5. Additional validation rules needed

Provide specific recommendations for improving data quality."#,
        sample_json
    )
}

/// Build prompt for anti-blocking strategy
pub fn build_anti_blocking_prompt(domain: &str, detected_measures: &[String]) -> String {
    format!(
        r#"Design an anti-blocking strategy for domain: {}

Detected anti-bot measures:
{}

Recommend:
1. Optimal request delays and patterns
2. User agent rotation strategy
3. Header configurations
4. Proxy requirements (if any)
5. Session management approach
6. Fallback strategies if blocked

Balance effectiveness with ethical scraping practices."#,
        domain,
        detected_measures.iter()
            .enumerate()
            .map(|(i, measure)| format!("{}. {}", i + 1, measure))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Template for structured LLM responses
pub struct ResponseTemplate {
    pub format: ResponseFormat,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ResponseFormat {
    YAML,
    JSON,
    PlainText,
    Structured,
}

impl ResponseTemplate {
    /// Create template for DSL generation
    pub fn dsl_generation() -> Self {
        Self {
            format: ResponseFormat::YAML,
            required_fields: vec![
                "version".to_string(),
                "target".to_string(),
                "rules".to_string(),
                "anti_blocking".to_string(),
                "output".to_string(),
            ],
            optional_fields: vec![
                "metadata".to_string(),
            ],
        }
    }
    
    /// Create template for selector suggestion
    pub fn selector_suggestion() -> Self {
        Self {
            format: ResponseFormat::JSON,
            required_fields: vec![
                "primary_selector".to_string(),
                "extraction_method".to_string(),
            ],
            optional_fields: vec![
                "fallback_selector".to_string(),
                "transformations".to_string(),
                "confidence".to_string(),
            ],
        }
    }
    
    /// Create template for error diagnosis
    pub fn error_diagnosis() -> Self {
        Self {
            format: ResponseFormat::Structured,
            required_fields: vec![
                "diagnosis".to_string(),
                "recommendations".to_string(),
            ],
            optional_fields: vec![
                "alternative_approaches".to_string(),
                "prevention_measures".to_string(),
            ],
        }
    }
}

/// Prompt optimization utilities
pub struct PromptOptimizer;

impl PromptOptimizer {
    /// Optimize prompt length while preserving key information
    pub fn optimize_length(prompt: &str, max_tokens: usize) -> String {
        // Simple token estimation (rough approximation)
        let estimated_tokens = prompt.split_whitespace().count();
        
        if estimated_tokens <= max_tokens {
            return prompt.to_string();
        }
        
        // Truncate while preserving structure
        let words: Vec<&str> = prompt.split_whitespace().collect();
        let target_words = (max_tokens as f32 * 0.8) as usize; // Leave some buffer
        
        if words.len() > target_words {
            let truncated = words[..target_words].join(" ");
            format!("{}...\n\n[Content truncated for length]", truncated)
        } else {
            prompt.to_string()
        }
    }
    
    /// Add context markers to improve LLM understanding
    pub fn add_context_markers(prompt: &str) -> String {
        format!(
            "<task>Web Scraping DSL Generation</task>\n<context>\n{}\n</context>\n<instruction>Generate valid YAML following the exact schema shown in examples.</instruction>",
            prompt
        )
    }
    
    /// Enhance prompt with chain-of-thought reasoning
    pub fn add_reasoning_chain(prompt: &str) -> String {
        format!(
            "{}\n\nBefore generating the final YAML, please think through:\n1. What type of website is this?\n2. What are the likely HTML structures?\n3. What selectors would be most reliable?\n4. What anti-blocking measures are appropriate?\n\nThen provide your final YAML configuration:",
            prompt
        )
    }
}
