#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use std::path::Path;
#[cfg(feature = "ui")]
use std::fs;
#[cfg(feature = "ui")]
use anyhow::Result;
#[cfg(feature = "ui")]
use tracing::{info, error, warn};
#[cfg(feature = "ui")]
use serde::{Serialize, Deserialize};

/// Icon theme types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconTheme {
    Default,
    Minimal,
    Colorful,
    Monochrome,
    Custom,
}

impl IconTheme {
    pub fn name(&self) -> &'static str {
        match self {
            IconTheme::Default => "Default",
            IconTheme::Minimal => "Minimal",
            IconTheme::Colorful => "Colorful",
            IconTheme::Monochrome => "Monochrome",
            IconTheme::Custom => "Custom",
        }
    }

    pub fn all() -> Vec<IconTheme> {
        vec![
            IconTheme::Default,
            IconTheme::Minimal,
            IconTheme::Colorful,
            IconTheme::Monochrome,
            IconTheme::Custom,
        ]
    }

    pub fn translation_key(&self) -> &'static str {
        match self {
            IconTheme::Default => "settings.icon_theme.default",
            IconTheme::Minimal => "settings.icon_theme.minimal",
            IconTheme::Colorful => "settings.icon_theme.colorful",
            IconTheme::Monochrome => "settings.icon_theme.monochrome",
            IconTheme::Custom => "settings.icon_theme.custom",
        }
    }
}

impl Default for IconTheme {
    fn default() -> Self {
        IconTheme::Default
    }
}

/// Icon manager for handling application icons
#[cfg(feature = "ui")]
pub struct IconManager {
    current_theme: IconTheme,
    custom_icon_path: Option<String>,
    available_icons: Vec<IconInfo>,
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone)]
pub struct IconInfo {
    pub name: String,
    pub path: String,
    pub theme: IconTheme,
    pub size: (u32, u32),
    pub description: String,
}

#[cfg(feature = "ui")]
impl IconManager {
    /// Create new icon manager
    pub fn new() -> Self {
        let mut manager = Self {
            current_theme: IconTheme::default(),
            custom_icon_path: None,
            available_icons: Vec::new(),
        };
        
        manager.load_default_icons();
        manager
    }

    /// Load default icons
    fn load_default_icons(&mut self) {
        // Default spider web icon
        self.available_icons.push(IconInfo {
            name: "Spider Web".to_string(),
            path: "default".to_string(),
            theme: IconTheme::Default,
            size: (32, 32),
            description: "Default spider web icon".to_string(),
        });

        // Minimal icon
        self.available_icons.push(IconInfo {
            name: "Minimal Circle".to_string(),
            path: "minimal".to_string(),
            theme: IconTheme::Minimal,
            size: (32, 32),
            description: "Minimal circular icon".to_string(),
        });

        // Colorful icon
        self.available_icons.push(IconInfo {
            name: "Colorful Globe".to_string(),
            path: "colorful".to_string(),
            theme: IconTheme::Colorful,
            size: (32, 32),
            description: "Colorful globe icon".to_string(),
        });

        // Monochrome icon
        self.available_icons.push(IconInfo {
            name: "Monochrome Square".to_string(),
            path: "monochrome".to_string(),
            theme: IconTheme::Monochrome,
            size: (32, 32),
            description: "Monochrome square icon".to_string(),
        });

        info!("Loaded {} default icons", self.available_icons.len());
    }

    /// Get current icon as egui::IconData
    pub fn get_current_icon(&self) -> Option<egui::IconData> {
        match self.current_theme {
            IconTheme::Default => self.create_spider_web_icon(),
            IconTheme::Minimal => self.create_minimal_icon(),
            IconTheme::Colorful => self.create_colorful_icon(),
            IconTheme::Monochrome => self.create_monochrome_icon(),
            IconTheme::Custom => {
                if let Some(path) = &self.custom_icon_path {
                    self.load_icon_from_file(path)
                } else {
                    self.create_spider_web_icon()
                }
            }
        }
    }

