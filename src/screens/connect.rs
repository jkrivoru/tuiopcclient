use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use log::{info, warn, error, debug};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::time::Duration;
use tokio::time;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use tui_logger::{TuiLoggerWidget, TuiWidgetState, TuiWidgetEvent, TuiLoggerLevelOutput};

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

pub struct ConnectScreen {    // Connection dialog state
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
    
    // Logger widget state
    pub logger_widget_state: TuiWidgetState,
    
    // Button management
    pub button_manager: ButtonManager,
}

impl ConnectScreen {    pub fn new() -> Self {        let mut screen = Self {
            step: ConnectDialogStep::ServerUrl,
            server_url_input: Input::default().with_value("opc.tcp://localhost:4840".to_string()),
            discovered_endpoints: Vec::new(),
            selected_endpoint_index: usize::default(),
            authentication_type: AuthenticationType::Anonymous,
            active_auth_field: AuthenticationField::Username,
            username_input: Input::default(),
            password_input: Input::default(),
            connect_in_progress: false,
            input_mode: InputMode::Editing,
            logger_widget_state: TuiWidgetState::new(),
            button_manager: ButtonManager::new(),
        };// Add initial log messages using the log crate
        info!("OPC UA Client initialized");
        info!("Loading configuration from config.json");
        warn!("Configuration file not found, using defaults");
        info!("Default server URL loaded");
        info!("Enter server URL and press Alt+N to discover endpoints");
        info!("Log Navigation: PageUp/PageDown to scroll, Home/End to jump, Escape to return to latest");
        info!("Keyboard: Alt+N to Next, Alt+C to Cancel");
        info!("Connection log initialized");
        debug!("Button manager created with hotkeys");
        debug!("Input handlers configured");
        info!("Connect screen ready");
        
        screen.setup_buttons_for_current_step();
        screen
    }    pub async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {
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
                        self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                        Ok(None)
                    }
                    KeyCode::PageDown => {
                        // Scroll connection log down
                        self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                        Ok(None)
                    }
                    KeyCode::Home => {
                        // Go to the beginning - scroll up multiple pages
                        for _ in 0..10 {
                            self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                        }
                        Ok(None)
                    }
                    KeyCode::End => {
                        // Go to the end (latest messages) - exit page mode
                        self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
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
                        self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                        Ok(None)
                    }
                    KeyCode::PageDown => {
                        // Scroll connection log down
                        self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                        Ok(None)
                    }
                    KeyCode::Home => {
                        // Go to the beginning - scroll up multiple pages
                        for _ in 0..10 {
                            self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                        }
                        Ok(None)
                    }
                    KeyCode::End => {
                        // Go to the end (latest messages) - exit page mode
                        self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
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
                        }                        Ok(None)
                    }                    KeyCode::PageUp => {
                        // Scroll connection log up
                        self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                        Ok(None)
                    }
                    KeyCode::PageDown => {
                        // Scroll connection log down
                        self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                        Ok(None)
                    }
                    KeyCode::Home => {
                        // Go to the beginning - scroll up multiple pages
                        for _ in 0..10 {
                            self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                        }
                        Ok(None)
                    }
                    KeyCode::End => {
                        // Go to the end (latest messages) - exit page mode
                        self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
                        Ok(None)
                    }
                    _ => Ok(None)
                }
            }
        }
    }    async fn discover_endpoints(&mut self) -> Result<()> {
        self.connect_in_progress = true;
        info!("Discovering endpoints...");
        warn!("Using demo mode - actual endpoint discovery not implemented yet");
        
        // Check for invalid URL to demonstrate error logging
        let url = self.server_url_input.value();
        if !url.starts_with("opc.tcp://") {
            error!("Invalid OPC UA server URL: must start with 'opc.tcp://'");
            self.connect_in_progress = false;
            return Ok(());
        }
        
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
        ];        self.connect_in_progress = false;
        self.step = ConnectDialogStep::EndpointSelection;
        self.setup_buttons_for_current_step();
        self.input_mode = InputMode::Normal;
        info!("Found {} endpoints", self.discovered_endpoints.len());
        
        Ok(())
    }

    async fn connect_with_settings(&mut self) -> Result<Option<ConnectionStatus>> {
        self.connect_in_progress = true;
          let auth_desc = match self.authentication_type {
            AuthenticationType::Anonymous => "Anonymous".to_string(),
            AuthenticationType::UserPassword => format!("User: {}", self.username_input.value()),
        };
        
        info!("Connecting with {}", auth_desc);
        
        // Simulate connection process
        time::sleep(Duration::from_millis(1000)).await;
        
        // For demo purposes, simulate successful connection
        self.connect_in_progress = false;
        info!("Connected successfully!");
        
        Ok(Some(ConnectionStatus::Connected))
    }    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Move events from hot buffer to main buffer
        tui_logger::move_events();
        
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
        let logger_widget = TuiLoggerWidget::default()
            .block(
                Block::default()
                    .title("Connection Log")
                    .borders(Borders::ALL)
            )
            // Custom formatting: datetime + severity only, no callstack
            .output_timestamp(Some("%Y-%m-%d %H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Long))
            .output_target(false)  // Disable target/module name
            .output_file(false)    // Disable file name
            .output_line(false)    // Disable line number
            .output_separator(' ') // Use space instead of colon
            // Color coding: Info - standard (white), Warning - yellow, Error - red
            .style_info(Style::default().fg(Color::White))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::DarkGray))
            .style_trace(Style::default().fg(Color::Gray))
            .state(&self.logger_widget_state);
        f.render_widget(logger_widget, chunks[1]);
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
                Constraint::Length(18), // Next button (12 * 1.5 = 18)
            ])
            .split(chunks[2]);        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("next", false);
        } else {
            self.button_manager.set_button_enabled("next", true);
        }// Render buttons using button manager (use chunks 0 and 2 for left/right positioning)
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
        f.render_widget(help, chunks[4]);    }

    pub fn reset(&mut self) {
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
            ConnectDialogStep::ServerUrl => {
                // Step 1: Only Cancel and Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                  self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
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
            }            "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl => {
                        // Discover endpoints first
                        self.discover_endpoints().await?;
                    }
                    ConnectDialogStep::EndpointSelection => {
                        if !self.discovered_endpoints.is_empty() {
                            self.step = ConnectDialogStep::Authentication;
                            if self.authentication_type == AuthenticationType::UserPassword {
                                self.active_auth_field = AuthenticationField::Username;
                                self.input_mode = InputMode::Editing;
                            } else {
                                self.input_mode = InputMode::Normal;
                            }
                            self.setup_buttons_for_current_step();
                        }
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
    }    fn get_help_text(&self) -> String {
        // Always show all help information in one line, context-aware for current step
        match self.step {
            ConnectDialogStep::ServerUrl => {
                "PageUp / PageDown - scroll log | Alt+C - Cancel | Alt+N - Next".to_string()
            }
            ConnectDialogStep::EndpointSelection => {
                "PageUp / PageDown - scroll log | Alt+C - Cancel | Alt+B - Back | Alt+N - Next".to_string()
            }
            ConnectDialogStep::Authentication => {
                "PageUp / PageDown - scroll log | Alt+C - Cancel | Alt+B - Back | Alt+O - Connect".to_string()
            }
        }
    }

    pub fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help_text = self.get_help_text();
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help_paragraph, area);
    }}
