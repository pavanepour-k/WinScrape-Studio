#[cfg(feature = "ui")]
use std::process::Command;
#[cfg(feature = "ui")]
use std::path::Path;
#[cfg(feature = "ui")]
use std::env;
#[cfg(feature = "ui")]
use tracing::{info, error, warn};

/// Windows application launcher with proper error handling and user experience
#[cfg(feature = "ui")]
pub struct WindowsLauncher;

#[cfg(feature = "ui")]
impl WindowsLauncher {
    /// Launch the Windows GUI application
    pub fn launch_gui() -> Result<(), Box<dyn std::error::Error>> {
        info!("Launching WinScrape Studio GUI...");
        
        // Check if we're running from the correct directory
        let current_dir = env::current_dir()?;
        info!("Current directory: {:?}", current_dir);
        
        // Check for required files
        let required_files = [
            "winscrape-studio.exe",
            "winscrape-studio.exe.manifest",
        ];
        
        for file in &required_files {
            if !Path::new(file).exists() {
                warn!("Required file not found: {}", file);
            }
        }
        
        // Set up environment variables for Windows
        env::set_var("RUST_LOG", "info");
        env::set_var("RUST_BACKTRACE", "1");
        
        // Launch the application
        let mut cmd = Command::new("winscrape-studio.exe");
        cmd.arg("--gui");
        cmd.current_dir(&current_dir);
        
        info!("Executing command: {:?}", cmd);
        
        let status = cmd.status()?;
        
        if status.success() {
            info!("Application launched successfully");
        } else {
            error!("Application failed to launch with exit code: {:?}", status.code());
        }
        
        Ok(())
    }
    
    /// Check system requirements
    pub fn check_system_requirements() -> Result<(), String> {
        info!("Checking system requirements...");
        
        // Check Windows version
        let os_info = os_info::get();
        if os_info.os_type() != os_info::Type::Windows {
            return Err("WinScrape Studio requires Windows 10 or later".to_string());
        }
        
        let version = os_info.version();
        info!("Windows version: {}", version);
        
        // Check if we have sufficient memory (basic check)
        let memory_info = sysinfo::System::new_all().total_memory();
        let memory_gb = memory_info / (1024 * 1024 * 1024);
        info!("Total system memory: {} GB", memory_gb);
        
        if memory_gb < 4 {
            warn!("System has less than 4GB RAM. Performance may be affected.");
        }
        
        // Check if we have internet connectivity
        if let Err(_) = std::net::TcpStream::connect("8.8.8.8:53") {
            warn!("No internet connectivity detected. Some features may not work.");
        }
        
        Ok(())
    }
    
