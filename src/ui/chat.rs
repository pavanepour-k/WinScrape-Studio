#[cfg(feature = "ui")]
use eframe::egui;
#[cfg(feature = "ui")]
use serde::{Deserialize, Serialize};

/// Chat interface for natural language interaction
#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInterface {
    pub messages: Vec<ChatMessage>,
    pub input_text: String,
    #[serde(skip)]
    pub pending_input: Option<String>,
}

#[cfg(feature = "ui")]
impl ChatInterface {
    pub fn new() -> Self {
        let mut chat = Self {
            messages: Vec::new(),
            input_text: String::new(),
            pending_input: None,
        };
        
        // Add welcome message
        chat.add_system_message("Welcome to WinScrape Studio! Describe what you want to scrape in plain English.".to_string());
        
        chat
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        // Chat history
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for message in &self.messages {
                    self.render_message(ui, message);
                }
            });
        
        ui.separator();
        
        // Input area
        ui.horizontal(|ui| {
            let input_response = ui.add_sized(
                [ui.available_width() - 60.0, 25.0],
                egui::TextEdit::singleline(&mut self.input_text)
                    .hint_text("Describe what you want to scrape...")
            );
            
            let send_button = ui.button("Send");
            
            if (input_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) 
                || send_button.clicked() {
                if !self.input_text.trim().is_empty() {
                    self.pending_input = Some(self.input_text.clone());
                    self.input_text.clear();
                }
            }
        });
    }
    
    fn render_message(&self, ui: &mut egui::Ui, message: &ChatMessage) {
        let (bg_color, text_color, icon) = match message.sender {
            MessageSender::User => (
                egui::Color32::from_rgb(45, 55, 72),
                egui::Color32::WHITE,
                "👤"
            ),
            MessageSender::System => (
                egui::Color32::from_rgb(72, 45, 55),
                egui::Color32::WHITE,
                "🤖"
            ),
            MessageSender::Assistant => (
                egui::Color32::from_rgb(55, 72, 45),
                egui::Color32::WHITE,
                "🕷️"
            ),
        };
        
        ui.group(|ui| {
            ui.visuals_mut().override_text_color = Some(text_color);
            ui.visuals_mut().widgets.noninteractive.bg_fill = bg_color;
            
            ui.horizontal(|ui| {
                ui.label(icon);
                ui.vertical(|ui| {
                    ui.label(&message.content);
                    ui.label(
                        egui::RichText::new(message.timestamp.format("%H:%M:%S").to_string())
                            .size(10.0)
                            .color(egui::Color32::GRAY)
                    );
                });
            });
        });
    }
    
    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            sender: MessageSender::User,
            content,
            timestamp: chrono::Utc::now(),
        });
    }
    
    pub fn add_system_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            sender: MessageSender::System,
            content,
            timestamp: chrono::Utc::now(),
        });
    }
    
    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            sender: MessageSender::Assistant,
            content,
            timestamp: chrono::Utc::now(),
        });
    }
    
    pub fn get_pending_input(&mut self) -> Option<String> {
        self.pending_input.take()
    }
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: MessageSender,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSender {
    User,
    System,
    Assistant,
}

// Stub implementation when UI feature is disabled
#[cfg(not(feature = "ui"))]
pub struct ChatInterface;

#[cfg(not(feature = "ui"))]
impl ChatInterface {
    pub fn new() -> Self {
        Self
    }
}