    /// Set icon theme
    pub fn set_theme(&mut self, theme: IconTheme) {
        info!("Setting icon theme to: {}", theme.name());
        self.current_theme = theme;
    }

    /// Get current theme
    pub fn current_theme(&self) -> IconTheme {
        self.current_theme
    }

    /// Set custom icon path
    pub fn set_custom_icon(&mut self, path: String) -> Result<()> {
        if Path::new(&path).exists() {
            self.custom_icon_path = Some(path.clone());
            self.current_theme = IconTheme::Custom;
            info!("Set custom icon path: {}", path);
            Ok(())
        } else {
            error!("Custom icon file not found: {}", path);
            Err(anyhow::anyhow!("Icon file not found: {}", path))
        }
    }

    /// Get available icons
    pub fn available_icons(&self) -> &[IconInfo] {
        &self.available_icons
    }

    /// Load icon from file
    fn load_icon_from_file(&self, path: &str) -> Option<egui::IconData> {
        match fs::read(path) {
            Ok(icon_data) => {
                if let Ok(image) = image::load_from_memory(&icon_data) {
                    let rgba = image.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    Some(egui::IconData {
                        rgba: rgba.into_raw(),
                        width: size[0] as u32,
                        height: size[1] as u32,
                    })
                } else {
                    error!("Failed to load image from file: {}", path);
                    None
                }
            }
            Err(e) => {
                error!("Failed to read icon file {}: {}", path, e);
                None
            }
        }
    }

