use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::time::Duration;
use tokio::time;

use crate::client::ConnectionStatus;
use crate::components::{Button, ButtonManager};

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
    Aes128Sha256RsaOaep,
    Aes256Sha256RsaPss,
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
    pub server_url: String,    pub discovered_endpoints: Vec<EndpointInfo>,
    pub selected_endpoint_index: usize,
    pub authentication_type: AuthenticationType,
    pub active_auth_field: AuthenticationField,
    pub username: String,
    pub password: String,
    pub connect_in_progress: bool,
    
    // Input handling
    pub input_mode: InputMode,
    pub cursor_position: usize,
    
    // Button management
    pub button_manager: ButtonManager,
    
    // Logging
    pub connection_logs: Vec<String>,
}

impl ConnectScreen {
    pub fn new() -> Self {
        let mut screen = Self {
            step: ConnectDialogStep::ServerUrl,
            server_url: "opc.tcp://localhost:4840".to_string(),
            discovered_endpoints: Vec::new(),
            selected_endpoint_index: 0,
            authentication_type: AuthenticationType::Anonymous,
            active_auth_field: AuthenticationField::Username,
            username: String::new(),
            password: String::new(),
            connect_in_progress: false,
            input_mode: InputMode::Editing,
            cursor_position: 24, // Position at end of default URL
            button_manager: ButtonManager::new(),
            connection_logs: vec![
                "[App Start] OPC UA Client initialized".to_string(),
                "[Ready] Enter server URL and press Enter to discover endpoints".to_string()
            ],
        };
        
        screen.setup_buttons_for_current_step();
        screen
    }    pub async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {        // Handle button input first
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
                    }
                    KeyCode::Esc => {
                        Ok(Some(ConnectionStatus::Disconnected))
                    }                    KeyCode::Char(c) => {
                        if self.input_mode == InputMode::Editing {
                            self.server_url.insert(self.cursor_position, c);
                            self.cursor_position += 1;
                        }
                        Ok(None)
                    }
                    KeyCode::Backspace => {
                        if self.input_mode == InputMode::Editing && self.cursor_position > 0 {
                            self.cursor_position -= 1;
                            self.server_url.remove(self.cursor_position);
                        }
                        Ok(None)
                    }
                    KeyCode::Left => {
                        if self.input_mode == InputMode::Editing && self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                        Ok(None)
                    }
                    KeyCode::Right => {
                        if self.input_mode == InputMode::Editing && self.cursor_position < self.server_url.len() {
                            self.cursor_position += 1;
                        }
                        Ok(None)
                    }
                    _ => Ok(None)
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
                    KeyCode::Enter | KeyCode::Tab => {
                        // Move to authentication step
                        self.step = ConnectDialogStep::Authentication;
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.cursor_position = self.username.len();
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                        Ok(None)
                    }
                    KeyCode::Esc => {
                        // Go back to URL step
                        self.step = ConnectDialogStep::ServerUrl;
                        self.cursor_position = self.server_url.len();
                        Ok(None)
                    }
                    _ => Ok(None)
                }
            }
            ConnectDialogStep::Authentication => {
                match key {
                    KeyCode::Up | KeyCode::Down => {
                        // Toggle authentication type
                        self.authentication_type = match self.authentication_type {
                            AuthenticationType::Anonymous => AuthenticationType::UserPassword,
                            AuthenticationType::UserPassword => AuthenticationType::Anonymous,
                        };
                        
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.cursor_position = self.username.len();
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                        Ok(None)
                    }
                    KeyCode::Tab => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            // Switch between username and password fields
                            self.active_auth_field = match self.active_auth_field {
                                AuthenticationField::Username => AuthenticationField::Password,
                                AuthenticationField::Password => AuthenticationField::Username,
                            };
                            // Update cursor position for the new field
                            self.cursor_position = match self.active_auth_field {
                                AuthenticationField::Username => self.username.len(),
                                AuthenticationField::Password => self.password.len(),
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
                    }
                    KeyCode::Char(c) => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            match self.active_auth_field {
                                AuthenticationField::Username => {
                                    self.username.insert(self.cursor_position, c);
                                    self.cursor_position += 1;
                                }
                                AuthenticationField::Password => {
                                    self.password.insert(self.cursor_position, c);
                                    self.cursor_position += 1;
                                }
                            }
                        }
                        Ok(None)
                    }
                    KeyCode::Backspace => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            match self.active_auth_field {
                                AuthenticationField::Username => {
                                    if self.cursor_position > 0 {
                                        self.cursor_position -= 1;
                                        self.username.remove(self.cursor_position);
                                    }
                                }
                                AuthenticationField::Password => {
                                    if self.cursor_position > 0 {
                                        self.cursor_position -= 1;
                                        self.password.remove(self.cursor_position);
                                    }
                                }
                            }
                        }
                        Ok(None)
                    }
                    KeyCode::Left => {
                        if self.authentication_type == AuthenticationType::UserPassword && self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                        Ok(None)
                    }
                    KeyCode::Right => {
                        if self.authentication_type == AuthenticationType::UserPassword {
                            let max_len = match self.active_auth_field {
                                AuthenticationField::Username => self.username.len(),
                                AuthenticationField::Password => self.password.len(),
                            };
                            if self.cursor_position < max_len {
                                self.cursor_position += 1;
                            }
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
                endpoint_url: self.server_url.clone(),
                security_policy: SecurityPolicy::None,
                security_mode: SecurityMode::None,
                display_name: "None - No Security".to_string(),
            },
            EndpointInfo {
                endpoint_url: self.server_url.clone(),
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
            AuthenticationType::UserPassword => format!("User: {}", self.username),
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
        }

        // Connection logs
        let log_text = self.connection_logs.join("\n");
        let logs_block = Paragraph::new(log_text)
            .block(Block::default()
                .title("Connection Log")
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });
        f.render_widget(logs_block, chunks[1]);
    }

    fn render_server_url_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // URL input
                Constraint::Length(3),  // Buttons
                Constraint::Min(0),     // Help text
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 1/3: Server URL")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // URL input
        let input_style = if self.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow).bg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };
        
        let input_text = self.format_input_with_cursor(&self.server_url, true);
        let input_block = Paragraph::new(input_text)
            .block(Block::default()
                .title("Server URL")
                .borders(Borders::ALL))
            .style(input_style);
        f.render_widget(input_block, chunks[1]);        // Buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[2]);

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("discover", false);
        } else {
            self.button_manager.set_button_enabled("discover", true);
        }
        
        if !self.discovered_endpoints.is_empty() {
            self.button_manager.set_button_enabled("next", true);
        }

        // Render buttons using button manager
        self.button_manager.render_buttons(f, &button_chunks);

        // Help text
        let help_text = if self.connect_in_progress {
            "Please wait while discovering server endpoints..."
        } else {
            "Enter server URL above, then use buttons below or shortcuts\nAlt+D - Discover | Alt+N - Next | Tab - Navigate | Esc - Cancel"
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[3]);
    }

    fn render_endpoint_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Endpoint list
                Constraint::Length(3),  // Buttons
                Constraint::Length(3),  // Help text
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 2/3: Select Endpoint")
            .style(Style::default().fg(Color::Yellow))
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
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(endpoint_list, chunks[1]);

        // Buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])            .split(chunks[2]);

        // Render buttons using button manager
        self.button_manager.render_buttons(f, &button_chunks);

        // Help text
        let help = Paragraph::new("↑↓ - Navigate endpoints | Alt+S - Select | Alt+N - Next | Alt+B - Back")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[3]);
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
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 3/3: Authentication")
            .style(Style::default().fg(Color::Yellow))
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
                .split(chunks[2]);

            // Username field
            let username_style = if self.active_auth_field == AuthenticationField::Username && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow).bg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };
            
            let username_text = self.format_input_with_cursor(&self.username, self.active_auth_field == AuthenticationField::Username);
            let username_block = Paragraph::new(username_text)
                .block(Block::default()
                    .title("Username")
                    .borders(Borders::ALL))
                .style(username_style);
            f.render_widget(username_block, user_chunks[0]);

            // Password field
            let password_style = if self.active_auth_field == AuthenticationField::Password && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow).bg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };
            
            let password_display = "*".repeat(self.password.len());
            let password_text = self.format_input_with_cursor(&password_display, self.active_auth_field == AuthenticationField::Password);
            let password_block = Paragraph::new(password_text)
                .block(Block::default()
                    .title("Password")
                    .borders(Borders::ALL))
                .style(password_style);
            f.render_widget(password_block, user_chunks[1]);
        }        // Buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[3]);

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("connect", false);
        } else {
            self.button_manager.set_button_enabled("connect", true);
        }

        // Render buttons using button manager
        self.button_manager.render_buttons(f, &button_chunks);

        // Help text
        let help_text = if self.authentication_type == AuthenticationType::UserPassword {
            "↑↓ - Change auth type | Tab - Switch fields | Alt+C - Connect | Alt+X - Cancel"
        } else {
            "↑↓ - Change auth type | Alt+C - Connect | Alt+X - Cancel"
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[4]);
    }

    fn format_input_with_cursor<'a>(&self, text: &'a str, show_cursor: bool) -> Line<'a> {
        if !show_cursor || self.input_mode != InputMode::Editing {
            return Line::from(text.to_string());
        }

        let mut spans = Vec::new();
        
        if self.cursor_position == 0 {
            spans.push(Span::styled("█", Style::default().bg(Color::White).fg(Color::Black)));
            spans.push(Span::raw(text));
        } else if self.cursor_position >= text.len() {
            spans.push(Span::raw(text));
            spans.push(Span::styled("█", Style::default().bg(Color::White).fg(Color::Black)));
        } else {
            let (before, after) = text.split_at(self.cursor_position);
            let cursor_char = after.chars().next().unwrap_or(' ');
            let remaining = &after[cursor_char.len_utf8()..];
            
            spans.push(Span::raw(before));
            spans.push(Span::styled(cursor_char.to_string(), Style::default().bg(Color::White).fg(Color::Black)));
            spans.push(Span::raw(remaining));
        }
        
        Line::from(spans)
    }

    pub fn add_connection_log(&mut self, message: &str) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        self.connection_logs.push(format!("[{}] {}", timestamp, message));
        
        // Keep only last 50 log entries
        if self.connection_logs.len() > 50 {
            self.connection_logs.remove(0);
        }
    }    pub fn reset(&mut self) {
        self.step = ConnectDialogStep::ServerUrl;
        self.discovered_endpoints.clear();
        self.selected_endpoint_index = 0;
        self.authentication_type = AuthenticationType::Anonymous;
        self.username.clear();
        self.password.clear();
        self.connect_in_progress = false;
        self.input_mode = InputMode::Editing;
        self.cursor_position = self.server_url.len();
        self.setup_buttons_for_current_step();
    }

    pub fn is_connecting(&self) -> bool {
        self.connect_in_progress
    }

    fn setup_buttons_for_current_step(&mut self) {
        self.button_manager.clear();
        
        match self.step {
            ConnectDialogStep::ServerUrl => {
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_enabled(false) // First step, no back
                );
                
                self.button_manager.add_button(
                    Button::new("discover", "Discover Endpoints")
                        .with_hotkey('d')
                        .with_ctrl_key('d')
                        .with_enabled(!self.connect_in_progress)
                );
                
                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_enabled(false) // Only after discovery
                );
            }
            ConnectDialogStep::EndpointSelection => {
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_enabled(true)
                );
                
                self.button_manager.add_button(
                    Button::new("select", "Select Endpoint")
                        .with_hotkey('s')
                        .with_enabled(!self.discovered_endpoints.is_empty())
                );
                
                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_enabled(!self.discovered_endpoints.is_empty())
                );
            }
            ConnectDialogStep::Authentication => {
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_enabled(true)
                );
                
                self.button_manager.add_button(
                    Button::new("connect", "Connect")
                        .with_hotkey('c')
                        .with_ctrl_key('c')
                        .with_enabled(!self.connect_in_progress)
                );
                
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('x')
                        .with_enabled(true)
                );
            }
        }
    }
    
    pub async fn handle_button_action(&mut self, button_id: &str) -> Result<Option<ConnectionStatus>> {
        match button_id {
            "back" => {
                match self.step {
                    ConnectDialogStep::EndpointSelection => {
                        self.step = ConnectDialogStep::ServerUrl;
                        self.cursor_position = self.server_url.len();
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
            }
            "discover" => {
                self.discover_endpoints().await?;
                Ok(None)
            }
            "select" | "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl => {
                        if !self.discovered_endpoints.is_empty() {
                            self.step = ConnectDialogStep::EndpointSelection;
                            self.input_mode = InputMode::Normal;
                            self.setup_buttons_for_current_step();
                        }
                    }
                    ConnectDialogStep::EndpointSelection => {
                        self.step = ConnectDialogStep::Authentication;
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.cursor_position = self.username.len();
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
            _ => Ok(None)
        }
    }
    
    pub fn handle_mouse_click(&mut self, column: u16, row: u16) -> Option<String> {
        self.button_manager.handle_mouse_click(column, row)
    }
}
