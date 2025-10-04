use regex::Regex;
use anyhow::Result;
use tracing::{warn, error, debug};
use crate::dsl::{ScrapePlan, Target, Rules, Field, SelectorType, ExtractionMethod, Transform, Pagination, PaginationMethod, AntiBlocking, Output, OutputFormat, Filter, FilterCondition};
use url::Url;

/// DSL validator for comprehensive validation of scrape plans
pub struct DSLValidator {
    css_selector_regex: Regex,
    xpath_regex: Regex,
    url_regex: Regex,
}

impl DSLValidator {
    pub fn new() -> Self {
        Self {
            css_selector_regex: Regex::new(r"^[a-zA-Z0-9\s\.\#\[\]\:\(\)\-_,>~\+\*=\^$|""']+$").unwrap(),
            xpath_regex: Regex::new(r"^[a-zA-Z0-9\s/\.\@\[\]\(\)\-_=\^$|""':]+$").unwrap(),
            url_regex: Regex::new(r"^https?://[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,}(/.*)?$").unwrap(),
        }
    }
    
    /// Validate a complete scrape plan
    pub fn validate(&self, plan: &ScrapePlan) -> Result<()> {
        debug!("Starting DSL validation for plan: {}", plan.target.domain);
        
        // Validate version
        self.validate_version(&plan.version)?;
        
        // Validate target
        self.validate_target(&plan.target)?;
        
        // Validate rules
        self.validate_rules(&plan.rules)?;
        
        // Validate anti-blocking settings
        self.validate_anti_blocking(&plan.anti_blocking)?;
        
        // Validate output configuration
        self.validate_output(&plan.output)?;
        
        // Cross-reference validation
        self.validate_cross_references(plan)?;
        
        debug!("DSL validation completed successfully");
        Ok(())
    }
    
    fn validate_version(&self, version: &str) -> Result<()> {
        if version.is_empty() {
            return Err(anyhow::anyhow!("Version cannot be empty"));
        }
        
        // Check if version follows semantic versioning
        if !version.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            return Err(anyhow::anyhow!("Invalid version format: {}", version));
        }
        