    /// Create spider web icon (default)
    fn create_spider_web_icon(&self) -> Option<egui::IconData> {
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // Create spider web pattern
                let r = if distance < 2.0 { 255 } else if distance < 8.0 { 200 } else if distance < 15.0 { 150 } else { 100 };
                let g = if distance < 2.0 { 100 } else if distance < 8.0 { 150 } else if distance < 15.0 { 200 } else { 255 };
                let b = if distance < 2.0 { 100 } else if distance < 8.0 { 100 } else if distance < 15.0 { 100 } else { 100 };
                let a = if distance < 16.0 { 255 } else { 0 };
                
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(a);
            }
        }
        
        Some(egui::IconData {
            rgba,
            width: size as u32,
            height: size as u32,
        })
    }

    /// Create minimal icon
    fn create_minimal_icon(&self) -> Option<egui::IconData> {
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // Create minimal circle
                let intensity = if distance < 12.0 { 255 } else { 0 };
                let a = if distance < 16.0 { 255 } else { 0 };
                
                rgba.push(intensity);
                rgba.push(intensity);
                rgba.push(intensity);
                rgba.push(a);
            }
        }
        
        Some(egui::IconData {
            rgba,
            width: size as u32,
            height: size as u32,
        })
    }

    /// Create colorful icon
    fn create_colorful_icon(&self) -> Option<egui::IconData> {
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                let distance = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);
                
                // Create colorful globe pattern
                let r = if distance < 12.0 { 
                    ((angle.cos() + 1.0) * 127.5) as u8 
                } else { 0 };
                let g = if distance < 12.0 { 
                    ((angle.sin() + 1.0) * 127.5) as u8 
                } else { 0 };
                let b = if distance < 12.0 { 
                    ((distance / 12.0) * 255.0) as u8 
                } else { 0 };
                let a = if distance < 16.0 { 255 } else { 0 };
                
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(a);
            }
        }
        
        Some(egui::IconData {
            rgba,
            width: size as u32,
            height: size as u32,
        })
    }

    /// Create monochrome icon
    fn create_monochrome_icon(&self) -> Option<egui::IconData> {
        let size = 32;
        let mut rgba = Vec::with_capacity(size * size * 4);
        
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - size as f32 / 2.0;
                let dy = y as f32 - size as f32 / 2.0;
                
                // Create monochrome square pattern
                let intensity = if dx.abs() < 8.0 && dy.abs() < 8.0 { 255 } else { 0 };
                let a = if dx.abs() < 16.0 && dy.abs() < 16.0 { 255 } else { 0 };
                
                rgba.push(intensity);
                rgba.push(intensity);
                rgba.push(intensity);
                rgba.push(a);
            }
        }
        
        Some(egui::IconData {
            rgba,
            width: size as u32,
            height: size as u32,
        })
    }

    /// Save current icon to file
    pub fn save_current_icon(&self, path: &str) -> Result<()> {
        if let Some(icon_data) = self.get_current_icon() {
            let image = image::RgbaImage::from_raw(
                icon_data.width,
                icon_data.height,
                icon_data.rgba,
            ).ok_or_else(|| anyhow::anyhow!("Failed to create image from icon data"))?;
            
            image.save(path)?;
            info!("Saved current icon to: {}", path);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No current icon to save"))
        }
    }

    /// Get icon preview as base64 string
    pub fn get_icon_preview(&self, theme: IconTheme) -> Option<String> {
        let icon_data = match theme {
            IconTheme::Default => self.create_spider_web_icon()?,
            IconTheme::Minimal => self.create_minimal_icon()?,
            IconTheme::Colorful => self.create_colorful_icon()?,
            IconTheme::Monochrome => self.create_monochrome_icon()?,
            IconTheme::Custom => {
                if let Some(path) = &self.custom_icon_path {
                    self.load_icon_from_file(path)?
                } else {
                    return None;
                }
            }
        };

        // Convert to base64 for preview
        let image = image::RgbaImage::from_raw(
            icon_data.width,
            icon_data.height,
            icon_data.rgba,
        )?;
        
        let mut buffer = Vec::new();
        if image.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageOutputFormat::Png).is_ok() {
            use base64::{Engine as _, engine::general_purpose};
            Some(general_purpose::STANDARD.encode(buffer))
        } else {
            None
        }
    }

    /// Reset to default icon
    pub fn reset_to_default(&mut self) {
        self.current_theme = IconTheme::Default;
        self.custom_icon_path = None;
        info!("Reset icon to default");
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct IconManager;

#[cfg(not(feature = "ui"))]
impl IconManager {
    pub fn new() -> Self { Self }
    pub fn set_theme(&mut self, _theme: IconTheme) {}
    pub fn current_theme(&self) -> IconTheme { IconTheme::Default }
    pub fn set_custom_icon(&mut self, _path: String) -> Result<()> { Ok(()) }
    pub fn available_icons(&self) -> &[IconInfo] { &[] }
    pub fn save_current_icon(&self, _path: &str) -> Result<()> { Ok(()) }
    pub fn get_icon_preview(&self, _theme: IconTheme) -> Option<String> { None }
    pub fn reset_to_default(&mut self) {}
}

#[cfg(not(feature = "ui"))]
#[derive(Debug, Clone)]
pub struct IconInfo {
    pub name: String,
    pub path: String,
    pub theme: IconTheme,
    pub size: (u32, u32),
    pub description: String,
}

#[cfg(not(feature = "ui"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconTheme {
    Default,
    Minimal,
    Colorful,
    Monochrome,
    Custom,
}

#[cfg(not(feature = "ui"))]
impl IconTheme {
    pub fn name(&self) -> &'static str { "Default" }
    pub fn all() -> Vec<IconTheme> { vec![IconTheme::Default] }
}

#[cfg(not(feature = "ui"))]
impl Default for IconTheme {
    fn default() -> Self { IconTheme::Default }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_theme() {
        assert_eq!(IconTheme::Default.name(), "Default");
        assert_eq!(IconTheme::Minimal.name(), "Minimal");
        assert_eq!(IconTheme::Colorful.name(), "Colorful");
    }

    #[test]
    fn test_icon_manager_creation() {
        let manager = IconManager::new();
        assert_eq!(manager.current_theme(), IconTheme::Default);
        assert!(!manager.available_icons().is_empty());
    }
}
