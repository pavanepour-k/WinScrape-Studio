use anyhow::Result;
use std::collections::HashSet;
use tracing::{debug, warn};

/// Domain whitelist/blacklist manager
pub struct DomainWhitelist {
    blocked_domains: HashSet<String>,
    allowed_domains: Option<HashSet<String>>,
}

impl DomainWhitelist {
    /// Create new domain whitelist
    pub fn new(blocked_domains: &[String]) -> Result<Self> {
        let blocked_domains = blocked_domains.iter().cloned().collect();
        
        Ok(Self {
            blocked_domains,
            allowed_domains: None,
        })
    }
    
    /// Create with both blocked and allowed domains
    pub fn new_with_allowlist(
        blocked_domains: &[String],
        allowed_domains: &[String],
    ) -> Result<Self> {
        let blocked_domains = blocked_domains.iter().cloned().collect();
        let allowed_domains = Some(allowed_domains.iter().cloned().collect());
        
        Ok(Self {
            blocked_domains,
            allowed_domains,
        })
    }
    
    /// Check if domain is blocked
    pub fn is_blocked(&self, domain: &str) -> bool {
        let normalized_domain = self.normalize_domain(domain);
        
        // Check exact match
        if self.blocked_domains.contains(&normalized_domain) {
            return true;
        }
        
        // Check subdomain matches
        for blocked in &self.blocked_domains {
            if normalized_domain.ends_with(&format!(".{}", blocked)) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if domain is allowed
    pub fn is_allowed(&self, domain: &str) -> bool {
        let normalized_domain = self.normalize_domain(domain);
        
        // If there's an allowlist, domain must be in it
        if let Some(allowed) = &self.allowed_domains {
            // Check exact match
            if allowed.contains(&normalized_domain) {
                return true;
            }
            
            // Check subdomain matches
            for allowed_domain in allowed {
                if normalized_domain.ends_with(&format!(".{}", allowed_domain)) {
                    return true;
                }
            }
            
            return false;
        }
        
        // If no allowlist, just check it's not blocked
        !self.is_blocked(domain)
    }
    
    /// Validate domain for scraping
    pub fn validate_domain(&self, domain: &str) -> Result<()> {
        debug!("Validating domain: {}", domain);
        
        if self.is_blocked(domain) {
            warn!("Domain is blocked: {}", domain);
            return Err(anyhow::anyhow!("Domain '{}' is blocked", domain));
        }
        
        if !self.is_allowed(domain) {
            warn!("Domain is not in allowlist: {}", domain);
            return Err(anyhow::anyhow!("Domain '{}' is not allowed", domain));
        }
        
        debug!("Domain validation passed: {}", domain);
        Ok(())
    }
    
    /// Normalize domain (remove protocol, www, etc.)
    fn normalize_domain(&self, domain: &str) -> String {
        let mut normalized = domain.to_lowercase();
        
        // Remove protocol
        if normalized.starts_with("http://") {
            normalized = normalized[7..].to_string();
        } else if normalized.starts_with("https://") {
            normalized = normalized[8..].to_string();
        }
        
        // Remove www prefix
        if normalized.starts_with("www.") {
            normalized = normalized[4..].to_string();
        }
        
        // Remove trailing slash and path
        if let Some(slash_pos) = normalized.find('/') {
            normalized = normalized[..slash_pos].to_string();
        }
        
        // Remove port
        if let Some(colon_pos) = normalized.find(':') {
            normalized = normalized[..colon_pos].to_string();
        }
        
        normalized
    }
    
    /// Add domain to blocklist
    pub fn block_domain(&mut self, domain: &str) {
        let normalized = self.normalize_domain(domain);
        debug!("Adding domain to blocklist: {}", normalized);
        self.blocked_domains.insert(normalized);
    }
    
    /// Remove domain from blocklist
    pub fn unblock_domain(&mut self, domain: &str) {
        let normalized = self.normalize_domain(domain);
        debug!("Removing domain from blocklist: {}", normalized);
        self.blocked_domains.remove(&normalized);
    }
    
    /// Add domain to allowlist
    pub fn allow_domain(&mut self, domain: &str) {
        let normalized = self.normalize_domain(domain);
        debug!("Adding domain to allowlist: {}", normalized);
        
        if self.allowed_domains.is_none() {
            self.allowed_domains = Some(HashSet::new());
        }
        
        self.allowed_domains.as_mut().unwrap().insert(normalized);
    }
    
    /// Remove domain from allowlist
    pub fn disallow_domain(&mut self, domain: &str) {
        let normalized = self.normalize_domain(domain);
        debug!("Removing domain from allowlist: {}", normalized);
        
        if let Some(allowed) = &mut self.allowed_domains {
            allowed.remove(&normalized);
        }
    }
    
    /// Get all blocked domains
    pub fn get_blocked_domains(&self) -> Vec<String> {
        self.blocked_domains.iter().cloned().collect()
    }
    
    /// Get all allowed domains
    pub fn get_allowed_domains(&self) -> Option<Vec<String>> {
        self.allowed_domains.as_ref().map(|set| set.iter().cloned().collect())
    }
    
    /// Check if domain is in a dangerous category
    pub fn is_dangerous_domain(&self, domain: &str) -> bool {
        let normalized = self.normalize_domain(domain);
        
        // Check for localhost and internal addresses
        if normalized == "localhost" || 
           normalized == "127.0.0.1" || 
           normalized.starts_with("192.168.") ||
           normalized.starts_with("10.") ||
           normalized.starts_with("172.") {
            return true;
        }
        
        // Check for suspicious TLDs
        let suspicious_tlds = [".tk", ".ml", ".ga", ".cf", ".onion"];
        for tld in &suspicious_tlds {
            if normalized.ends_with(tld) {
                return true;
            }
        }
        
        // Check for IP addresses (generally suspicious for scraping)
        if self.is_ip_address(&normalized) {
            return true;
        }
        
        false
    }
    
    /// Check if string is an IP address
    fn is_ip_address(&self, s: &str) -> bool {
        // Simple IPv4 check
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() == 4 {
            return parts.iter().all(|part| {
                part.parse::<u8>().is_ok()
            });
        }
        
        // Simple IPv6 check (basic)
        s.contains(':') && s.chars().all(|c| c.is_ascii_hexdigit() || c == ':')
    }
    
    /// Get domain statistics
    pub fn get_stats(&self) -> DomainStats {
        DomainStats {
            blocked_count: self.blocked_domains.len(),
            allowed_count: self.allowed_domains.as_ref().map(|set| set.len()),
            has_allowlist: self.allowed_domains.is_some(),
        }
    }
}

/// Domain statistics
#[derive(Debug, Clone)]
pub struct DomainStats {
    pub blocked_count: usize,
    pub allowed_count: Option<usize>,
    pub has_allowlist: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_domain_normalization() {
        let whitelist = DomainWhitelist::new(&[]).unwrap();
        
        assert_eq!(whitelist.normalize_domain("https://www.example.com/path"), "example.com");
        assert_eq!(whitelist.normalize_domain("http://example.com:8080"), "example.com");
        assert_eq!(whitelist.normalize_domain("Example.COM"), "example.com");
        assert_eq!(whitelist.normalize_domain("www.subdomain.example.com"), "subdomain.example.com");
    }
    
    #[test]
    fn test_domain_blocking() {
        let mut whitelist = DomainWhitelist::new(&["blocked.com".to_string()]).unwrap();
        
        assert!(whitelist.is_blocked("blocked.com"));
        assert!(whitelist.is_blocked("sub.blocked.com"));
        assert!(!whitelist.is_blocked("notblocked.com"));
        
        whitelist.block_domain("newblock.com");
        assert!(whitelist.is_blocked("newblock.com"));
    }
    
    #[test]
    fn test_domain_allowing() {
        let whitelist = DomainWhitelist::new_with_allowlist(
            &[],
            &["allowed.com".to_string()]
        ).unwrap();
        
        assert!(whitelist.is_allowed("allowed.com"));
        assert!(whitelist.is_allowed("sub.allowed.com"));
        assert!(!whitelist.is_allowed("notallowed.com"));
    }
    
    #[test]
    fn test_dangerous_domains() {
        let whitelist = DomainWhitelist::new(&[]).unwrap();
        
        assert!(whitelist.is_dangerous_domain("localhost"));
        assert!(whitelist.is_dangerous_domain("127.0.0.1"));
        assert!(whitelist.is_dangerous_domain("192.168.1.1"));
        assert!(whitelist.is_dangerous_domain("example.tk"));
        assert!(!whitelist.is_dangerous_domain("example.com"));
    }
    
    #[test]
    fn test_ip_address_detection() {
        let whitelist = DomainWhitelist::new(&[]).unwrap();
        
        assert!(whitelist.is_ip_address("192.168.1.1"));
        assert!(whitelist.is_ip_address("10.0.0.1"));
        assert!(!whitelist.is_ip_address("example.com"));
        assert!(!whitelist.is_ip_address("192.168.1.256")); // Invalid IP
    }
    
    #[test]
    fn test_validation() {
        let whitelist = DomainWhitelist::new(&["blocked.com".to_string()]).unwrap();
        
        assert!(whitelist.validate_domain("allowed.com").is_ok());
        assert!(whitelist.validate_domain("blocked.com").is_err());
        assert!(whitelist.validate_domain("sub.blocked.com").is_err());
    }
}