        Ok(())
    }
    
    fn validate_target(&self, target: &Target) -> Result<()> {
        if target.domain.is_empty() {
            return Err(anyhow::anyhow!("Domain cannot be empty"));
        }
        
        if target.start_urls.is_empty() {
            return Err(anyhow::anyhow!("At least one start URL is required"));
        }
        
        // Validate domain format
        if !self.url_regex.is_match(&format!("https://{}", target.domain)) {
            return Err(anyhow::anyhow!("Invalid domain format: {}", target.domain));
        }
        
        // Validate start URLs
        for url_str in &target.start_urls {
            match Url::parse(url_str) {
                Ok(url) => {
                    if !url.scheme().starts_with("http") {
                        return Err(anyhow::anyhow!("URL must use HTTP or HTTPS scheme: {}", url_str));
                    }
                    
                    if let Some(host) = url.host_str() {
                        if !host.ends_with(&target.domain) {
                            warn!("URL host '{}' does not match target domain '{}'", host, target.domain);
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Invalid URL '{}': {}", url_str, e));
                }
            }
        }
        
        // Validate URL patterns if provided
        if let Some(patterns) = &target.url_patterns {
            for pattern in patterns {
                self.validate_url_pattern(pattern)?;
            }
        }
        
        // Validate max_pages if provided
        if let Some(max_pages) = target.max_pages {
            if max_pages == 0 {
                return Err(anyhow::anyhow!("Max pages must be greater than 0"));
            }
            if max_pages > 10000 {
                warn!("Very high max_pages value: {}. This may cause performance issues.", max_pages);
            }
        }
        
        Ok(())
    }
    
    fn validate_url_pattern(&self, pattern: &str) -> Result<()> {
        if pattern.is_empty() {
            return Err(anyhow::anyhow!("URL pattern cannot be empty"));
        }
        
        // Check for basic URL pattern validity
        if !pattern.contains("http") && !pattern.contains("https") {
            return Err(anyhow::anyhow!("URL pattern must contain http or https: {}", pattern));
        }
        
        // Check for dangerous patterns
        if pattern.contains("file://") || pattern.contains("ftp://") {
            return Err(anyhow::anyhow!("Unsafe URL scheme in pattern: {}", pattern));
        }
        
        Ok(())
    }
    
    fn validate_rules(&self, rules: &Rules) -> Result<()> {
        if rules.item_selector.is_empty() {
            return Err(anyhow::anyhow!("Item selector cannot be empty"));
        }
        
        if rules.fields.is_empty() {
            return Err(anyhow::anyhow!("At least one field must be defined"));
        }
        
        // Validate item selector
        self.validate_selector(&rules.item_selector, &SelectorType::CSS)?;
        
        // Validate fields
        let mut field_names = std::collections::HashSet::new();
        for field in &rules.fields {
            self.validate_field(field)?;
            
            if !field_names.insert(&field.name) {
                return Err(anyhow::anyhow!("Duplicate field name: {}", field.name));
            }
        }
        
        // Validate pagination if provided
        if let Some(pagination) = &rules.pagination {
            self.validate_pagination(pagination)?;
        }
        
        // Validate filters if provided
        if let Some(filters) = &rules.filters {
            for filter in filters {
                self.validate_filter(filter, &field_names)?;
            }
        }
        
        Ok(())
    }
    
    fn validate_field(&self, field: &Field) -> Result<()> {
        if field.name.is_empty() {
            return Err(anyhow::anyhow!("Field name cannot be empty"));
        }
        
        if field.selector.is_empty() {
            return Err(anyhow::anyhow!("Field selector cannot be empty"));
        }
        
        // Validate selector
        self.validate_selector(&field.selector, &field.selector_type)?;
        
        // Validate extraction method
        self.validate_extraction_method(&field.extraction, &field.selector_type)?;
        
        // Validate transforms if provided
        if let Some(transforms) = &field.transform {
            for transform in transforms {
                self.validate_transform(transform)?;
            }
        }
        
        Ok(())
    }
    
    fn validate_selector(&self, selector: &str, selector_type: &SelectorType) -> Result<()> {
        if selector.is_empty() {
            return Err(anyhow::anyhow!("Selector cannot be empty"));
        }
        
        match selector_type {
            SelectorType::CSS => {
                if !self.css_selector_regex.is_match(selector) {
                    return Err(anyhow::anyhow!("Invalid CSS selector: {}", selector));
                }
                
                // Check for potentially dangerous selectors
                if selector.contains("script") || selector.contains("style") {
                    warn!("Selector contains potentially dangerous elements: {}", selector);
                }
                
                // Check for overly complex selectors
                if selector.len() > 200 {
                    warn!("Very long CSS selector: {} characters", selector.len());
                }
            }
            SelectorType::XPath => {
                if !self.xpath_regex.is_match(selector) {
                    return Err(anyhow::anyhow!("Invalid XPath selector: {}", selector));
                }
                
                // Check for potentially dangerous XPath expressions
                if selector.contains("//script") || selector.contains("//style") {
                    warn!("XPath selector contains potentially dangerous elements: {}", selector);
                }
                
                // Check for overly complex XPath
                if selector.len() > 500 {
                    warn!("Very long XPath selector: {} characters", selector.len());
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_extraction_method(
        &self,
        extraction: &ExtractionMethod,
        _selector_type: &SelectorType,
    ) -> Result<()> {
        match extraction {
            ExtractionMethod::Text => Ok(()),
            ExtractionMethod::Html => Ok(()),
            ExtractionMethod::Attribute { name } => {
                if name.is_empty() {
                    return Err(anyhow::anyhow!("Attribute name cannot be empty"));
                }
                if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                    return Err(anyhow::anyhow!("Invalid attribute name: {}", name));
                }
                Ok(())
            }
            ExtractionMethod::Href => Ok(()),
            ExtractionMethod::Src => Ok(()),
        }
    }
    
    fn validate_transform(&self, transform: &Transform) -> Result<()> {
        match transform {
            Transform::Trim => Ok(()),
            Transform::Lowercase => Ok(()),
            Transform::Uppercase => Ok(()),
            Transform::Regex { pattern, replacement } => {
                if pattern.is_empty() {
                    return Err(anyhow::anyhow!("Regex pattern cannot be empty"));
                }
                // Validate regex pattern
                if Regex::new(pattern).is_err() {
                    return Err(anyhow::anyhow!("Invalid regex pattern: {}", pattern));
                }
                Ok(())
            }
            Transform::ParseNumber => Ok(()),
            Transform::ParseDate { format } => {
                if let Some(format) = format {
                    if format.is_empty() {
                        return Err(anyhow::anyhow!("Date format cannot be empty"));
                    }
                }
                Ok(())
            }
            Transform::RemoveHtml => Ok(()),
            Transform::ExtractDomain => Ok(()),
        }
    }
    
    fn validate_pagination(&self, pagination: &Pagination) -> Result<()> {
        match &pagination.method {
            PaginationMethod::Link { next_selector } => {
                if next_selector.is_empty() {
                    return Err(anyhow::anyhow!("Next selector cannot be empty for link pagination"));
                }
                self.validate_selector(next_selector, &SelectorType::CSS)?;
            }
            PaginationMethod::Button { button_selector } => {
                if button_selector.is_empty() {
                    return Err(anyhow::anyhow!("Button selector cannot be empty for button pagination"));
                }
                self.validate_selector(button_selector, &SelectorType::CSS)?;
            }
            PaginationMethod::Scroll { scroll_pause_ms } => {
                if *scroll_pause_ms == 0 {
                    return Err(anyhow::anyhow!("Scroll pause time must be greater than 0"));
                }
                if *scroll_pause_ms > 10000 {
                    warn!("Very long scroll pause time: {}ms", scroll_pause_ms);
                }
            }
            PaginationMethod::UrlPattern { pattern, start, end } => {
                if pattern.is_empty() {
                    return Err(anyhow::anyhow!("URL pattern cannot be empty"));
                }
                if start >= end {
                    return Err(anyhow::anyhow!("Start value must be less than end value"));
                }
                if end - start > 1000 {
                    warn!("Very large pagination range: {} to {}", start, end);
                }
            }
        }
        
        // Validate selector if provided
        if let Some(selector) = &pagination.selector {
            self.validate_selector(selector, &SelectorType::CSS)?;
        }
        
        // Validate max_pages if provided
        if let Some(max_pages) = pagination.max_pages {
            if max_pages == 0 {
                return Err(anyhow::anyhow!("Max pages must be greater than 0"));
            }
            if max_pages > 1000 {
                warn!("Very high max_pages value: {}. This may cause performance issues.", max_pages);
            }
        }
        
        // Validate wait_time if provided
        if let Some(wait_time) = pagination.wait_time_ms {
            if wait_time > 30000 {
                warn!("Very long wait time: {}ms", wait_time);
            }
        }
        
        Ok(())
    }
    
    fn validate_filter(
        &self,
        filter: &Filter,
        field_names: &std::collections::HashSet<&String>,
    ) -> Result<()> {
        if !field_names.contains(&filter.field) {
            return Err(anyhow::anyhow!("Filter references unknown field: {}", filter.field));
        }
        
        match &filter.condition {
            FilterCondition::Contains { value } => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("Contains filter value cannot be empty"));
                }
            }
            FilterCondition::NotContains { value } => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("NotContains filter value cannot be empty"));
                }
            }
            FilterCondition::Equals { value } => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("Equals filter value cannot be empty"));
                }
            }
            FilterCondition::NotEquals { value } => {
                if value.is_empty() {
                    return Err(anyhow::anyhow!("NotEquals filter value cannot be empty"));
                }
            }
            FilterCondition::Regex { pattern } => {
                if pattern.is_empty() {
                    return Err(anyhow::anyhow!("Regex pattern cannot be empty"));
                }
                if Regex::new(pattern).is_err() {
                    return Err(anyhow::anyhow!("Invalid regex pattern: {}", pattern));
                }
            }
            FilterCondition::LengthMin { min } => {
                if *min == 0 {
                    return Err(anyhow::anyhow!("Length minimum must be greater than 0"));
                }
            }
            FilterCondition::LengthMax { max } => {
                if *max == 0 {
                    return Err(anyhow::anyhow!("Length maximum must be greater than 0"));
                }
            }
            FilterCondition::NotEmpty => Ok(()),
        }
        
        Ok(())
    }
    
    fn validate_anti_blocking(&self, anti_blocking: &AntiBlocking) -> Result<()> {
        // Validate delay configuration
        if anti_blocking.randomized_delays.min_ms > anti_blocking.randomized_delays.max_ms {
            return Err(anyhow::anyhow!("Min delay cannot be greater than max delay"));
        }
        
        if anti_blocking.randomized_delays.min_ms == 0 && anti_blocking.randomized_delays.max_ms == 0 {
            warn!("No delays configured - this may trigger rate limiting");
        }
        
        if anti_blocking.randomized_delays.max_ms > 30000 {
            warn!("Very long delay configured: {}ms", anti_blocking.randomized_delays.max_ms);
        }
        
        // Validate proxy configuration if provided
        if let Some(proxy) = &anti_blocking.proxy {
            if proxy.enabled && proxy.proxies.is_empty() {
                return Err(anyhow::anyhow!("Proxy is enabled but no proxies are configured"));
            }
            
            for proxy_url in &proxy.proxies {
                if !proxy_url.starts_with("http://") && !proxy_url.starts_with("https://") {
                    return Err(anyhow::anyhow!("Invalid proxy URL format: {}", proxy_url));
                }
            }
        }
        
        // Validate headers if provided
        if let Some(headers) = &anti_blocking.headers {
            for (name, value) in headers {
                if name.is_empty() {
                    return Err(anyhow::anyhow!("Header name cannot be empty"));
                }
                if value.is_empty() {
                    return Err(anyhow::anyhow!("Header value cannot be empty"));
                }
                
                // Check for potentially dangerous headers
                let dangerous_headers = ["authorization", "cookie", "x-api-key"];
                if dangerous_headers.contains(&name.to_lowercase().as_str()) {
                    warn!("Potentially sensitive header configured: {}", name);
                }
            }
        }
        
        Ok(())
    }
    
    fn validate_output(&self, output: &Output) -> Result<()> {
        if output.format.is_empty() {
            return Err(anyhow::anyhow!("At least one output format must be specified"));
        }
        
        for format in &output.format {
            match format {
                OutputFormat::CSV => Ok(()),
                OutputFormat::JSON => Ok(()),
                OutputFormat::XLSX => Ok(()),
                OutputFormat::Parquet => Ok(()),
            }?;
        }
        
        // Validate limit if provided
        if let Some(limit) = output.limit {
            if limit == 0 {
                return Err(anyhow::anyhow!("Output limit must be greater than 0"));
            }
            if limit > 1000000 {
                warn!("Very high output limit: {}. This may cause memory issues.", limit);
            }
        }
        
        // Validate dedupe keys if provided
        if let Some(dedupe_keys) = &output.dedupe_keys {
            if dedupe_keys.is_empty() {
                return Err(anyhow::anyhow!("Dedupe keys cannot be empty"));
            }
        }
        
        // Validate sort configuration if provided
        if let Some(sort_by) = &output.sort_by {
            if sort_by.is_empty() {
                return Err(anyhow::anyhow!("Sort field cannot be empty"));
            }
        }
        
        Ok(())
    }
    
    fn validate_cross_references(&self, plan: &ScrapePlan) -> Result<()> {
        // Check if all referenced fields in filters exist
        if let Some(filters) = &plan.rules.filters {
            let field_names: std::collections::HashSet<String> = plan.rules.fields
                .iter()
                .map(|f| f.name.clone())
                .collect();
            
            for filter in filters {
                if !field_names.contains(&filter.field) {
                    return Err(anyhow::anyhow!("Filter references unknown field: {}", filter.field));
                }
            }
        }
        
        // Check if sort field exists in output configuration
        if let Some(sort_by) = &plan.output.sort_by {
            let field_names: std::collections::HashSet<String> = plan.rules.fields
                .iter()
                .map(|f| f.name.clone())
                .collect();
            
            if !field_names.contains(sort_by) {
                return Err(anyhow::anyhow!("Sort field '{}' does not exist in field definitions", sort_by));
            }
        }
        
        // Check if dedupe keys exist in field definitions
        if let Some(dedupe_keys) = &plan.output.dedupe_keys {
            let field_names: std::collections::HashSet<String> = plan.rules.fields
                .iter()
                .map(|f| f.name.clone())
                .collect();
            
            for key in dedupe_keys {
                if !field_names.contains(key) {
                    return Err(anyhow::anyhow!("Dedupe key '{}' does not exist in field definitions", key));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate DSL for security concerns
    pub fn validate_security(&self, plan: &ScrapePlan) -> Result<()> {
        // Check for potentially dangerous domains
        if plan.target.domain.contains("localhost") || plan.target.domain.contains("127.0.0.1") {
            warn!("Scraping localhost or local IP addresses");
        }
        
        // Check for potentially dangerous selectors
        for field in &plan.rules.fields {
            if field.selector.contains("script") || field.selector.contains("style") {
                warn!("Field '{}' uses potentially dangerous selector: {}", field.name, field.selector);
            }
        }
        
        // Check for excessive pagination
        if let Some(pagination) = &plan.rules.pagination {
            if let Some(max_pages) = pagination.max_pages {
                if max_pages > 1000 {
                    warn!("High pagination limit: {} pages", max_pages);
                }
            }
        }
        
        Ok(())
    }
}

impl Default for DSLValidator {
    fn default() -> Self {
        Self::new()
    }
}