    /// Show error dialog
    pub fn show_error_dialog(title: &str, message: &str) {
        #[cfg(windows)]
        {
            use winapi::um::winuser::{MessageBoxA, MB_OK, MB_ICONERROR};
            use std::ffi::CString;
            
            let title_c = CString::new(title).unwrap_or_default();
            let message_c = CString::new(message).unwrap_or_default();
            
            unsafe {
                MessageBoxA(
                    std::ptr::null_mut(),
                    message_c.as_ptr(),
                    title_c.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        
        #[cfg(not(windows))]
        {
            eprintln!("Error: {} - {}", title, message);
        }
    }
    
    /// Show info dialog
    pub fn show_info_dialog(title: &str, message: &str) {
        #[cfg(windows)]
        {
            use winapi::um::winuser::{MessageBoxA, MB_OK, MB_ICONINFORMATION};
            use std::ffi::CString;
            
            let title_c = CString::new(title).unwrap_or_default();
            let message_c = CString::new(message).unwrap_or_default();
            
            unsafe {
                MessageBoxA(
                    std::ptr::null_mut(),
                    message_c.as_ptr(),
                    title_c.as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
        }
        
        #[cfg(not(windows))]
        {
            println!("Info: {} - {}", title, message);
        }
    }
    
    /// Check for updates
    pub fn check_for_updates() -> Result<Option<String>, Box<dyn std::error::Error>> {
        info!("Checking for updates...");
        
        // This would typically check a remote server for updates
        // For now, we'll just return None (no updates)
        Ok(None)
    }
    
    /// Install update
    pub fn install_update(update_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Installing update from: {}", update_url);
        
        // This would download and install the update
        // For now, we'll just show a message
        Self::show_info_dialog(
            "Update Available",
            "An update is available. Please download and install it manually from the website.",
        );
        
        Ok(())
    }
    
    /// Create desktop shortcut
    pub fn create_desktop_shortcut() -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        {
            use std::fs;
            use std::path::PathBuf;
            
            let desktop = env::var("USERPROFILE")? + "\\Desktop";
            let shortcut_path = PathBuf::from(desktop).join("WinScrape Studio.lnk");
            
            // Create a simple batch file as a shortcut for now
            let batch_content = format!(
                "@echo off\ncd /d \"{}\"\nstart winscrape-studio.exe --gui\n",
                env::current_dir()?.display()
            );
            
            fs::write(shortcut_path.with_extension("bat"), batch_content)?;
            
            info!("Desktop shortcut created");
        }
        
        Ok(())
    }
    
    /// Create start menu shortcut
    pub fn create_start_menu_shortcut() -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        {
            use std::fs;
            use std::path::PathBuf;
            
            let start_menu = env::var("APPDATA")? + "\\Microsoft\\Windows\\Start Menu\\Programs";
            let shortcut_dir = PathBuf::from(start_menu).join("WinScrape Studio");
            
            fs::create_dir_all(&shortcut_dir)?;
            
            let shortcut_path = shortcut_dir.join("WinScrape Studio.bat");
            
            // Create a simple batch file as a shortcut for now
            let batch_content = format!(
                "@echo off\ncd /d \"{}\"\nstart winscrape-studio.exe --gui\n",
                env::current_dir()?.display()
            );
            
            fs::write(shortcut_path, batch_content)?;
            
            info!("Start menu shortcut created");
        }
        
        Ok(())
    }
    
    /// Register file associations
    pub fn register_file_associations() -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        {
            use winapi::um::winreg::{RegCreateKeyA, RegSetValueExA, HKEY_CLASSES_ROOT};
            use winapi::um::winnt::KEY_SET_VALUE;
            use std::ffi::CString;
            
            let exe_path = env::current_exe()?;
            let exe_path_str = exe_path.to_string_lossy();
            
            // Register .wss file extension
            let key_name = CString::new(".wss")?;
            let value_name = CString::new("")?;
            let value_data = CString::new("WinScrapeStudio.Document")?;
            
            unsafe {
                let mut hkey = std::ptr::null_mut();
                RegCreateKeyA(HKEY_CLASSES_ROOT, key_name.as_ptr(), &mut hkey);
                RegSetValueExA(hkey, value_name.as_ptr(), 0, 1, value_data.as_ptr() as *const _, value_data.as_bytes().len() as u32);
            }
            
            info!("File associations registered");
        }
        
        Ok(())
    }
    
    /// Unregister file associations
    pub fn unregister_file_associations() -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(windows)]
        {
            use winapi::um::winreg::{RegDeleteKeyA, HKEY_CLASSES_ROOT};
            use std::ffi::CString;
            
            unsafe {
                let key_name = CString::new(".wss")?;
                RegDeleteKeyA(HKEY_CLASSES_ROOT, key_name.as_ptr());
                
                let class_name = CString::new("WinScrapeStudio.Document")?;
                RegDeleteKeyA(HKEY_CLASSES_ROOT, class_name.as_ptr());
            }
            
            info!("File associations unregistered");
        }
        
        Ok(())
    }
    
    /// Get application version
    pub fn get_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
    
    /// Get application name
    pub fn get_name() -> String {
        "WinScrape Studio".to_string()
    }
    
    /// Get application description
    pub fn get_description() -> String {
        "A natural language web scraping tool for Windows".to_string()
    }
    
    /// Get application website
    pub fn get_website() -> String {
        "https://github.com/winscrape-studio".to_string()
    }
    
    /// Get application support email
    pub fn get_support_email() -> String {
        "pavanepour@outlook.com".to_string()
    }
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct WindowsLauncher;

#[cfg(not(feature = "ui"))]
impl WindowsLauncher {
    pub fn launch_gui() -> Result<(), Box<dyn std::error::Error>> {
        Err("GUI feature not enabled".into())
    }
    pub fn check_system_requirements() -> Result<(), String> {
        Err("GUI feature not enabled".into())
    }
    pub fn show_error_dialog(_title: &str, _message: &str) {}
    pub fn show_info_dialog(_title: &str, _message: &str) {}
    pub fn check_for_updates() -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(None)
    }
    pub fn install_update(_update_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    pub fn create_desktop_shortcut() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    pub fn create_start_menu_shortcut() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    pub fn register_file_associations() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    pub fn unregister_file_associations() -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    pub fn get_version() -> String { "0.1.0".to_string() }
    pub fn get_name() -> String { "WinScrape Studio".to_string() }
    pub fn get_description() -> String { "A natural language web scraping tool".to_string() }
    pub fn get_website() -> String { "https://github.com/winscrape-studio".to_string() }
    pub fn get_support_email() -> String { "pavanepour@outlook.com".to_string() }
}
