/// Validation utility functions
pub struct ValidationUtils;

impl ValidationUtils {
    /// Validate email format
    pub fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }
    
    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }
    
    /// Validate domain format
    pub fn is_valid_domain(domain: &str) -> bool {
        !domain.is_empty() && domain.contains('.')
    }
}
