use rand::seq::SliceRandom;
use std::sync::Arc;

/// User agent rotator for avoiding detection
pub struct UserAgentRotator {
    user_agents: Arc<Vec<String>>,
}

impl UserAgentRotator {
    pub fn new(user_agents: &[String]) -> Self {
        let agents = if user_agents.is_empty() {
            Self::default_user_agents()
        } else {
            user_agents.to_vec()
        };
        
        Self {
            user_agents: Arc::new(agents),
        }
    }
    
    /// Get a random user agent
    pub fn get_random_user_agent(&self) -> &str {
        let mut rng = rand::thread_rng();
        self.user_agents.choose(&mut rng).unwrap()
    }
    
    /// Get user agent by index (for deterministic selection)
    pub fn get_user_agent_by_index(&self, index: usize) -> &str {
        &self.user_agents[index % self.user_agents.len()]
    }
    
    /// Get all available user agents
    pub fn get_all_user_agents(&self) -> &[String] {
        &self.user_agents
    }
    
    /// Get count of available user agents
    pub fn count(&self) -> usize {
        self.user_agents.len()
    }
    
    /// Default user agents covering major browsers and platforms
    fn default_user_agents() -> Vec<String> {
        vec![
            // Chrome on Windows
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36".to_string(),
            
            // Firefox on Windows
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:119.0) Gecko/20100101 Firefox/119.0".to_string(),
            
            // Edge on Windows
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Edge/119.0.0.0 Safari/537.36".to_string(),
            
            // Chrome on macOS
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            
            // Safari on macOS
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15".to_string(),
            
            // Firefox on macOS
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0".to_string(),
            
            // Chrome on Linux
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            
            // Firefox on Linux
            "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0".to_string(),
            
            // Mobile Chrome (Android)
            "Mozilla/5.0 (Linux; Android 10; SM-G973F) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36".to_string(),
            "Mozilla/5.0 (Linux; Android 11; SM-G991B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Mobile Safari/537.36".to_string(),
            
            // Mobile Safari (iOS)
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1".to_string(),
            "Mozilla/5.0 (iPad; CPU OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1".to_string(),
        ]
    }
    
    /// Get user agents for specific browser
    pub fn get_browser_user_agents(&self, browser: Browser) -> Vec<&String> {
        self.user_agents.iter()
            .filter(|ua| match browser {
                Browser::Chrome => ua.contains("Chrome") && !ua.contains("Edge"),
                Browser::Firefox => ua.contains("Firefox"),
                Browser::Safari => ua.contains("Safari") && !ua.contains("Chrome"),
                Browser::Edge => ua.contains("Edge"),
            })
            .collect()
    }
    
    /// Get user agents for specific platform
    pub fn get_platform_user_agents(&self, platform: Platform) -> Vec<&String> {
        self.user_agents.iter()
            .filter(|ua| match platform {
                Platform::Windows => ua.contains("Windows NT"),
                Platform::MacOS => ua.contains("Macintosh"),
                Platform::Linux => ua.contains("X11; Linux"),
                Platform::Android => ua.contains("Android"),
                Platform::IOs => ua.contains("iPhone") || ua.contains("iPad"),
            })
            .collect()
    }
    
    /// Get random user agent for specific browser and platform
    pub fn get_random_user_agent_for(&self, browser: Option<Browser>, platform: Option<Platform>) -> &str {
        let mut candidates = self.user_agents.iter().collect::<Vec<_>>();
        
        if let Some(browser) = browser {
            candidates.retain(|ua| match browser {
                Browser::Chrome => ua.contains("Chrome") && !ua.contains("Edge"),
                Browser::Firefox => ua.contains("Firefox"),
                Browser::Safari => ua.contains("Safari") && !ua.contains("Chrome"),
                Browser::Edge => ua.contains("Edge"),
            });
        }
        
        if let Some(platform) = platform {
            candidates.retain(|ua| match platform {
                Platform::Windows => ua.contains("Windows NT"),
                Platform::MacOS => ua.contains("Macintosh"),
                Platform::Linux => ua.contains("X11; Linux"),
                Platform::Android => ua.contains("Android"),
                Platform::IOs => ua.contains("iPhone") || ua.contains("iPad"),
            });
        }
        
        if candidates.is_empty() {
            return self.get_random_user_agent();
        }
        
        let mut rng = rand::thread_rng();
        candidates.choose(&mut rng).unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Browser {
    Chrome,
    Firefox,
    Safari,
    Edge,
}

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Android,
    IOs,
}

impl Default for UserAgentRotator {
    fn default() -> Self {
        Self::new(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_agent_rotation() {
        let rotator = UserAgentRotator::default();
        
        // Test that we get different user agents
        let ua1 = rotator.get_random_user_agent();
        let ua2 = rotator.get_random_user_agent();
        
        assert!(!ua1.is_empty());
        assert!(!ua2.is_empty());
        
        // Test index-based selection
        let ua_by_index = rotator.get_user_agent_by_index(0);
        assert!(!ua_by_index.is_empty());
        
        // Test count
        assert!(rotator.count() > 0);
    }
    
    #[test]
    fn test_browser_filtering() {
        let rotator = UserAgentRotator::default();
        
        let chrome_agents = rotator.get_browser_user_agents(Browser::Chrome);
        assert!(!chrome_agents.is_empty());
        
        for agent in chrome_agents {
            assert!(agent.contains("Chrome"));
            assert!(!agent.contains("Edge")); // Edge also contains Chrome
        }
        
        let firefox_agents = rotator.get_browser_user_agents(Browser::Firefox);
        assert!(!firefox_agents.is_empty());
        
        for agent in firefox_agents {
            assert!(agent.contains("Firefox"));
        }
    }
    
    #[test]
    fn test_platform_filtering() {
        let rotator = UserAgentRotator::default();
        
        let windows_agents = rotator.get_platform_user_agents(Platform::Windows);
        assert!(!windows_agents.is_empty());
        
        for agent in windows_agents {
            assert!(agent.contains("Windows NT"));
        }
        
        let macos_agents = rotator.get_platform_user_agents(Platform::MacOS);
        assert!(!macos_agents.is_empty());
        
        for agent in macos_agents {
            assert!(agent.contains("Macintosh"));
        }
    }
    
    #[test]
    fn test_combined_filtering() {
        let rotator = UserAgentRotator::default();
        
        let chrome_windows = rotator.get_random_user_agent_for(Some(Browser::Chrome), Some(Platform::Windows));
        assert!(chrome_windows.contains("Chrome"));
        assert!(chrome_windows.contains("Windows NT"));
        assert!(!chrome_windows.contains("Edge"));
    }
}
