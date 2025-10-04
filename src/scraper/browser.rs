#[cfg(feature = "browser")]
use anyhow::Result;
#[cfg(feature = "browser")]
use playwright::Playwright;
#[cfg(feature = "browser")]
use serde_json::Value;
#[cfg(feature = "browser")]
use std::time::Duration;
#[cfg(feature = "browser")]
use tracing::{debug, warn, error};
#[cfg(feature = "browser")]
use url::Url;

#[cfg(feature = "browser")]
use crate::config::ScrapingConfig;
#[cfg(feature = "browser")]
use crate::dsl::ScrapePlan;

/// Browser-based scraping client using Playwright
#[cfg(feature = "browser")]
pub struct BrowserClient {
    config: ScrapingConfig,
    playwright: Playwright,
}

// Ensure BrowserClient is Send + Sync
#[cfg(feature = "browser")]
unsafe impl Send for BrowserClient {}
#[cfg(feature = "browser")]
unsafe impl Sync for BrowserClient {}

#[cfg(feature = "browser")]
impl BrowserClient {
    /// Create new browser client
    pub async fn new(config: &ScrapingConfig) -> Result<Self> {
        debug!("Initializing browser client");
        
        let playwright = Playwright::initialize().await?;
        
        Ok(Self {
            config: config.clone(),
            playwright,
        })
    }
    
    /// Scrape URL using browser
    pub async fn scrape_url(&self, url: &Url, plan: &ScrapePlan) -> Result<Vec<Value>> {
        debug!("Browser scraping URL: {}", url);
        
        // Launch browser
        let browser = self.playwright
            .chromium()
            .launcher()
            .headless(true)
            .launch()
            .await?;
        
        // Create context with custom settings
        let context = browser
            .context_builder()
            .user_agent(&self.get_user_agent())
            .viewport(Some(playwright::api::Viewport { width: 1920, height: 1080 }))
            .build()
            .await?;
        
        // Create page
        let page = context.new_page().await?;
        
        // Set timeout
        page.set_default_timeout(self.config.browser_timeout_seconds as u32);
        
        let result = async {
            // Navigate to page
            page.goto_builder(url.as_str())
                .goto()
                .await?;
            
            // Wait for content to load
            tokio::time::sleep(Duration::from_millis(2000)).await;
            
            // Handle pagination if needed
            if let Some(pagination) = &plan.rules.pagination {
                self.handle_pagination(&page, pagination).await?;
            }
            
            // Extract data
            let items = self.extract_data_from_page(&page, plan).await?;
            
            Ok::<Vec<Value>, anyhow::Error>(items)
        }.await;
        
        // Clean up
        if let Err(e) = browser.close().await {
            warn!("Failed to close browser: {}", e);
        }
        
        result
    }
    
    /// Handle pagination in browser
    async fn handle_pagination(
        &self,
        page: &playwright::api::Page,
        pagination: &crate::dsl::Pagination,
    ) -> Result<()> {
        use crate::dsl::PaginationMethod;
        
        let max_pages = pagination.max_pages.unwrap_or(10);
        let wait_time = Duration::from_millis(pagination.wait_time_ms.unwrap_or(2000));
        
        match &pagination.method {
            PaginationMethod::Link { next_selector } => {
                for page_num in 1..max_pages {
                    debug!("Handling pagination page {}", page_num);
                    
                    // Check if next link exists
                    if page.query_selector(next_selector).await?.is_none() {
                        debug!("No more pages found");
                        break;
                    }
                    
                    // Click next link
                    page.evaluate::<(), ()>(&format!("document.querySelector('{}').click()", next_selector), ()).await?;
                    
                    // Wait for page to load
                    tokio::time::sleep(wait_time).await;
                }
            }
            PaginationMethod::Button { button_selector } => {
                for page_num in 1..max_pages {
                    debug!("Clicking load more button, page {}", page_num);
                    
                    // Check if button exists and is visible
                    if page.query_selector(button_selector).await?.is_none() {
                        debug!("Load more button not found");
                        break;
                    }
                    
                    // Click button
                    if let Err(e) = page.evaluate::<(), ()>(&format!("document.querySelector('{}').click()", button_selector), ()).await {
                        debug!("Failed to click load more button: {}", e);
                        break;
                    }
                    
                    // Wait for content to load
                    tokio::time::sleep(wait_time).await;
                }
            }
            PaginationMethod::Scroll { scroll_pause_ms } => {
                let scroll_pause = Duration::from_millis(*scroll_pause_ms);
                
                for _ in 0..max_pages {
                    // Get current page height
                    let current_height: i32 = page
                        .evaluate::<(), serde_json::Value>("document.body.scrollHeight", ())
                        .await?
                        .as_i64()
                        .unwrap_or(0) as i32;
                    
                    // Scroll to bottom
                    page.evaluate::<(), ()>("window.scrollTo(0, document.body.scrollHeight)", ())
                        .await?;
                    
                    // Wait for new content
                    tokio::time::sleep(scroll_pause).await;
                    
                    // Check if page height changed
                    let new_height: i32 = page
                        .evaluate::<(), serde_json::Value>("document.body.scrollHeight", ())
                        .await?
                        .as_i64()
                        .unwrap_or(0) as i32;
                    
                    if new_height <= current_height {
                        debug!("No more content loaded after scrolling");
                        break;
                    }
                }
            }
            PaginationMethod::UrlPattern { .. } => {
                // URL pattern pagination is handled at the URL level, not in browser
                debug!("URL pattern pagination not applicable in browser context");
            }
        }
        
        Ok(())
    }
    
