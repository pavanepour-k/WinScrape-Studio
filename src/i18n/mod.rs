use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    English,
    Korean,
    Japanese,
    Chinese,
    Spanish,
    French,
    German,
    Russian,
}

impl Language {
    /// Get language code (ISO 639-1)
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Korean => "ko",
            Language::Japanese => "ja",
            Language::Chinese => "zh",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Russian => "ru",
        }
    }

    /// Get language name in English
    pub fn name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Korean => "한국어",
            Language::Japanese => "日本語",
            Language::Chinese => "中文",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Russian => "Русский",
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![
            Language::English,
            Language::Korean,
            Language::Japanese,
            Language::Chinese,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Russian,
        ]
    }

    /// Parse language from code
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" => Some(Language::English),
            "ko" => Some(Language::Korean),
            "ja" => Some(Language::Japanese),
            "zh" => Some(Language::Chinese),
            "es" => Some(Language::Spanish),
            "fr" => Some(Language::French),
            "de" => Some(Language::German),
            "ru" => Some(Language::Russian),
            _ => None,
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

/// Translation key type
pub type TranslationKey = &'static str;

/// Translation value type
pub type TranslationValue = String;

/// Translation map for a single language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageTranslations {
    pub language: Language,
    pub translations: HashMap<String, TranslationValue>,
}

impl LanguageTranslations {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            translations: HashMap::new(),
        }
    }

    pub fn add_translation(&mut self, key: &str, value: String) {
        self.translations.insert(key.to_string(), value);
    }

    pub fn get_translation(&self, key: &str) -> Option<&String> {
        self.translations.get(key)
    }
}

/// Internationalization manager
#[derive(Debug, Clone)]
pub struct I18nManager {
    current_language: Language,
    translations: HashMap<Language, LanguageTranslations>,
    fallback_language: Language,
}

impl I18nManager {
    /// Create new I18n manager
    pub fn new() -> Self {
        let mut manager = Self {
            current_language: Language::default(),
            translations: HashMap::new(),
            fallback_language: Language::English,
        };

        // Load default translations
        manager.load_default_translations();
        manager
    }

    /// Set current language
    pub fn set_language(&mut self, language: Language) {
        info!("Setting language to: {}", language.name());
        self.current_language = language;
    }

    /// Get current language
    pub fn current_language(&self) -> Language {
        self.current_language
    }

    /// Get translation for current language
    pub fn t(&self, key: &str) -> String {
        self.get_translation(key, self.current_language)
    }

    /// Get translation for specific language
    pub fn get_translation(&self, key: &str, language: Language) -> String {
        // Try current language first
        if let Some(translations) = self.translations.get(&language) {
            if let Some(translation) = translations.get_translation(key) {
                return translation.clone();
            }
        }

        // Fallback to English
        if language != self.fallback_language {
            if let Some(translations) = self.translations.get(&self.fallback_language) {
                if let Some(translation) = translations.get_translation(key) {
                    warn!("Translation missing for key '{}' in language '{}', using fallback", key, language.name());
                    return translation.clone();
                }
            }
        }

        // Return key as fallback
        warn!("Translation missing for key '{}' in all languages, using key as fallback", key);
        key.to_string()
    }

    /// Load translations from file
    pub fn load_translations_from_file(&mut self, language: Language, file_path: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;
        let translations: HashMap<String, String> = serde_json::from_str(&content)?;
        let translation_count = translations.len();
        
        let mut lang_translations = LanguageTranslations::new(language);
        for (key, value) in translations {
            lang_translations.add_translation(&key, value);
        }
        
        self.translations.insert(language, lang_translations);
        info!("Loaded {} translations for language: {}", translation_count, language.name());
        
        Ok(())
    }

