
/// LLM processing utilities
pub struct LLMProcessor;

impl LLMProcessor {
    /// Process natural language to extract key information
    pub fn extract_intent(input: &str) -> IntentAnalysis {
        let input_lower = input.to_lowercase();
        
        let mut analysis = IntentAnalysis::default();
        
        // Extract domain
        if let Some(domain) = Self::extract_domain_from_text(&input_lower) {
            analysis.domain = Some(domain);
        }
        
        // Enhanced scraping type detection with confidence scoring
        let (scraping_type, confidence) = Self::detect_scraping_type(&input_lower);
        analysis.scraping_type = scraping_type;
        analysis.confidence = confidence;
        
        // Extract fields
        analysis.fields = Self::extract_fields_from_text(&input_lower);
        
        analysis
    }
    
    /// Detect scraping type with confidence scoring
    fn detect_scraping_type(input: &str) -> (ScrapingType, f32) {
        let mut scores = std::collections::HashMap::new();
        
        // E-commerce indicators
        let ecommerce_keywords = [
            "product", "price", "cost", "buy", "purchase", "shop", "store", "cart", 
            "checkout", "ecommerce", "e-commerce", "retail", "sale", "discount",
            "inventory", "stock", "shipping", "delivery", "order", "payment"
        ];
        let ecommerce_score = ecommerce_keywords.iter()
            .map(|&keyword| if input.contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / ecommerce_keywords.len() as f32;
        scores.insert(ScrapingType::Ecommerce, ecommerce_score);
        
        // News indicators
        let news_keywords = [
            "news", "article", "story", "blog", "post", "headline", "journalism",
            "report", "breaking", "update", "press", "media", "publication",
            "editorial", "opinion", "analysis", "coverage"
        ];
        let news_score = news_keywords.iter()
            .map(|&keyword| if input.contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / news_keywords.len() as f32;
        scores.insert(ScrapingType::News, news_score);
        
        // Directory indicators
        let directory_keywords = [
            "directory", "listing", "business", "contact", "address", "phone",
            "email", "company", "organization", "yellow pages", "catalog",
            "registry", "database", "index", "guide"
        ];
        let directory_score = directory_keywords.iter()
            .map(|&keyword| if input.contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / directory_keywords.len() as f32;
        scores.insert(ScrapingType::Directory, directory_score);
        
        // Social media indicators
        let social_keywords = [
            "social", "post", "comment", "profile", "user", "follower", "like",
            "share", "tweet", "facebook", "twitter", "instagram", "linkedin",
            "community", "forum", "discussion", "chat"
        ];
        let social_score = social_keywords.iter()
            .map(|&keyword| if input.contains(keyword) { 1.0 } else { 0.0 })
            .sum::<f32>() / social_keywords.len() as f32;
        scores.insert(ScrapingType::Social, social_score);
        
        // Find the type with highest score
        let (best_type, best_score) = scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap_or((&ScrapingType::Generic, &0.0));
        
        // If no clear winner, use generic
        if *best_score < 0.1 {
            (ScrapingType::Generic, 0.5)
        } else {
            (*best_type, (*best_score * 0.8 + 0.2).min(1.0)) // Normalize confidence
        }
    }
    
    fn extract_domain_from_text(text: &str) -> Option<String> {
        // Simple domain extraction
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if word.contains('.') && (word.contains(".com") || word.contains(".org") || word.contains(".net")) {
                return Some(word.to_string());
            }
        }
        None
    }
    
    fn extract_fields_from_text(text: &str) -> Vec<String> {
        let mut fields = Vec::new();
        
        // Enhanced field extraction with more patterns
        let field_patterns: &[(&str, &[&str])] = &[
            ("title", &["title", "name", "headline", "heading", "subject"]),
            ("price", &["price", "cost", "amount", "fee", "rate", "charge"]),
            ("description", &["description", "summary", "details", "content", "text", "body"]),
            ("image", &["image", "photo", "picture", "img", "thumbnail", "screenshot"]),
            ("url", &["url", "link", "href", "website", "page"]),
            ("date", &["date", "time", "timestamp", "created", "published", "updated"]),
            ("author", &["author", "writer", "creator", "byline", "publisher"]),
            ("category", &["category", "tag", "type", "class", "genre", "section"]),
            ("rating", &["rating", "score", "review", "stars", "grade"]),
            ("location", &["location", "address", "place", "city", "country"]),
            ("contact", &["contact", "phone", "email", "address", "telephone"]),
            ("status", &["status", "state", "condition", "availability"]),
        ];
        
        for (field_name, keywords) in field_patterns.iter() {
            if keywords.iter().any(|&keyword| text.contains(keyword)) {
                fields.push(field_name.to_string());
            }
        }
        
        // Remove duplicates while preserving order
        let mut unique_fields = Vec::new();
        for field in fields {
            if !unique_fields.contains(&field) {
                unique_fields.push(field);
            }
        }
        
        unique_fields
    }
}

#[derive(Debug, Clone)]
pub struct IntentAnalysis {
    pub domain: Option<String>,
    pub scraping_type: ScrapingType,
    pub fields: Vec<String>,
    pub confidence: f32,
}

impl Default for IntentAnalysis {
    fn default() -> Self {
        Self {
            domain: None,
            scraping_type: ScrapingType::Generic,
            fields: Vec::new(),
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrapingType {
    Generic,
    Ecommerce,
    News,
    Directory,
    Social,
}
