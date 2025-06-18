use anyhow::Result;
use chrono;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::Duration;
use tokio::time;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use tui_textarea::TextArea;

use crate::client::ConnectionStatus;
use crate::components::{Button, ButtonManager, ButtonColor};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectDialogStep {
    ServerUrl,
    EndpointSelection,
    Authentication,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityPolicy {
    None,
    Basic128Rsa15,
    Basic256,
    Basic256Sha256,
    Aes128Sha256RsaOaep,    Aes256Sha256RsaPss,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityMode {
    None,
    Sign,
    SignAndEncrypt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthenticationType {
    Anonymous,
    UserPassword,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthenticationField {
    Username,
    Password,
}

#[derive(Debug, Clone)]
pub struct EndpointInfo {
    pub endpoint_url: String,
    pub security_policy: SecurityPolicy,
    pub security_mode: SecurityMode,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct ConnectScreen {
    // Connection dialog state
    pub step: ConnectDialogStep,
    pub server_url_input: Input,
    pub discovered_endpoints: Vec<EndpointInfo>,
    pub selected_endpoint_index: usize,
    pub authentication_type: AuthenticationType,
    pub active_auth_field: AuthenticationField,
    pub username_input: Input,
    pub password_input: Input,
    pub connect_in_progress: bool,
    
    // Input handling
    pub input_mode: InputMode,
    
    // Button management
    pub button_manager: ButtonManager,
      // Logging
    pub connection_log_textarea: TextArea<'static>,
}

impl ConnectScreen {
    pub fn new() -> Self {        // Initialize connection log textarea
        let mut log_textarea = TextArea::default();
        log_textarea.set_style(Style::default().fg(Color::Rgb(135, 135, 175))); // Light blue-grey
        log_textarea.set_block(
            Block::default()
                .title("Connection Log")
                .borders(Borders::ALL)
        );
        
        // Set to read-only mode
        log_textarea.set_cursor_line_style(Style::default());
        log_textarea.set_line_number_style(Style::default());
        
        let mut screen = Self {
            step: ConnectDialogStep::ServerUrl,
            server_url_input: Input::default().with_value("opc.tcp://localhost:4840".to_string()),
            discovered_endpoints: Vec::new(),
            selected_endpoint_index: 0,
            authentication_type: AuthenticationType::Anonymous,
            active_auth_field: AuthenticationField::Username,
            username_input: Input::default(),
            password_input: Input::default(),
            connect_in_progress: false,
            input_mode: InputMode::Editing,
            button_manager: ButtonManager::new(),
            connection_log_textarea: log_textarea,
        };        // Add initial log messages using the proper method
        screen.add_connection_log("OPC UA Client initialized");
        screen.add_connection_log("Loading configuration from config.json");
        screen.add_connection_log("Default server URL loaded");
        screen.add_connection_log("Enter server URL and press Enter to discover endpoints");
        screen.add_connection_log("Use PageUp/PageDown to scroll this log");
        screen.add_connection_log("Alt+O to Connect, Alt+C to Cancel");
        screen.add_connection_log("Connection log initialized");
        screen.add_connection_log("Button manager created with hotkeys");
        screen.add_connection_log("Input handlers configured");
        screen.add_connection_log("Connect screen ready");
        
        screen.setup_buttons_for_current_step();
        screen
    }pub async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {
        // Handle button input first
        if let Some(button_id) = self.button_manager.handle_key_input(key, modifiers) {
            return self.handle_button_action(&button_id).await;
        }

        match self.step {
            ConnectDialogStep::ServerUrl => {
                match key {
                    KeyCode::Enter | KeyCode::Tab => {
                        // Discover endpoints
                        self.discover_endpoints().await?;
                        Ok(None)
                    }                    KeyCode::Esc => {
                        Ok(Some(ConnectionStatus::Disconnected))
                    }
                    KeyCode::PageUp => {
                        // Scroll connection log up
                        for _ in 0..5 {
                            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Up);
                        }
                        Ok(None)
                    }
                    KeyCode::PageDown => {
                        // Scroll connection log down
                        for _ in 0..5 {
                            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Down);
                        }
                        Ok(None)
                    }
                    _ => {
                        // Let tui-input handle the key event
                        if self.input_mode == InputMode::Editing {
                            self.server_url_input.handle_event(&crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new(key, modifiers)
                            ));
                        }
                        Ok(None)
                    }
                }
            }
            ConnectDialogStep::EndpointSelection => {
                match key {
                    KeyCode::Up => {
                        if self.selected_endpoint_index > 0 {
                            self.selected_endpoint_index -= 1;
                        }
                        Ok(None)
                    }
                    KeyCode::Down => {
                        if self.selected_endpoint_index < self.discovered_endpoints.len().saturating_sub(1) {
                            self.selected_endpoint_index += 1;
                        }
                        Ok(None)
                    }
                    KeyCode::Enter | KeyCode::Tab => {                        // Move to authentication step
                        self.step = ConnectDialogStep::Authentication;
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                        Ok(None)
                    }                    KeyCode::Esc => {
                        // Go back to URL step
                        self.step = ConnectDialogStep::ServerUrl;
                        Ok(None)
                    }
                    KeyCode::PageUp => {
                        // Scroll connection log up
                        for _ in 0..5 {
                            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Up);
                        }
                        Ok(None)
                    }
                    KeyCode::PageDown => {
                        // Scroll connection log down
                        for _ in 0..5 {
                            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Down);
                        }
                        Ok(None)
                    }
                    _ => Ok(None)
                }
            }
            ConnectDialogStep::Authentication => {
                match key {                    KeyCode::Up | KeyCode::Down => {
                        // Toggle authentication type
                        self.authentication_type = match self.authentication_type {
                            AuthenticationType::Anonymous => AuthenticationType::UserPassword,
                            AuthenticationType::UserPassword => AuthenticationType::Anonymous,
                        };
                        
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                        Ok(None)
                    }                    KeyCode::Tab => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            // Switch between username and password fields
                            self.active_auth_field = match self.active_auth_field {
                                AuthenticationField::Username => AuthenticationField::Password,
                                AuthenticationField::Password => AuthenticationField::Username,
                            };
                            self.input_mode = InputMode::Editing;
                        }
                        Ok(None)
                    }
                    KeyCode::Enter => {
                        // Connect with selected settings
                        self.connect_with_settings().await
                    }
                    KeyCode::Esc => {
                        // Go back to endpoint selection
                        self.step = ConnectDialogStep::EndpointSelection;
                        self.input_mode = InputMode::Normal;
                        Ok(None)
                    }                    KeyCode::Char(_c) => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            match self.active_auth_field {
                                AuthenticationField::Username => {
                                    self.username_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                                AuthenticationField::Password => {
                                    self.password_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                            }
                        }
                        Ok(None)
                    }
                    KeyCode::Backspace => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            match self.active_auth_field {
                                AuthenticationField::Username => {
                                    self.username_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                                AuthenticationField::Password => {
                                    self.password_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                            }
                        }
                        Ok(None)
                    }
                    KeyCode::Left => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            match self.active_auth_field {
                                AuthenticationField::Username => {
                                    self.username_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                                AuthenticationField::Password => {
                                    self.password_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                            }
                        }
                        Ok(None)
                    }
                    KeyCode::Right => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            match self.active_auth_field {
                                AuthenticationField::Username => {
                                    self.username_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }
                                AuthenticationField::Password => {
                                    self.password_input.handle_event(&crossterm::event::Event::Key(
                                        crossterm::event::KeyEvent::new(key, modifiers)
                                    ));
                                }                            }
                        }
                        Ok(None)
                    }
                    KeyCode::PageUp => {
                        // Scroll connection log up
                        for _ in 0..5 {
                            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Up);
                        }
                        Ok(None)
                    }
                    KeyCode::PageDown => {
                        // Scroll connection log down
                        for _ in 0..5 {
                            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Down);
                        }
                        Ok(None)
                    }
                    _ => Ok(None)
                }
            }
        }
    }

    async fn discover_endpoints(&mut self) -> Result<()> {
        self.connect_in_progress = true;
        self.add_connection_log("Discovering endpoints...");
        
        // Simulate endpoint discovery
        time::sleep(Duration::from_millis(500)).await;
          // Mock discovered endpoints based on URL
        self.discovered_endpoints = vec![
            EndpointInfo {
                endpoint_url: self.server_url_input.value().to_string(),
                security_policy: SecurityPolicy::None,
                security_mode: SecurityMode::None,
                display_name: "None - No Security".to_string(),
            },
            EndpointInfo {
                endpoint_url: self.server_url_input.value().to_string(),
                security_policy: SecurityPolicy::Basic256Sha256,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Basic256Sha256 - Sign & Encrypt".to_string(),
            },
        ];
          self.connect_in_progress = false;
        self.step = ConnectDialogStep::EndpointSelection;
        self.input_mode = InputMode::Normal;
        self.setup_buttons_for_current_step();
        self.add_connection_log(&format!("Found {} endpoints", self.discovered_endpoints.len()));
        
        Ok(())
    }

    async fn connect_with_settings(&mut self) -> Result<Option<ConnectionStatus>> {
        self.connect_in_progress = true;
          let auth_desc = match self.authentication_type {
            AuthenticationType::Anonymous => "Anonymous".to_string(),
            AuthenticationType::UserPassword => format!("User: {}", self.username_input.value()),
        };
        
        self.add_connection_log(&format!("Connecting with {}", auth_desc));
        
        // Simulate connection process
        time::sleep(Duration::from_millis(1000)).await;
        
        // For demo purposes, simulate successful connection
        self.connect_in_progress = false;
        self.add_connection_log("Connected successfully!");
        
        Ok(Some(ConnectionStatus::Connected))
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // Main connect area
                Constraint::Length(8),  // Connection logs
            ])
            .split(area);

        match self.step {            ConnectDialogStep::ServerUrl => self.render_server_url_step(f, chunks[0]),
            ConnectDialogStep::EndpointSelection => self.render_endpoint_step(f, chunks[0]),
            ConnectDialogStep::Authentication => self.render_auth_step(f, chunks[0]),
        }        // Connection logs with scrolling support
        f.render_widget(self.connection_log_textarea.widget(), chunks[1]);
    }

    fn render_server_url_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // URL input
                Constraint::Length(3),  // Buttons
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 1/3: Server URL")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);        // URL input
        let input_style = Style::default().fg(Color::Yellow);
        
        // Use tui-input's built-in scrolling and rendering
        let width = chunks[1].width.max(3) - 3; // Account for borders
        let scroll = self.server_url_input.visual_scroll(width as usize);
        
        let input_text = Paragraph::new(self.server_url_input.value())
            .style(input_style)
            .scroll((0, scroll as u16))
            .block(Block::default()
                .title("Server URL")
                .borders(Borders::ALL)
                .title_style(Style::default().fg(Color::Yellow)));
        
        f.render_widget(input_text, chunks[1]);
        
        // Position cursor if editing
        if self.input_mode == InputMode::Editing {
            let cursor_x = self.server_url_input.visual_cursor().max(scroll) - scroll + 1;
            f.set_cursor(chunks[1].x + cursor_x as u16, chunks[1].y + 1);
        }// Buttons (2 buttons for step 1) - left and right positioning, 50% wider
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space between
                Constraint::Length(18), // Discover button (12 * 1.5 = 18)
            ])
            .split(chunks[2]);

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("discover", false);
        } else {            self.button_manager.set_button_enabled("discover", true);
        }        // Render buttons using button manager (use chunks 0 and 2 for left/right positioning)
        let button_rects = &[button_chunks[0], button_chunks[2]];
        self.button_manager.render_buttons(f, button_rects);
    }

    fn render_endpoint_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Endpoint list
                Constraint::Length(3),  // Buttons
            ])
            .split(area);// Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 2/3: Select Endpoint")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Endpoint list
        let items: Vec<ListItem> = self.discovered_endpoints
            .iter()
            .enumerate()
            .map(|(i, endpoint)| {
                let prefix = if i == self.selected_endpoint_index { "▶ " } else { "  " };
                let security_desc = format!("{:?} - {:?}", endpoint.security_policy, endpoint.security_mode);
                ListItem::new(format!("{}{}  [{}]", prefix, endpoint.display_name, security_desc))
            })
            .collect();

        let endpoint_list = List::new(items)
            .block(Block::default()
                .title("Available Endpoints")
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));        f.render_widget(endpoint_list, chunks[1]);        // Buttons (3 buttons for step 2) - left, center, right positioning, 50% wider
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Back button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Next button (12 * 1.5 = 18)
            ])
            .split(chunks[2]);        // Render buttons using button manager (use chunks 0, 2, 4 for left/center/right positioning)
        let button_rects = &[button_chunks[0], button_chunks[2], button_chunks[4]];
        self.button_manager.render_buttons(f, button_rects);
    }

    fn render_auth_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(5),  // Auth type selection
                Constraint::Length(6),  // User details (if needed)
                Constraint::Length(3),  // Buttons
                Constraint::Min(0),     // Help text
            ])
            .split(area);        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 3/3: Authentication")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Authentication type selection
        let auth_items = vec![
            if self.authentication_type == AuthenticationType::Anonymous {
                "▶ Anonymous (No credentials required)"
            } else {
                "  Anonymous (No credentials required)"
            },
            if self.authentication_type == AuthenticationType::UserPassword {
                "▶ Username & Password"
            } else {
                "  Username & Password"
            }
        ];

        let auth_text = auth_items.join("\n");
        let auth_block = Paragraph::new(auth_text)
            .block(Block::default()
                .title("Authentication Method")
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(auth_block, chunks[1]);

        // User details (if username/password is selected)
        if self.authentication_type == AuthenticationType::UserPassword {
            let user_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Username
                    Constraint::Length(3),  // Password
                ])
                .split(chunks[2]);            // Username field
            let username_style = if self.active_auth_field == AuthenticationField::Username && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let width = user_chunks[0].width.max(3) - 3;
            let scroll = self.username_input.visual_scroll(width as usize);
            
            let username_text = Paragraph::new(self.username_input.value())
                .style(username_style)
                .scroll((0, scroll as u16))
                .block(Block::default()
                    .title("Username")
                    .borders(Borders::ALL));
            f.render_widget(username_text, user_chunks[0]);
            
            // Position cursor if editing username
            if self.active_auth_field == AuthenticationField::Username && self.input_mode == InputMode::Editing {
                let cursor_x = self.username_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(user_chunks[0].x + cursor_x as u16, user_chunks[0].y + 1);
            }

            // Password field
            let password_style = if self.active_auth_field == AuthenticationField::Password && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let width = user_chunks[1].width.max(3) - 3;
            let scroll = self.password_input.visual_scroll(width as usize);
            let password_display = "*".repeat(self.password_input.value().len());
            
            let password_text = Paragraph::new(password_display)
                .style(password_style)
                .scroll((0, scroll as u16))
                .block(Block::default()
                    .title("Password")
                    .borders(Borders::ALL));
            f.render_widget(password_text, user_chunks[1]);
            
            // Position cursor if editing password
            if self.active_auth_field == AuthenticationField::Password && self.input_mode == InputMode::Editing {
                let cursor_x = self.password_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(user_chunks[1].x + cursor_x as u16, user_chunks[1].y + 1);
            }}        // Buttons (3 buttons for step 3) - left, center, right positioning, 50% wider
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Back button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Connect button (12 * 1.5 = 18)
            ])
            .split(chunks[3]);

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("connect", false);
        } else {
            self.button_manager.set_button_enabled("connect", true);        }

        // Render buttons using button manager (use chunks 0, 2, 4 for left/center/right positioning)
        let button_rects = &[button_chunks[0], button_chunks[2], button_chunks[4]];
        self.button_manager.render_buttons(f, button_rects);

        // Help text
        let help_text = if self.authentication_type == AuthenticationType::UserPassword {
            "↑↓ - Change auth type | Tab - Switch fields | Alt+C - Connect | Alt+B - Back | Alt+X - Cancel"
        } else {
            "↑↓ - Change auth type | Alt+C - Connect | Alt+B - Back | Alt+X - Cancel"
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[4]);
    }    pub fn add_connection_log(&mut self, message: &str) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        
        // Handle multiline messages with proper indentation
        let lines: Vec<&str> = message.lines().collect();
        if lines.is_empty() {
            return;
        }
        
        // First line gets the timestamp
        let first_line = format!("{} {}", timestamp, lines[0]);
        
        // Move to end of textarea and add new line if not empty
        self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::End);
        if !self.connection_log_textarea.is_empty() {
            self.connection_log_textarea.insert_newline();
        }
        self.connection_log_textarea.insert_str(&first_line);
        
        // Additional lines get 2-space indentation
        for line in lines.iter().skip(1) {
            self.connection_log_textarea.insert_newline();
            self.connection_log_textarea.insert_str(&format!("  {}", line));
        }
        
        // Auto-scroll to bottom to show latest entry
        self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::End);
        
        // Limit the number of lines to prevent memory issues
        while self.connection_log_textarea.lines().len() > 50 {
            self.connection_log_textarea.move_cursor(tui_textarea::CursorMove::Head);
            self.connection_log_textarea.delete_line_by_head();
        }
    }pub fn reset(&mut self) {
        self.step = ConnectDialogStep::ServerUrl;
        self.discovered_endpoints.clear();
        self.selected_endpoint_index = 0;
        self.authentication_type = AuthenticationType::Anonymous;
        self.username_input.reset();
        self.password_input.reset();
        self.connect_in_progress = false;
        self.input_mode = InputMode::Editing;
        self.setup_buttons_for_current_step();
    }

    pub fn is_connecting(&self) -> bool {
        self.connect_in_progress
    }    fn setup_buttons_for_current_step(&mut self) {
        self.button_manager.clear();
        
        match self.step {
            ConnectDialogStep::ServerUrl => {                // Step 1: Only Cancel and Discover
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("discover", "Discover")
                        .with_hotkey('d')
                        .with_ctrl_key('d')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.connect_in_progress)
                );
            }
            ConnectDialogStep::EndpointSelection => {                // Step 2: Cancel, Back, Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue)
                );
                
                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.discovered_endpoints.is_empty())
                );
            }
            ConnectDialogStep::Authentication => {                // Step 3: Cancel, Back, Connect
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue)
                );
                  self.button_manager.add_button(
                    Button::new("connect", "Connect")
                        .with_hotkey('o')
                        .with_ctrl_key('c')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.connect_in_progress)
                );
            }
        }
    }      pub async fn handle_button_action(&mut self, button_id: &str) -> Result<Option<ConnectionStatus>> {
        match button_id {
            "back" => {
                match self.step {                    ConnectDialogStep::EndpointSelection => {
                        self.step = ConnectDialogStep::ServerUrl;
                        self.input_mode = InputMode::Editing;
                    }
                    ConnectDialogStep::Authentication => {
                        self.step = ConnectDialogStep::EndpointSelection;
                        self.input_mode = InputMode::Normal;
                    }
                    _ => {}
                }
                self.setup_buttons_for_current_step();
                Ok(None)
            }            "discover" => {
                self.discover_endpoints().await?;
                Ok(None)
            }
            "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl => {
                        if !self.discovered_endpoints.is_empty() {
                            self.step = ConnectDialogStep::EndpointSelection;
                            self.input_mode = InputMode::Normal;
                            self.setup_buttons_for_current_step();
                        }
                    }
                    ConnectDialogStep::EndpointSelection => {
                        self.step = ConnectDialogStep::Authentication;                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                        self.setup_buttons_for_current_step();
                    }
                    _ => {}
                }
                Ok(None)
            }
            "connect" => {
                self.connect_with_settings().await
            }
            "cancel" => {
                Ok(Some(ConnectionStatus::Disconnected))
            }
            _ => Ok(None)        }
    }

    pub fn handle_mouse_click(&mut self, column: u16, row: u16) -> Option<String> {
        self.button_manager.handle_mouse_click(column, row)
    }

    pub fn handle_mouse_down(&mut self, column: u16, row: u16) -> bool {
        self.button_manager.handle_mouse_down(column, row)
    }

    pub fn handle_mouse_up(&mut self, column: u16, row: u16) -> Option<String> {
        self.button_manager.handle_mouse_up(column, row)
    }
}