    /// Save translations to file
    pub fn save_translations_to_file(&self, language: Language, file_path: &str) -> Result<()> {
        if let Some(translations) = self.translations.get(&language) {
            let json = serde_json::to_string_pretty(&translations.translations)?;
            std::fs::write(file_path, json)?;
            info!("Saved translations for language: {}", language.name());
        }
        Ok(())
    }

    /// Load default translations
    fn load_default_translations(&mut self) {
        // English translations
        let mut english = LanguageTranslations::new(Language::English);
        english.add_translation("app.title", "WinScrape Studio".to_string());
        english.add_translation("app.version", "Version".to_string());
        english.add_translation("app.description", "A natural language web scraping tool".to_string());
        english.add_translation("app.website", "Website".to_string());
        english.add_translation("app.support", "Support".to_string());
        
        // Navigation
        english.add_translation("nav.chat", "Chat".to_string());
        english.add_translation("nav.jobs", "Jobs".to_string());
        english.add_translation("nav.results", "Results".to_string());
        english.add_translation("nav.settings", "Settings".to_string());
        english.add_translation("nav.help", "Help".to_string());
        
        // Chat interface
        english.add_translation("chat.title", "Natural Language Scraping".to_string());
        english.add_translation("chat.description", "Describe what you want to scrape in plain English. The AI will generate a scraping plan for you.".to_string());
        english.add_translation("chat.input_placeholder", "Describe what you want to scrape...".to_string());
        english.add_translation("chat.send", "Send".to_string());
        english.add_translation("chat.examples", "Examples".to_string());
        english.add_translation("chat.try_examples", "Try these examples:".to_string());
        
        // Jobs
        english.add_translation("jobs.title", "Scraping Jobs".to_string());
        english.add_translation("jobs.no_jobs", "No Jobs Yet".to_string());
        english.add_translation("jobs.no_jobs_description", "Start by describing what you want to scrape in the Chat tab.".to_string());
        english.add_translation("jobs.go_to_chat", "Go to Chat".to_string());
        english.add_translation("jobs.status.running", "Running".to_string());
        english.add_translation("jobs.status.completed", "Completed".to_string());
        english.add_translation("jobs.status.failed", "Failed".to_string());
        english.add_translation("jobs.status.queued", "Queued".to_string());
        english.add_translation("jobs.status.cancelled", "Cancelled".to_string());
        
        // Settings
        english.add_translation("settings.title", "Settings & Configuration".to_string());
        english.add_translation("settings.general", "General Settings".to_string());
        english.add_translation("settings.theme", "Theme".to_string());
        english.add_translation("settings.theme.dark", "Dark".to_string());
        english.add_translation("settings.theme.light", "Light".to_string());
        english.add_translation("settings.language", "Language".to_string());
        english.add_translation("settings.auto_save", "Auto-save settings".to_string());
        english.add_translation("settings.notifications", "Show notifications".to_string());
        english.add_translation("settings.minimize_to_tray", "Minimize to system tray".to_string());
        english.add_translation("settings.icon_theme", "Icon Theme".to_string());
        english.add_translation("settings.icon_theme.default", "Default".to_string());
        english.add_translation("settings.icon_theme.minimal", "Minimal".to_string());
        english.add_translation("settings.icon_theme.colorful", "Colorful".to_string());
        english.add_translation("settings.icon_theme.monochrome", "Monochrome".to_string());
        english.add_translation("settings.icon_theme.custom", "Custom".to_string());
        
        // Scraping settings
        english.add_translation("settings.scraping", "Scraping Settings".to_string());
        english.add_translation("settings.max_concurrent", "Max concurrent requests".to_string());
        english.add_translation("settings.timeout", "Request timeout (seconds)".to_string());
        english.add_translation("settings.respect_robots", "Respect robots.txt".to_string());
        english.add_translation("settings.browser_fallback", "Enable browser fallback".to_string());
        
        // Export settings
        english.add_translation("settings.export", "Export Settings".to_string());
        
        // Common UI elements
        english.add_translation("button.save", "Save".to_string());
        english.add_translation("button.cancel", "Cancel".to_string());
        english.add_translation("button.reset", "Reset".to_string());
        english.add_translation("button.apply", "Apply".to_string());
        english.add_translation("button.ok", "OK".to_string());
        english.add_translation("button.yes", "Yes".to_string());
        english.add_translation("button.no", "No".to_string());
        english.add_translation("status.ready", "Ready".to_string());
        english.add_translation("status.running", "Running".to_string());
        english.add_translation("status.completed", "Completed".to_string());
        english.add_translation("status.error", "Error".to_string());
        english.add_translation("status.paused", "Paused".to_string());
        english.add_translation("settings.default_format", "Default export format".to_string());
        english.add_translation("settings.include_metadata", "Include metadata in exports".to_string());
        english.add_translation("settings.compress_exports", "Compress large exports".to_string());
        
        // Security settings
        english.add_translation("settings.security", "Security Settings".to_string());
        english.add_translation("settings.input_validation", "Enable input validation".to_string());
        english.add_translation("settings.output_filtering", "Filter sensitive data from output".to_string());
        english.add_translation("settings.blocked_domains", "Blocked domains:".to_string());
        
        // Help
        english.add_translation("help.title", "Help & Documentation".to_string());
        english.add_translation("help.getting_started", "Getting Started".to_string());
        english.add_translation("help.step1", "1. Go to the Chat tab".to_string());
        english.add_translation("help.step2", "2. Describe what you want to scrape in plain English".to_string());
        english.add_translation("help.step3", "3. Review the generated scraping plan".to_string());
        english.add_translation("help.step4", "4. Approve and run the scraping job".to_string());
        english.add_translation("help.step5", "5. Export your results".to_string());
        english.add_translation("help.examples", "Example Requests".to_string());
        english.add_translation("help.features", "Features".to_string());
        english.add_translation("help.about", "About".to_string());
        
        // Common actions
        english.add_translation("action.save", "Save".to_string());
        english.add_translation("action.cancel", "Cancel".to_string());
        english.add_translation("action.close", "Close".to_string());
        english.add_translation("action.ok", "OK".to_string());
        english.add_translation("action.yes", "Yes".to_string());
        english.add_translation("action.no", "No".to_string());
        english.add_translation("action.export", "Export".to_string());
        english.add_translation("action.import", "Import".to_string());
        english.add_translation("action.refresh", "Refresh".to_string());
        english.add_translation("action.delete", "Delete".to_string());
        english.add_translation("action.edit", "Edit".to_string());
        english.add_translation("action.view", "View".to_string());
        
        // Notifications
        english.add_translation("notification.success", "Success".to_string());
        english.add_translation("notification.error", "Error".to_string());
        english.add_translation("notification.warning", "Warning".to_string());
        english.add_translation("notification.info", "Information".to_string());
        
        self.translations.insert(Language::English, english);

        // Korean translations
        let mut korean = LanguageTranslations::new(Language::Korean);
        korean.add_translation("app.title", "WinScrape Studio".to_string());
        korean.add_translation("app.version", "버전".to_string());
        korean.add_translation("app.description", "자연어 웹 스크래핑 도구".to_string());
        korean.add_translation("app.website", "웹사이트".to_string());
        korean.add_translation("app.support", "지원".to_string());
        
        // Navigation
        korean.add_translation("nav.chat", "채팅".to_string());
        korean.add_translation("nav.jobs", "작업".to_string());
        korean.add_translation("nav.results", "결과".to_string());
        korean.add_translation("nav.settings", "설정".to_string());
        korean.add_translation("nav.help", "도움말".to_string());
        
        // Chat interface
        korean.add_translation("chat.title", "자연어 스크래핑".to_string());
        korean.add_translation("chat.description", "스크래핑하고 싶은 내용을 평범한 한국어로 설명하세요. AI가 스크래핑 계획을 생성해드립니다.".to_string());
        korean.add_translation("chat.input_placeholder", "스크래핑하고 싶은 내용을 설명하세요...".to_string());
        korean.add_translation("chat.send", "전송".to_string());
        korean.add_translation("chat.examples", "예시".to_string());
        korean.add_translation("chat.try_examples", "다음 예시를 시도해보세요:".to_string());
        
        // Jobs
        korean.add_translation("jobs.title", "스크래핑 작업".to_string());
        korean.add_translation("jobs.no_jobs", "작업이 없습니다".to_string());
        korean.add_translation("jobs.no_jobs_description", "채팅 탭에서 스크래핑하고 싶은 내용을 설명하여 시작하세요.".to_string());
        korean.add_translation("jobs.go_to_chat", "채팅으로 이동".to_string());
        korean.add_translation("jobs.status.running", "실행 중".to_string());
        korean.add_translation("jobs.status.completed", "완료됨".to_string());
        korean.add_translation("jobs.status.failed", "실패함".to_string());
        korean.add_translation("jobs.status.queued", "대기 중".to_string());
        korean.add_translation("jobs.status.cancelled", "취소됨".to_string());
        
        // Settings
        korean.add_translation("settings.title", "설정 및 구성".to_string());
        korean.add_translation("settings.general", "일반 설정".to_string());
        korean.add_translation("settings.theme", "테마".to_string());
        korean.add_translation("settings.theme.dark", "다크".to_string());
        korean.add_translation("settings.theme.light", "라이트".to_string());
        korean.add_translation("settings.language", "언어".to_string());
        korean.add_translation("settings.auto_save", "설정 자동 저장".to_string());
        korean.add_translation("settings.notifications", "알림 표시".to_string());
        korean.add_translation("settings.minimize_to_tray", "시스템 트레이로 최소화".to_string());
        korean.add_translation("settings.icon_theme", "아이콘 테마".to_string());
        korean.add_translation("settings.icon_theme.default", "기본값".to_string());
        korean.add_translation("settings.icon_theme.minimal", "미니멀".to_string());
        korean.add_translation("settings.icon_theme.colorful", "컬러풀".to_string());
        korean.add_translation("settings.icon_theme.monochrome", "모노크롬".to_string());
        korean.add_translation("settings.icon_theme.custom", "사용자 정의".to_string());
        
        // Scraping settings
        korean.add_translation("settings.scraping", "스크래핑 설정".to_string());
        korean.add_translation("settings.max_concurrent", "최대 동시 요청 수".to_string());
        korean.add_translation("settings.timeout", "요청 시간 제한 (초)".to_string());
        korean.add_translation("settings.respect_robots", "robots.txt 준수".to_string());
        korean.add_translation("settings.browser_fallback", "브라우저 폴백 활성화".to_string());
        
        // Export settings
        korean.add_translation("settings.export", "내보내기 설정".to_string());
        korean.add_translation("settings.default_format", "기본 내보내기 형식".to_string());
        korean.add_translation("settings.include_metadata", "내보내기에 메타데이터 포함".to_string());
        korean.add_translation("settings.compress_exports", "대용량 내보내기 압축".to_string());
        
        // Common UI elements
        korean.add_translation("button.save", "저장".to_string());
        korean.add_translation("button.cancel", "취소".to_string());
        korean.add_translation("button.reset", "재설정".to_string());
        korean.add_translation("button.apply", "적용".to_string());
        korean.add_translation("button.ok", "확인".to_string());
        korean.add_translation("button.yes", "예".to_string());
        korean.add_translation("button.no", "아니오".to_string());
        korean.add_translation("status.ready", "준비됨".to_string());
        korean.add_translation("status.running", "실행 중".to_string());
        korean.add_translation("status.completed", "완료됨".to_string());
        korean.add_translation("status.error", "오류".to_string());
        korean.add_translation("status.paused", "일시정지됨".to_string());
        
        // Security settings
        korean.add_translation("settings.security", "보안 설정".to_string());
        korean.add_translation("settings.input_validation", "입력 검증 활성화".to_string());
        korean.add_translation("settings.output_filtering", "출력에서 민감한 데이터 필터링".to_string());
        korean.add_translation("settings.blocked_domains", "차단된 도메인:".to_string());
        
        // Help
        korean.add_translation("help.title", "도움말 및 문서".to_string());
        korean.add_translation("help.getting_started", "시작하기".to_string());
        korean.add_translation("help.step1", "1. 채팅 탭으로 이동".to_string());
        korean.add_translation("help.step2", "2. 스크래핑하고 싶은 내용을 평범한 한국어로 설명".to_string());
        korean.add_translation("help.step3", "3. 생성된 스크래핑 계획 검토".to_string());
        korean.add_translation("help.step4", "4. 스크래핑 작업 승인 및 실행".to_string());
        korean.add_translation("help.step5", "5. 결과 내보내기".to_string());
        korean.add_translation("help.examples", "요청 예시".to_string());
        korean.add_translation("help.features", "기능".to_string());
        korean.add_translation("help.about", "정보".to_string());
        
        // Common actions
        korean.add_translation("action.save", "저장".to_string());
        korean.add_translation("action.cancel", "취소".to_string());
        korean.add_translation("action.close", "닫기".to_string());
        korean.add_translation("action.ok", "확인".to_string());
        korean.add_translation("action.yes", "예".to_string());
        korean.add_translation("action.no", "아니오".to_string());
        korean.add_translation("action.export", "내보내기".to_string());
        korean.add_translation("action.import", "가져오기".to_string());
        korean.add_translation("action.refresh", "새로고침".to_string());
        korean.add_translation("action.delete", "삭제".to_string());
        korean.add_translation("action.edit", "편집".to_string());
        korean.add_translation("action.view", "보기".to_string());
        
        // Notifications
        korean.add_translation("notification.success", "성공".to_string());
        korean.add_translation("notification.error", "오류".to_string());
        korean.add_translation("notification.warning", "경고".to_string());
        korean.add_translation("notification.info", "정보".to_string());
        
        self.translations.insert(Language::Korean, korean);

        info!("Loaded default translations for {} languages", self.translations.len());
    }

