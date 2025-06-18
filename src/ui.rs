use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEvent, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::{
    io::{self, Stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::client::{OpcUaClientManager, ConnectionStatus, SubscriptionItem};
use crate::menu::{MenuRenderer, MenuType};
use crate::statusbar::{StatusBarRenderer, Screen};

pub struct App {
    client_manager: Arc<Mutex<OpcUaClientManager>>,
    current_screen: Screen,
    should_quit: bool,    // Connection dialog
    server_url: String,
    security_policy: SecurityPolicy,
    // Enhanced connection dialog
    connect_dialog_step: ConnectDialogStep,
    discovered_endpoints: Vec<EndpointInfo>,
    selected_endpoint_index: usize,    authentication_type: AuthenticationType,
    active_auth_field: AuthenticationField,
    username: String,
    password: String,
    connect_in_progress: bool,
    // Browse tree
    browse_items: Vec<BrowseItem>,
    browse_state: ListState,
    current_node_id: String,
    // Subscriptions
    subscriptions: Vec<SubscriptionItem>,
    subscription_state: ListState,
    // Status
    status_message: String,
    connection_status: ConnectionStatus,    // Dialogs
    show_connect_dialog: bool,
    show_connect_in_main: bool,  // Show connect UI in main screen
    show_security_dialog: bool,
    show_node_properties: bool,
    show_write_dialog: bool,
    show_server_info_dialog: bool,
    selected_node: Option<BrowseItem>,
    write_value: String,
    // Connection logs
    connection_logs: Vec<String>,
    // Input mode
    input_mode: InputMode,
    cursor_position: usize,    // UI components
    menu_renderer: MenuRenderer,
    statusbar_renderer: StatusBarRenderer,
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

#[derive(Debug, Clone)]
pub struct EndpointInfo {
    pub endpoint_url: String,
    pub security_policy: SecurityPolicy,
    pub security_mode: SecurityMode,
    pub display_name: String,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectDialogStep {
    ServerUrl,
    EndpointSelection,
    Authentication,
}

#[derive(Debug, Clone)]
pub struct BrowseItem {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub is_folder: bool,
    pub value: Option<String>,
    pub data_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

impl App {    pub fn new(client_manager: Arc<Mutex<OpcUaClientManager>>) -> Self {
        let mut browse_state = ListState::default();
        browse_state.select(Some(0));
        
        let mut subscription_state = ListState::default();
        subscription_state.select(Some(0));

        Self {
            client_manager,
            current_screen: Screen::Main,
            should_quit: false,            server_url: "opc.tcp://localhost:4840".to_string(),
            security_policy: SecurityPolicy::None,
            // Enhanced connection dialog
            connect_dialog_step: ConnectDialogStep::ServerUrl,
            discovered_endpoints: Vec::new(),
            selected_endpoint_index: 0,            authentication_type: AuthenticationType::Anonymous,
            active_auth_field: AuthenticationField::Username,
            username: String::new(),
            password: String::new(),
            connect_in_progress: false,
            browse_items: Vec::new(),
            browse_state,
            current_node_id: "ns=0;i=85".to_string(), // Objects folder
            subscriptions: Vec::new(),
            subscription_state,
            status_message: "Ready".to_string(),
            connection_status: ConnectionStatus::Disconnected,            show_connect_dialog: false,
            show_connect_in_main: true,  // Show connect interface on startup
            show_security_dialog: false,
            show_node_properties: false,
            show_write_dialog: false,
            show_server_info_dialog: false,            selected_node: None,
            write_value: String::new(),
            // Connection logs
            connection_logs: vec!["[App Start] OPC UA Client initialized".to_string(), "[Ready] Enter server URL and press Enter to discover endpoints".to_string()],
            // Input mode
            input_mode: InputMode::Editing,  // Start in editing mode for connect
            cursor_position: 24,  // Position at end of default URL
            // UI components
            menu_renderer: MenuRenderer::new(),
            statusbar_renderer: StatusBarRenderer::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        execute!(terminal.backend_mut(), crossterm::event::EnableMouseCapture)?;

        let result = self.run_app(&mut terminal).await;        disable_raw_mode()?;
        execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture)?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        result
    }

    async fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.ui(f))?;

            let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));            if crossterm::event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_input(key.code, key.modifiers).await?;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_event(mouse).await?;
                    }
                    _ => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick().await;
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }    async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        // Handle connect dialog inputs (both editing and navigation)
        if self.show_connect_dialog || self.show_connect_in_main {
            return self.handle_connect_dialog_input(key).await;
        }

        if self.input_mode == InputMode::Editing {
            
            match key {
                KeyCode::Enter => {
                    if self.show_write_dialog {
                        self.write_node_value().await?;
                        self.show_write_dialog = false;
                        self.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Esc => {
                    self.show_write_dialog = false;
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => {
                    if self.show_write_dialog {
                        self.write_value.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                    }
                }
                KeyCode::Backspace => {
                    if self.show_write_dialog && self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.write_value.remove(self.cursor_position);
                    }
                }
                KeyCode::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    let max_len = if self.show_write_dialog {
                        self.write_value.len()
                    } else {
                        0
                    };
                    if self.cursor_position < max_len {
                        self.cursor_position += 1;
                    }
                }
                _ => {}
            }
            return Ok(());
        }        // Handle dialog inputs
        if self.show_security_dialog || self.show_node_properties || self.show_server_info_dialog {
            match key {
                KeyCode::Esc => {
                    self.show_security_dialog = false;
                    self.show_node_properties = false;
                    self.show_server_info_dialog = false;
                }
                _ => {}
            }
            return Ok(());
        }

        // Global hotkeys
        if modifiers.contains(KeyModifiers::ALT) {
            match key {                KeyCode::Char('f') => {
                    // File menu
                    self.open_connect_dialog();
                }
                KeyCode::Char('h') => {
                    self.current_screen = Screen::Help;
                }
                KeyCode::Char('x') => {
                    self.should_quit = true;
                }
                _ => {}
            }
            return Ok(());
        }

        // Screen-specific inputs
        match self.current_screen {
            Screen::Main => {
                match key {
                    KeyCode::Char('1') => self.current_screen = Screen::Browse,
                    KeyCode::Char('2') => self.current_screen = Screen::Subscriptions,
                    KeyCode::Char('3') => self.current_screen = Screen::Help,                    KeyCode::Char('c') => {
                        self.open_connect_dialog();
                    }
                    KeyCode::Char('d') => {
                        self.disconnect_from_server().await?;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    _ => {}
                }
            }
            Screen::Browse => {
                match key {
                    KeyCode::Up => {
                        if let Some(selected) = self.browse_state.selected() {
                            if selected > 0 {
                                self.browse_state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(selected) = self.browse_state.selected() {
                            if selected < self.browse_items.len().saturating_sub(1) {
                                self.browse_state.select(Some(selected + 1));
                            }
                        }
                    }
                    KeyCode::Enter => {                if let Some(selected) = self.browse_state.selected() {
                    if let Some(item) = self.browse_items.get(selected).cloned() {
                        if item.is_folder {
                            self.browse_node(&item.node_id).await?;
                        }
                    }
                }
                    }
                    KeyCode::Char('a') => {
                        if let Some(selected) = self.browse_state.selected() {
                            if let Some(item) = self.browse_items.get(selected) {
                                self.add_to_subscription(item.clone()).await?;
                            }
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Some(selected) = self.browse_state.selected() {
                            if let Some(item) = self.browse_items.get(selected) {
                                self.selected_node = Some(item.clone());
                                self.show_node_properties = true;
                            }
                        }
                    }
                    KeyCode::Char('w') => {
                        if let Some(selected) = self.browse_state.selected() {
                            if let Some(item) = self.browse_items.get(selected) {
                                self.selected_node = Some(item.clone());
                                self.write_value.clear();
                                self.show_write_dialog = true;
                                self.input_mode = InputMode::Editing;
                                self.cursor_position = 0;
                            }
                        }
                    }
                    KeyCode::Char('b') => {
                        // Browse parent (go back)
                        self.browse_parent().await?;
                    }
                    KeyCode::Esc => self.current_screen = Screen::Main,
                    _ => {}
                }
            }
            Screen::Subscriptions => {
                match key {
                    KeyCode::Up => {
                        if let Some(selected) = self.subscription_state.selected() {
                            if selected > 0 {
                                self.subscription_state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(selected) = self.subscription_state.selected() {
                            if selected < self.subscriptions.len().saturating_sub(1) {
                                self.subscription_state.select(Some(selected + 1));
                            }
                        }
                    }
                    KeyCode::Delete | KeyCode::Char('d') => {
                        if let Some(selected) = self.subscription_state.selected() {
                            self.remove_from_subscription(selected).await?;
                        }
                    }
                    KeyCode::Esc => self.current_screen = Screen::Main,
                    _ => {}
                }
            }
            Screen::Help => {
                match key {
                    KeyCode::Esc => self.current_screen = Screen::Main,
                    _ => {}                }
            }
        }

        Ok(())
    }    async fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is on menu bar (first row)
                if mouse.row == 0 {
                    self.handle_menu_click(mouse.column).await?;
                }
                // Check if click is on dropdown menu
                else if let Some(menu_type) = self.menu_renderer.get_active_menu() {
                    self.handle_dropdown_click(menu_type, mouse.column, mouse.row).await?;
                }
                // Click elsewhere closes menu
                else {
                    self.menu_renderer.close_menu();
                }
            }
            _ => {}
        }
        Ok(())
    }async fn handle_menu_click(&mut self, column: u16) -> Result<()> {
        // Update menu renderer connection status first
        self.menu_renderer.set_connection_status(self.connection_status.clone());
        
        if let Some(_menu_type) = self.menu_renderer.handle_menu_click(column) {
            // Menu state updated in renderer
        }
        Ok(())
    }    async fn handle_dropdown_click(&mut self, menu_type: MenuType, column: u16, row: u16) -> Result<()> {
        match menu_type {
            MenuType::File => {
                // File dropdown: x=1, y=1, width=25, height=6
                if column >= 1 && column <= 25 && row >= 1 && row <= 6 {
                    match row {                        2 => {
                            // "Connect..." clicked
                            self.open_connect_dialog();
                            self.menu_renderer.close_menu();
                        }
                        3 => {
                            // "Server Info..." clicked
                            self.show_server_info_dialog = true;
                            self.menu_renderer.close_menu();
                        }
                        5 => {
                            // "Exit" clicked (after separator line)                            self.should_quit = true;
                            self.menu_renderer.close_menu();
                        }
                        _ => {}
                    }
                } else {
                    // Click outside dropdown closes it
                    self.menu_renderer.close_menu();
                }            }
            MenuType::Browse => {
                // Browse dropdown: x=9, y=1, width=20, height=4
                if column >= 9 && column <= 28 && row >= 1 && row <= 4 {
                    match row {
                        2 => {
                            // "Browse Server" clicked
                            self.current_screen = Screen::Browse;
                            self.menu_renderer.close_menu();
                        }
                        3 => {
                            // "Refresh" clicked
                            if self.connection_status == ConnectionStatus::Connected {
                                let _ = self.browse_node(&self.current_node_id.clone()).await;
                            }
                            self.menu_renderer.close_menu();
                        }
                        _ => {}
                    }
                } else {
                    // Click outside dropdown closes it
                    self.menu_renderer.close_menu();
                }
            }
        }
        Ok(())
    }    async fn on_tick(&mut self) {
        // Update connection status and subscriptions
        if let Ok(client) = self.client_manager.try_lock() {
            self.connection_status = client.get_connection_status();
            if let Ok(subs) = client.get_subscription_items().await {
                self.subscriptions = subs;
            }
        }
        
        // Update renderers with current state
        self.menu_renderer.set_connection_status(self.connection_status.clone());
        self.statusbar_renderer.set_connection_status(self.connection_status.clone());
        self.statusbar_renderer.set_current_screen(self.current_screen.clone());
        self.statusbar_renderer.set_status_message(self.status_message.clone());
    }    fn ui(&mut self, f: &mut Frame) {
        let size = f.size();

        // Main layout with menu bar and status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Menu bar
                Constraint::Min(0),    // Main content
                Constraint::Length(1), // Status bar
            ])
            .split(size);

        // Menu bar
        self.menu_renderer.render_menu_bar(f, chunks[0]);

        // Main content
        match self.current_screen {
            Screen::Main => self.render_main_screen(f, chunks[1]),
            Screen::Browse => self.render_browse_screen(f, chunks[1]),
            Screen::Subscriptions => self.render_subscriptions_screen(f, chunks[1]),
            Screen::Help => self.render_help_screen(f, chunks[1]),
        }

        // Status bar
        self.statusbar_renderer.render_status_bar(f, chunks[2], self.menu_renderer.get_active_menu().as_ref());

        // Dialogs
        if self.show_connect_dialog {
            self.render_connect_dialog(f, size);
        }
        if self.show_security_dialog {
            self.render_security_dialog(f, size);
        }
        if self.show_node_properties {
            self.render_node_properties_dialog(f, size);
        }
        if self.show_write_dialog {
            self.render_write_dialog(f, size);
        }        if self.show_server_info_dialog {
            self.render_server_info_dialog(f, size);
        }
          // Render dropdown menus (must be last to appear on top)
        if let Some(menu_type) = self.menu_renderer.get_active_menu() {
            self.menu_renderer.render_dropdown_menu(f, menu_type);
        }    }

    fn render_main_screen(&self, f: &mut Frame, area: Rect) {
        if self.show_connect_in_main && self.connection_status == ConnectionStatus::Disconnected {
            self.render_main_connect_screen(f, area);
        } else {
            self.render_main_status_screen(f, area);
        }
    }

    fn render_main_status_screen(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Connection info
                Constraint::Min(0),     // Options
            ])
            .split(area);

        // Connection info box
        let connection_info = vec![
            format!("Server URL: {}", self.server_url),
            format!("Status: {:?}", self.connection_status),
            format!("Security: {:?}", self.security_policy),
            "".to_string(),
            "Available Options:".to_string(),
            "1 - Browse OPC Server".to_string(),
            "2 - View Subscriptions".to_string(),
            "3 - Help".to_string(),
        ];

        let connection_block = Paragraph::new(connection_info.join("\n"))
            .block(Block::default()
                .title("OPC UA Client")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)))
            .wrap(Wrap { trim: true });
        f.render_widget(connection_block, chunks[0]);

        // Quick actions
        let actions = vec![
            "Hot Keys:",
            "Alt+F - Connect to Server",
            "C - Connect to Server",
            "D - Disconnect from Server",
            "Q - Quit Application",
            "",
            "Press number keys to select options above",
        ];

        let actions_block = Paragraph::new(actions.join("\n"))
            .block(Block::default()
                .title("Quick Actions")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)));
        f.render_widget(actions_block, chunks[1]);
    }

    fn render_main_connect_screen(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),  // Connect form
                Constraint::Percentage(30),  // Connection logs
            ])
            .split(area);

        // Connect form on the left
        match self.connect_dialog_step {
            ConnectDialogStep::ServerUrl => self.render_main_server_url_step(f, chunks[0]),
            ConnectDialogStep::EndpointSelection => self.render_main_endpoint_step(f, chunks[0]),
            ConnectDialogStep::Authentication => self.render_main_auth_step(f, chunks[0]),
        }

        // Connection logs on the right
        let log_text = self.connection_logs.join("\n");
        let logs_block = Paragraph::new(log_text)
            .block(Block::default()
                .title("Connection Log")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)))
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });
        f.render_widget(logs_block, chunks[1]);
    }    fn render_main_server_url_step(&self, f: &mut Frame, area: Rect) {
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
        f.render_widget(input_block, chunks[1]);

        // Buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[2]);

        let back_button = Paragraph::new("[ Back ]")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(back_button, button_chunks[0]);

        let discover_button = if self.connect_in_progress {
            Paragraph::new("[ Discovering... ]")
                .style(Style::default().fg(Color::Yellow))
        } else {
            Paragraph::new("[ Discover Endpoints ]")
                .style(Style::default().fg(Color::Green))
        };
        let discover_button = discover_button
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(discover_button, button_chunks[1]);

        let next_button = Paragraph::new("[ Next ]")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(next_button, button_chunks[2]);

        // Help text
        let help_text = if self.connect_in_progress {
            "Please wait while discovering server endpoints..."
        } else {
            "Enter server URL above, then click 'Discover Endpoints' or press Enter\nEsc - Cancel | Enter - Discover | Tab - Navigate buttons"
        };
        
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[3]);
    }

    fn render_main_endpoint_step(&self, f: &mut Frame, area: Rect) {
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
            ])
            .split(chunks[2]);

        let back_button = Paragraph::new("[ Back ]")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(back_button, button_chunks[0]);

        let select_button = Paragraph::new("[ Select Endpoint ]")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(select_button, button_chunks[1]);

        let next_button = Paragraph::new("[ Next ]")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(next_button, button_chunks[2]);

        // Help text
        let help = Paragraph::new("↑↓ - Navigate endpoints | Enter - Select | Tab - Navigate buttons")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[3]);
    }

    fn render_main_auth_step(&self, f: &mut Frame, area: Rect) {
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
            },
        ];

        let auth_list = Paragraph::new(auth_items.join("\n"))
            .block(Block::default()
                .title("Authentication Method")
                .borders(Borders::ALL));
        f.render_widget(auth_list, chunks[1]);

        // User details (if UserPassword is selected)
        if self.authentication_type == AuthenticationType::UserPassword {
            let user_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Username
                    Constraint::Length(3),  // Password
                ])
                .split(chunks[2]);

            let username_style = if self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow).bg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };

            let username_input = Paragraph::new(self.username.as_str())
                .block(Block::default()
                    .title("Username")
                    .borders(Borders::ALL))
                .style(username_style);
            f.render_widget(username_input, user_chunks[0]);

            let password_display = "*".repeat(self.password.len());
            let password_input = Paragraph::new(password_display)
                .block(Block::default()
                    .title("Password")
                    .borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            f.render_widget(password_input, user_chunks[1]);
        }

        // Buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(chunks[3]);

        let back_button = Paragraph::new("[ Back ]")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(back_button, button_chunks[0]);

        let connect_button = Paragraph::new("[ Connect ]")
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(connect_button, button_chunks[1]);

        let cancel_button = Paragraph::new("[ Cancel ]")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(cancel_button, button_chunks[2]);

        // Help text
        let help_text = match self.authentication_type {
            AuthenticationType::Anonymous => {
                "↑↓ - Select authentication | Enter - Connect | Tab - Navigate buttons"
            }
            AuthenticationType::UserPassword => {
                "↑↓ - Select auth | Tab - Switch fields | Enter - Connect"
            }
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[4]);
    }
    
    // Missing methods that need to be implemented
    
    fn add_connection_log(&mut self, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.connection_logs.push(format!("[{}] {}", timestamp, message));
        // Keep only last 10 log entries
        if self.connection_logs.len() > 10 {
            self.connection_logs.remove(0);
        }
    }

    fn open_connect_dialog(&mut self) {
        // Use main screen connect mode when disconnected
        if self.connection_status == ConnectionStatus::Disconnected {
            self.show_connect_in_main = true;
            self.current_screen = Screen::Main;
        } else {
            self.show_connect_dialog = true;
        }
        
        self.connect_dialog_step = ConnectDialogStep::ServerUrl;
        self.discovered_endpoints.clear();
        self.selected_endpoint_index = 0;
        self.authentication_type = AuthenticationType::Anonymous;
        self.username.clear();
        self.password.clear();
        self.connect_in_progress = false;
        self.input_mode = InputMode::Editing;
        self.cursor_position = self.server_url.len();
        self.connection_logs.clear();
        self.add_connection_log("Starting connection process...");
    }    fn close_connect_dialog(&mut self) {
        self.show_connect_dialog = false;
        self.show_connect_in_main = false;
        self.connect_dialog_step = ConnectDialogStep::ServerUrl;
        self.discovered_endpoints.clear();
        self.selected_endpoint_index = 0;
        self.authentication_type = AuthenticationType::Anonymous;
        self.active_auth_field = AuthenticationField::Username;
        self.username.clear();
        self.password.clear();
        self.connect_in_progress = false;
        self.input_mode = InputMode::Normal;
        self.current_screen = Screen::Main;  // Ensure we stay on main screen
    }

    async fn handle_connect_dialog_input(&mut self, key: KeyCode) -> Result<()> {
        match self.connect_dialog_step {
            ConnectDialogStep::ServerUrl => {
                match key {
                    KeyCode::Enter | KeyCode::Tab => {
                        // Discover endpoints
                        self.discover_endpoints().await?;
                    }
                    KeyCode::Esc => {
                        self.close_connect_dialog();
                    }                    KeyCode::Char(c) => {
                        if self.input_mode == InputMode::Editing {
                            self.server_url.insert(self.cursor_position, c);
                            self.cursor_position += 1;
                        }
                    }
                    KeyCode::Backspace => {
                        if self.input_mode == InputMode::Editing && self.cursor_position > 0 {
                            self.cursor_position -= 1;
                            self.server_url.remove(self.cursor_position);
                        }
                    }
                    KeyCode::Left => {
                        if self.input_mode == InputMode::Editing && self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.input_mode == InputMode::Editing && self.cursor_position < self.server_url.len() {
                            self.cursor_position += 1;
                        }
                    }
                    _ => {}
                }
            }
            ConnectDialogStep::EndpointSelection => {
                match key {
                    KeyCode::Up => {
                        if self.selected_endpoint_index > 0 {
                            self.selected_endpoint_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected_endpoint_index < self.discovered_endpoints.len().saturating_sub(1) {
                            self.selected_endpoint_index += 1;
                        }
                    }                    KeyCode::Enter | KeyCode::Tab => {
                        // Move to authentication step
                        self.connect_dialog_step = ConnectDialogStep::Authentication;
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.cursor_position = self.username.len();
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                    }
                    KeyCode::Esc => {
                        // Go back to URL step
                        self.connect_dialog_step = ConnectDialogStep::ServerUrl;
                        self.cursor_position = self.server_url.len();
                    }
                    _ => {}
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
                            self.cursor_position = self.username.len();
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                    }KeyCode::Tab => {
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
                    }
                    KeyCode::Enter => {
                        // Connect with selected settings
                        self.connect_with_settings().await?;
                    }
                    KeyCode::Esc => {
                        // Go back to endpoint selection
                        self.connect_dialog_step = ConnectDialogStep::EndpointSelection;
                        self.input_mode = InputMode::Normal;
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
                    }
                    KeyCode::Left => {
                        if self.authentication_type == AuthenticationType::UserPassword && self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
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
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn discover_endpoints(&mut self) -> Result<()> {
        self.connect_in_progress = true;
        self.add_connection_log("Discovering endpoints...");
        
        // Simulate endpoint discovery (in real implementation, this would call OPC UA discovery)
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Mock discovered endpoints based on URL
        self.discovered_endpoints = vec![
            EndpointInfo {
                endpoint_url: self.server_url.clone(),
                security_policy: SecurityPolicy::None,
                security_mode: SecurityMode::None,
                display_name: "No Security".to_string(),
            },
            EndpointInfo {
                endpoint_url: self.server_url.clone(),
                security_policy: SecurityPolicy::Basic256Sha256,
                security_mode: SecurityMode::Sign,
                display_name: "Basic256Sha256 with Signing".to_string(),
            },
            EndpointInfo {
                endpoint_url: self.server_url.clone(),
                security_policy: SecurityPolicy::Basic256Sha256,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Basic256Sha256 with Sign & Encrypt".to_string(),
            },
        ];
        
        self.connect_in_progress = false;
        self.connect_dialog_step = ConnectDialogStep::EndpointSelection;
        self.input_mode = InputMode::Normal;  // Switch to normal mode for endpoint selection
        self.add_connection_log(&format!("Found {} endpoints", self.discovered_endpoints.len()));
        
        Ok(())
    }

    async fn connect_with_settings(&mut self) -> Result<()> {
        if let Some(endpoint) = self.discovered_endpoints.get(self.selected_endpoint_index) {
            self.security_policy = endpoint.security_policy.clone();
            
            self.add_connection_log(&format!(
                "Connecting with {} authentication...", 
                match self.authentication_type {
                    AuthenticationType::Anonymous => "anonymous",
                    AuthenticationType::UserPassword => "username/password",
                }
            ));
            
            // Simulate connection (in real implementation, this would establish OPC UA session)
            tokio::time::sleep(Duration::from_millis(1000)).await;
            
            // Update client manager with connection details
            let connection_result = {
                let mut client = self.client_manager.lock().await;
                client.connect(&self.server_url, &self.security_policy).await
            };
            
            match connection_result {
                Ok(_) => {
                    self.add_connection_log("Connected successfully");
                    let current_node_id = self.current_node_id.clone();
                    self.close_connect_dialog();
                    // Start browsing from root
                    self.browse_node(&current_node_id).await?;
                }
                Err(e) => {
                    self.add_connection_log(&format!("Connection failed: {}", e));
                }
            }
        }
        
        Ok(())
    }

    // Add the remaining missing methods as simple implementations for now
    async fn connect_to_server(&mut self) -> Result<()> {
        let mut client = self.client_manager.lock().await;
        client.connect(&self.server_url, &self.security_policy).await
    }

    async fn disconnect_from_server(&mut self) -> Result<()> {
        let mut client = self.client_manager.lock().await;
        client.disconnect().await
    }

    async fn browse_node(&mut self, node_id: &str) -> Result<()> {
        let mut client = self.client_manager.lock().await;
        self.browse_items = client.browse_node(node_id).await?;
        Ok(())
    }

    async fn browse_parent(&mut self) -> Result<()> {
        // Simple implementation - go to root
        self.current_node_id = "ns=0;i=85".to_string();
        self.browse_node(&self.current_node_id.clone()).await
    }

    async fn add_to_subscription(&mut self, _item: BrowseItem) -> Result<()> {
        // Simple mock implementation
        Ok(())
    }

    async fn remove_from_subscription(&mut self, _index: usize) -> Result<()> {
        // Simple mock implementation
        Ok(())
    }

    async fn write_node_value(&mut self) -> Result<()> {
        // Simple mock implementation
        Ok(())
    }

    // Add remaining render methods
    fn render_browse_screen(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Browse tree
        let items: Vec<ListItem> = self.browse_items
            .iter()
            .map(|item| {
                let icon = if item.is_folder { "📁" } else { "📄" };
                let value_str = item.value.as_deref().unwrap_or("");
                ListItem::new(format!("{} {} ({})", icon, item.display_name, value_str))
            })
            .collect();

        let browse_list = List::new(items)
            .block(Block::default()
                .title(format!("Browse: {}", self.current_node_id))
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");

        f.render_stateful_widget(browse_list, chunks[0], &mut self.browse_state);

        // Actions panel
        let actions = vec![
            "Actions:",
            "",
            "↑↓ - Navigate",
            "Enter - Browse folder",
            "A - Add to subscription",
            "P - View properties",
            "W - Write value",
            "B - Go back/parent",
            "Esc - Main menu",
        ];

        let actions_block = Paragraph::new(actions.join("\n"))
            .block(Block::default()
                .title("Actions")
                .borders(Borders::ALL));
        f.render_widget(actions_block, chunks[1]);
    }

    fn render_subscriptions_screen(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Subscription items
        let items: Vec<ListItem> = self.subscriptions
            .iter()
            .map(|item| {
                ListItem::new(format!("{}: {} ({})", 
                    item.display_name, 
                    item.value.as_deref().unwrap_or("N/A"),
                    item.timestamp.as_deref().unwrap_or("N/A")))
            })
            .collect();

        let subscription_list = List::new(items)
            .block(Block::default()
                .title("Active Subscriptions")
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");

        f.render_stateful_widget(subscription_list, chunks[0], &mut self.subscription_state);

        // Actions panel
        let total_items = format!("Total items: {}", self.subscriptions.len());
        let actions = vec![
            "Actions:",
            "",
            "↑↓ - Navigate",
            "D/Del - Remove subscription",
            "Esc - Main menu",
            "",
            &total_items,
        ];

        let actions_block = Paragraph::new(actions.join("\n"))
            .block(Block::default()
                .title("Actions")
                .borders(Borders::ALL));
        f.render_widget(actions_block, chunks[1]);
    }

    fn render_help_screen(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            "OPC UA Client - Help",
            "",
            "GLOBAL HOTKEYS:",
            "Alt+F - File menu / Connect",
            "Alt+H - Help screen",
            "Alt+X - Exit application",
            "",
            "MAIN SCREEN:",
            "1 - Browse OPC Server",
            "2 - View Subscriptions", 
            "3 - Help",
            "C - Connect to server",
            "D - Disconnect from server",
            "Q - Quit",
            "",
            "BROWSE SCREEN:",
            "↑↓ - Navigate nodes",
            "Enter - Browse into folder",
            "A - Add node to subscription",
            "P - View node properties",
            "W - Write value to node",
            "B - Go back to parent node",
            "",
            "SUBSCRIPTION SCREEN:",
            "↑↓ - Navigate subscriptions",
            "D/Del - Remove subscription",
            "",
            "Press Esc to return to main screen from any screen.",
        ];

        let help_block = Paragraph::new(help_text.join("\n"))
            .block(Block::default()
                .title("Help")
                .borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(help_block, area);
    }    fn render_connect_dialog(&self, f: &mut Frame, area: Rect) {
        match self.connect_dialog_step {
            ConnectDialogStep::ServerUrl => self.render_server_url_step(f, area),
            ConnectDialogStep::EndpointSelection => self.render_endpoint_selection_step(f, area),
            ConnectDialogStep::Authentication => self.render_authentication_step(f, area),
        }
    }

    fn render_server_url_step(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(70, 25, area);
        f.render_widget(Clear, popup_area);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // URL input
                Constraint::Length(3),  // Connect button
                Constraint::Min(0),     // Help text
            ])
            .split(popup_area.inner(&Margin { vertical: 1, horizontal: 1 }));

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 1/3")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        // URL input
        let input_style = if self.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow).bg(Color::Blue)
        } else {
            Style::default().fg(Color::White)
        };
        
        let input_block = Paragraph::new(self.server_url.as_str())
            .block(Block::default()
                .title("Server URL")
                .borders(Borders::ALL))
            .style(input_style);
        f.render_widget(input_block, chunks[1]);

        // Connect button
        let button_text = if self.connect_in_progress {
            "Discovering Endpoints..."
        } else {
            "[ Discover Endpoints ]"
        };
        
        let button = Paragraph::new(button_text)
            .style(Style::default().fg(Color::Green).bg(Color::Black))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(button, chunks[2]);

        // Help text
        let help_text = if self.connect_in_progress {
            "Please wait while discovering server endpoints..."
        } else {
            "Enter server URL and press Tab to discover endpoints\nEnter - Discover | Esc - Cancel | Tab - Next"
        };
        
        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });
        f.render_widget(help, chunks[3]);

        let block = Block::default()
            .title("Connect to Server")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        f.render_widget(block, popup_area);
    }

    fn render_endpoint_selection_step(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(80, 30, area);
        f.render_widget(Clear, popup_area);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Endpoint list
                Constraint::Length(3),  // Help text
            ])
            .split(popup_area.inner(&Margin { vertical: 1, horizontal: 1 }));

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 2/3: Select Endpoint")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
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

        // Help text
        let help = Paragraph::new("↑↓ - Navigate endpoints | Enter - Select | Esc - Back | Tab - Skip to Auth")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[2]);

        let block = Block::default()
            .title("Select Security Endpoint")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        f.render_widget(block, popup_area);
    }    fn render_authentication_step(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(70, 25, area);
        f.render_widget(Clear, popup_area);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(5),  // Auth type selection
                Constraint::Length(6),  // User details (if needed)
                Constraint::Min(0),     // Help text
            ])
            .split(popup_area.inner(&Margin { vertical: 1, horizontal: 1 }));

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 3/3: Authentication")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
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
            },
        ];

        let auth_list = Paragraph::new(auth_items.join("\n"))
            .block(Block::default()
                .title("Authentication Method")
                .borders(Borders::ALL));
        f.render_widget(auth_list, chunks[1]);

        // User details (if UserPassword is selected)
        if self.authentication_type == AuthenticationType::UserPassword {
            let user_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Username
                    Constraint::Length(3),  // Password
                ])
                .split(chunks[2]);

            // Username field
            let username_is_active = self.active_auth_field == AuthenticationField::Username;
            let username_style = if self.input_mode == InputMode::Editing && username_is_active {
                Style::default().fg(Color::Yellow).bg(Color::Blue)
            } else if username_is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let username_text = if username_is_active && self.input_mode == InputMode::Editing {
                self.format_input_with_cursor(&self.username, true)
            } else {
                self.username.clone()
            };

            let username_input = Paragraph::new(username_text)
                .block(Block::default()
                    .title("Username")
                    .borders(Borders::ALL))
                .style(username_style);
            f.render_widget(username_input, user_chunks[0]);

            // Password field
            let password_is_active = self.active_auth_field == AuthenticationField::Password;
            let password_style = if self.input_mode == InputMode::Editing && password_is_active {
                Style::default().fg(Color::Yellow).bg(Color::Blue)
            } else if password_is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let password_display = if password_is_active && self.input_mode == InputMode::Editing {
                let masked = "*".repeat(self.password.len());
                self.format_input_with_cursor(&masked, true)
            } else {
                "*".repeat(self.password.len())
            };

            let password_input = Paragraph::new(password_display)
                .block(Block::default()
                    .title("Password")
                    .borders(Borders::ALL))
                .style(password_style);
            f.render_widget(password_input, user_chunks[1]);
        }

        // Help text
        let help_text = match self.authentication_type {
            AuthenticationType::Anonymous => {
                "↑↓ - Select authentication | Enter - Connect | Esc - Back"
            }
            AuthenticationType::UserPassword => {
                "↑↓ - Select auth | Tab - Switch fields | Enter - Connect | Esc - Back"
            }
        };

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[3]);

        let block = Block::default()
            .title("Authentication")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        f.render_widget(block, popup_area);
    }

    fn render_security_dialog(&self, f: &mut Frame, area: Rect) {
        // Implementation for security policy selection dialog
        let popup_area = self.centered_rect(50, 15, area);
        f.render_widget(Clear, popup_area);
        
        let block = Block::default()
            .title("Security Policy")
            .borders(Borders::ALL);
        f.render_widget(block, popup_area);
    }

    fn render_node_properties_dialog(&self, f: &mut Frame, area: Rect) {
        if let Some(node) = &self.selected_node {
            let popup_area = self.centered_rect(70, 25, area);
            f.render_widget(Clear, popup_area);
            
            let properties = vec![
                format!("Node ID: {}", node.node_id),
                format!("Display Name: {}", node.display_name),
                format!("Node Class: {}", node.node_class),
                format!("Data Type: {}", node.data_type.as_deref().unwrap_or("N/A")),
                format!("Value: {}", node.value.as_deref().unwrap_or("N/A")),
                format!("Is Folder: {}", node.is_folder),
            ];

            let props_block = Paragraph::new(properties.join("\n"))
                .block(Block::default()
                    .title("Node Properties")
                    .borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(props_block, popup_area);
        }
    }

    fn render_write_dialog(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(60, 15, area);
        f.render_widget(Clear, popup_area);
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(popup_area.inner(&Margin { vertical: 1, horizontal: 1 }));

        if let Some(node) = &self.selected_node {
            let title = Paragraph::new(format!("Write Value to: {}", node.display_name))
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            f.render_widget(title, chunks[0]);
        }

        let input_block = Paragraph::new(self.write_value.as_str())
            .block(Block::default()
                .title("New Value")
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(input_block, chunks[1]);

        let help = Paragraph::new("Enter - Write | Esc - Cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[2]);

        let block = Block::default()
            .title("Write Value")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        f.render_widget(block, popup_area);
    }

    fn render_server_info_dialog(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(70, 30, area);
        f.render_widget(Clear, popup_area);
        
        let server_info = if self.connection_status == ConnectionStatus::Connected {
            let server_url = format!("Server URL: {}", self.server_url);
            let connection_status = format!("Connection Status: {:?}", self.connection_status);
            let security_policy = format!("Security Policy: {:?}", self.security_policy);
            let session_id = format!("Session ID: {}", "12345-abcde-67890");
            let session_timeout = format!("Session Timeout: {} ms", 60000);
            
            vec![
                "Server Information".to_string(),
                "─────────────────────────────────────────────".to_string(),
                server_url,
                connection_status,
                security_policy,
                "".to_string(),
                "Server Details:".to_string(),
                "Application Name: Demo OPC Server".to_string(),
                "Application Type: Server".to_string(),
                "Product URI: urn:demo-server".to_string(),
                "Server State: Running".to_string(),
                "".to_string(),
                "Endpoints:".to_string(),
                "• opc.tcp://localhost:4840 (None)".to_string(),
                "• opc.tcp://localhost:4840 (Basic256Sha256)".to_string(),
                "• opc.tcp://localhost:4840 (Basic256Sha256)".to_string(),
                "".to_string(),
                "Namespaces:".to_string(),
                "• 0: http://opcfoundation.org/UA/".to_string(),
                "• 1: urn:demo-server".to_string(),
                "".to_string(),
                "Session Info:".to_string(),
                session_id,
                session_timeout,
            ]
        } else {
            vec![
                "Server Information".to_string(),
                "─────────────────────────────────────────────".to_string(),
                "Not connected to any server".to_string(),
                "".to_string(),
                "To connect:".to_string(),
                "1. Use File > Connect... menu".to_string(),
                "2. Enter server URL".to_string(),
                "3. Select security policy".to_string(),
                "4. Click Connect".to_string(),
            ]
        };

        let info_text = server_info.join("\n");
        let info_block = Paragraph::new(info_text)
            .block(Block::default()
                .title("Server Information")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .wrap(Wrap { trim: true });
        f.render_widget(info_block, popup_area);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn format_input_with_cursor(&self, text: &str, show_cursor: bool) -> String {
        if show_cursor && self.input_mode == InputMode::Editing {
            let mut result = text.to_string();
            if self.cursor_position <= result.len() {
                result.insert(self.cursor_position, '|');
            } else {
                result.push('|');
            }
            result
        } else {
            text.to_string()
        }
    }
}