    /// Extract data from page
    async fn extract_data_from_page(&self, page: &playwright::api::Page, plan: &ScrapePlan) -> Result<Vec<Value>> {
        debug!("Extracting data from page");
        
        // Build JavaScript extraction script
        let extraction_script = self.build_extraction_script(plan)?;
        
        // Execute extraction
        let result: serde_json::Value = page.evaluate(&extraction_script, ()).await?;
        
        // Parse result
        if let Some(array) = result.as_array() {
            Ok(array.clone())
        } else {
            Ok(vec![])
        }
    }
    
    /// Build JavaScript extraction script
    fn build_extraction_script(&self, plan: &ScrapePlan) -> Result<String> {
        let mut script = String::new();
        
        script.push_str("(function() {\n");
        script.push_str("  const results = [];\n");
        script.push_str(&format!("  const items = document.querySelectorAll('{}');\n", plan.rules.item_selector));
        script.push_str("  \n");
        script.push_str("  for (const item of items) {\n");
        script.push_str("    const data = {};\n");
        
        // Add field extractions
        for field in &plan.rules.fields {
            let field_script = self.build_field_extraction_script(field)?;
            script.push_str(&field_script);
        }
        
        // Add metadata
        script.push_str("    data._source_url = window.location.href;\n");
        script.push_str("    data._scraped_at = new Date().toISOString();\n");
        script.push_str("    data._method = 'browser';\n");
        
        script.push_str("    results.push(data);\n");
        script.push_str("  }\n");
        script.push_str("  \n");
        script.push_str("  return results;\n");
        script.push_str("})()");
        
        Ok(script)
    }
    
    /// Build field extraction script
    fn build_field_extraction_script(&self, field: &crate::dsl::Field) -> Result<String> {
        use crate::dsl::{SelectorType, ExtractionMethod};
        
        let mut script = String::new();
        
        // Only CSS selectors supported in browser
        if matches!(field.selector_type, SelectorType::XPath) {
            return Err(anyhow::anyhow!("XPath selectors not supported in browser mode"));
        }
        
        script.push_str("    try {\n");
        script.push_str(&format!("      const element = item.querySelector('{}');\n", field.selector));
        script.push_str("      if (element) {\n");
        
        let extraction_code = match &field.extraction {
            ExtractionMethod::Text => "element.textContent || ''".to_string(),
            ExtractionMethod::Html => "element.innerHTML || ''".to_string(),
            ExtractionMethod::Attribute { name } => format!("element.getAttribute('{}') || ''", name),
            ExtractionMethod::Href => "element.href || element.getAttribute('href') || ''".to_string(),
            ExtractionMethod::Src => "element.src || element.getAttribute('src') || ''".to_string(),
        };
        
        script.push_str(&format!("        let value = {};\n", extraction_code));
        
        // Apply transformations
        if let Some(transforms) = &field.transform {
            for transform in transforms {
                let transform_code = self.build_transform_script(transform)?;
                script.push_str(&format!("        value = {};\n", transform_code));
            }
        }
        
        script.push_str(&format!("        data['{}'] = value;\n", field.name));
        script.push_str("      }\n");
        script.push_str("    } catch (e) {\n");
        script.push_str(&format!("      console.warn('Failed to extract field {}: ' + e.message);\n", field.name));
        script.push_str("    }\n");
        
        Ok(script)
    }
    
    /// Build transform script
    fn build_transform_script(&self, transform: &crate::dsl::Transform) -> Result<String> {
        use crate::dsl::Transform;
        
        let script = match transform {
            Transform::Trim => "value.trim()".to_string(),
            Transform::Lowercase => "value.toLowerCase()".to_string(),
            Transform::Uppercase => "value.toUpperCase()".to_string(),
            Transform::Regex { pattern, replacement } => {
                format!("value.replace(new RegExp('{}', 'g'), '{}')", pattern, replacement)
            }
            Transform::ParseNumber => "parseFloat(value) || value".to_string(),
            Transform::ParseDate { .. } => "value".to_string(), // Keep as string for now
            Transform::RemoveHtml => "value.replace(/<[^>]*>/g, '')".to_string(),
            Transform::ExtractDomain => {
                "(() => { try { return new URL(value).hostname; } catch { return value; } })()".to_string()
            }
        };
        
        Ok(script)
    }
    
    /// Get user agent for browser
    fn get_user_agent(&self) -> String {
        self.config.user_agents.first()
            .cloned()
            .unwrap_or_else(|| "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string())
    }
}

// Stub implementation when browser feature is disabled
#[cfg(not(feature = "browser"))]
pub struct BrowserClient;

#[cfg(not(feature = "browser"))]
impl BrowserClient {
    pub async fn new(_config: &crate::config::ScrapingConfig) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Browser feature not enabled"))
    }
    
    pub async fn scrape_url(&self, _url: &url::Url, _plan: &crate::dsl::ScrapePlan) -> anyhow::Result<Vec<serde_json::Value>> {
        Err(anyhow::anyhow!("Browser feature not enabled"))
    }
}