    /// Get all available languages
    pub fn available_languages(&self) -> Vec<Language> {
        self.translations.keys().cloned().collect()
    }

    /// Check if language is available
    pub fn is_language_available(&self, language: Language) -> bool {
        self.translations.contains_key(&language)
    }
}

impl Default for I18nManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global I18n manager instance
pub type GlobalI18nManager = Arc<std::sync::RwLock<I18nManager>>;

/// Create global I18n manager
pub fn create_global_i18n_manager() -> GlobalI18nManager {
    Arc::new(std::sync::RwLock::new(I18nManager::new()))
}

/// Get translation helper function
pub fn t(key: TranslationKey) -> String {
    // This would be connected to the global I18n manager in a real implementation
    // For now, return the key as fallback
    key.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Korean.code(), "ko");
        assert_eq!(Language::Japanese.code(), "ja");
    }

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("ko"), Some(Language::Korean));
        assert_eq!(Language::from_code("invalid"), None);
    }

    #[test]
    fn test_i18n_manager() {
        let mut manager = I18nManager::new();
        
        // Test default language
        assert_eq!(manager.current_language(), Language::English);
        
        // Test translation
        let translation = manager.t("app.title");
        assert_eq!(translation, "WinScrape Studio");
        
        // Test language change
        manager.set_language(Language::Korean);
        assert_eq!(manager.current_language(), Language::Korean);
        
        // Test Korean translation
        let korean_translation = manager.t("app.title");
        assert_eq!(korean_translation, "WinScrape Studio");
    }
}
