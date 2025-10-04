/// String utility functions
pub struct StringUtils;

impl StringUtils {
    /// Clean and normalize string
    pub fn normalize(s: &str) -> String {
        s.trim().to_string()
    }
    
    /// Truncate string to max length
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
    
    /// Check if string is empty or whitespace only
    pub fn is_blank(s: &str) -> bool {
        s.trim().is_empty()
    }
}
