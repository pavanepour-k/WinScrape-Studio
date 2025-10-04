use anyhow::Result;
use crate::dsl::ScrapePlan;

/// DSL parser for converting various formats
pub struct DSLParser;

impl DSLParser {
    /// Parse DSL from YAML string
    pub fn parse_yaml(yaml: &str) -> Result<ScrapePlan> {
        Ok(serde_yaml::from_str(yaml)?)
    }
    
    /// Parse DSL from JSON string
    pub fn parse_json(json: &str) -> Result<ScrapePlan> {
        Ok(serde_json::from_str(json)?)
    }
    
    /// Convert DSL to YAML
    pub fn to_yaml(dsl: &ScrapePlan) -> Result<String> {
        Ok(serde_yaml::to_string(dsl)?)
    }
    
    /// Convert DSL to JSON
    pub fn to_json(dsl: &ScrapePlan) -> Result<String> {
        Ok(serde_json::to_string_pretty(dsl)?)
    }
}
