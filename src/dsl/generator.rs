use anyhow::Result;
use crate::dsl::{ScrapePlan, Target, Field, SelectorType, ExtractionMethod};

/// DSL generator for creating scraping plans programmatically
pub struct DSLGenerator;

impl DSLGenerator {
    /// Generate basic DSL for a domain
    pub fn generate_basic(domain: &str, start_url: &str) -> Result<ScrapePlan> {
        let mut plan = ScrapePlan::default();
        
        plan.target = Target {
            domain: domain.to_string(),
            start_urls: vec![start_url.to_string()],
            url_patterns: None,
            max_pages: Some(10),
        };
        
        // Add basic fields
        plan.rules.fields = vec![
            Field {
                name: "title".to_string(),
                selector: "h1, h2, .title, [class*='title']".to_string(),
                selector_type: SelectorType::CSS,
                extraction: ExtractionMethod::Text,
                required: false,
                transform: None,
            },
            Field {
                name: "link".to_string(),
                selector: "a".to_string(),
                selector_type: SelectorType::CSS,
                extraction: ExtractionMethod::Href,
                required: false,
                transform: None,
            },
        ];
        
        Ok(plan)
    }
    
    /// Generate e-commerce DSL
    pub fn generate_ecommerce(domain: &str, start_url: &str) -> Result<ScrapePlan> {
        let mut plan = Self::generate_basic(domain, start_url)?;
        
        // Override with e-commerce specific fields
        plan.rules.fields = vec![
            Field {
                name: "product_name".to_string(),
                selector: ".product-title, .product-name, h1".to_string(),
                selector_type: SelectorType::CSS,
                extraction: ExtractionMethod::Text,
                required: true,
                transform: None,
            },
            Field {
                name: "price".to_string(),
                selector: ".price, .cost, [class*='price']".to_string(),
                selector_type: SelectorType::CSS,
                extraction: ExtractionMethod::Text,
                required: false,
                transform: None,
            },
            Field {
                name: "image".to_string(),
                selector: ".product-image img, .product-photo img".to_string(),
                selector_type: SelectorType::CSS,
                extraction: ExtractionMethod::Src,
                required: false,
                transform: None,
            },
        ];
        
        plan.rules.item_selector = ".product, .product-item, [class*='product']".to_string();
        
        Ok(plan)
    }
}
