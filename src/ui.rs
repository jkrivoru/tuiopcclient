use anyhow::Result;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame, Terminal,
};
use std::{
    io::{self, Stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

use crate::client::{ConnectionStatus, OpcUaClientManager};
use crate::screens::{BrowseScreen, ConnectScreen};
use crate::screens::connect::ConnectDialogStep;

pub struct App {
    client_manager: Arc<RwLock<OpcUaClientManager>>,
    should_quit: bool,
    test_mode: bool,

    // App state
    app_state: AppState,

    // Screens
    connect_screen: ConnectScreen,
    browse_screen: Option<BrowseScreen>,
}

#[derive(Debug, Clone)]
enum AppState {
    Connecting,
    Connected(String), // Store server URL
}

impl App {
    pub fn new(client_manager: Arc<RwLock<OpcUaClientManager>>) -> Self {
        Self {
            client_manager,
            should_quit: false,
            test_mode: false,
            app_state: AppState::Connecting,
            connect_screen: ConnectScreen::new(),
            browse_screen: None,
        }
    }

    pub fn new_with_browse_test(client_manager: Arc<RwLock<OpcUaClientManager>>) -> Self {
        let test_server_url = "opc.tcp://test-server:4840".to_string();
        Self {
            client_manager: client_manager.clone(),
            should_quit: false,
            test_mode: true,
            app_state: AppState::Connected(test_server_url.clone()),
            connect_screen: ConnectScreen::new(),
            browse_screen: Some(BrowseScreen::new(test_server_url, client_manager)),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        execute!(terminal.backend_mut(), crossterm::event::EnableMouseCapture)?;

        let result = self.run_app_loop(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            crossterm::event::DisableMouseCapture
        )?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        result
    }

    async fn run_app_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.render_ui(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {                match event::read()? {
                    Event::Key(key) => {
                        // Only process key press events, not key release
                        if key.kind == KeyEventKind::Press {
                            self.handle_key_input(key.code, key.modifiers).await?;
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_input(mouse, terminal).await?;
                    }
                    _ => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.handle_tick().await;
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }        Ok(())
    }    async fn handle_key_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match &self.app_state {
            AppState::Connecting => {
                // Handle connect screen input
                if let Some(connection_result) =
                    self.connect_screen.handle_input(key, modifiers).await?
                {
                    self.handle_connection_result(connection_result).await;
                }
            }
            AppState::Connected(_server_url) => {
                // Handle browse screen input
                if let Some(browse_screen) = &mut self.browse_screen {
                    if let Some(connection_result) =
                        browse_screen.handle_input(key, modifiers).await?
                    {
                        match connection_result {
                            ConnectionStatus::Disconnected => {
                                // User wants to quit
                                self.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }            }
        }
        Ok(())
    }

    async fn handle_mouse_input(&mut self, mouse: MouseEvent, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Ignore mouse move events to prevent spam
        if let MouseEventKind::Moved = mouse.kind {
            return Ok(());
        }

        match &self.app_state {
            AppState::Connecting => {
                // Handle connect screen mouse events
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.connect_screen
                            .handle_mouse_down(mouse.column, mouse.row);
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        // First check for button clicks
                        if let Some(button_id) =
                            self.connect_screen.handle_mouse_up(mouse.column, mouse.row)
                        {
                            // Handle button action
                            if let Some(connection_result) =
                                self.connect_screen.handle_button_action(&button_id).await?
                            {
                            self.handle_connection_result(connection_result).await;
                            }                        } else {
                            // If not a button, handle other mouse clicks (endpoints, fields, etc.)
                            let size = terminal.size()?;
                            let rect = ratatui::layout::Rect {
                                x: 0,
                                y: 0,
                                width: size.width,
                                height: size.height,
                            };
                            self.connect_screen.handle_mouse_click(mouse.column, mouse.row, rect);
                        }
                    }
                    _ => {}
                }
            }            AppState::Connected(_) => {
                // Handle browse screen mouse events
                if let Some(browse_screen) = &mut self.browse_screen {
                    let size = terminal.size()?;
                    let full_area = ratatui::layout::Rect {
                        x: 0,
                        y: 0,
                        width: size.width,
                        height: size.height,
                    };
                    
                    // Calculate the tree area (70% of the main content area)
                    let main_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(0),    // Main content area
                            Constraint::Length(1), // Status bar
                        ])
                        .split(full_area);

                    let content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(70), // Tree view
                            Constraint::Percentage(30), // Attributes panel
                        ])
                        .split(main_chunks[0]);
                    
                    // Tree area with borders - inner area for actual content
                    let tree_area = Rect {
                        x: content_chunks[0].x + 1,  // Account for left border
                        y: content_chunks[0].y + 1,  // Account for top border  
                        width: content_chunks[0].width.saturating_sub(2), // Account for both borders
                        height: content_chunks[0].height.saturating_sub(2), // Account for both borders
                    };
                    
                    if let Some(connection_result) = 
                        browse_screen.handle_mouse_input(mouse, tree_area).await? {
                        match connection_result {
                            ConnectionStatus::Disconnected => {
                                self.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_tick(&mut self) {
        match &self.app_state {
            AppState::Connecting => {
                // Handle pending operations for connect screen
                match self.connect_screen.handle_pending_operations().await {
                    Ok(Some(connection_result)) => {
                        // Handle connection result using helper
                        self.handle_connection_result(connection_result).await;
                    }
                    Ok(None) => {
                        // No change, continue as normal
                    }
                    Err(e) => {
                        log::error!("Error handling connect screen operations: {}", e);
                    }
                }
            }            AppState::Connected(_) => {
                // Update connection status from client manager
                if let Ok(client) = self.client_manager.try_read() {
                    let status = client.get_connection_status();
                    if status == ConnectionStatus::Disconnected {                        // Connection was lost, go back to connect screen
                        log::warn!("Lost connection to server, returning to connect screen");
                        self.app_state = AppState::Connecting;
                        self.browse_screen = None;
                        self.connect_screen.async_reset().await;
                    }
                }
            }
        }
    }    /// Helper method to handle connection results consistently
    async fn handle_connection_result(&mut self, connection_result: ConnectionStatus) {
        match connection_result {
            ConnectionStatus::Connecting => {
                // Get the server URL from the connect screen and attempt the actual connection
                let server_url = self.connect_screen.get_server_url();                log::info!("Attempting real connection to: {}", server_url);

                // Actually connect the client manager to the server
                let connection_result = {
                    let mut client_guard = self.client_manager.write().await;
                    client_guard.connect(&server_url).await
                };
                
                match connection_result {
                    Ok(()) => {
                        log::info!("Client manager successfully connected to: {}", server_url);
                        
                        // Update client manager status (ensure write lock is released quickly)
                        {
                            if let Ok(mut client) = self.client_manager.try_write() {
                                client.set_connection_status(ConnectionStatus::Connected);
                            }
                        }                        // Transition to browse screen
                        self.app_state = AppState::Connected(server_url.clone());
                        let mut browse_screen = BrowseScreen::new(server_url.clone(), self.client_manager.clone());
                        
                        // Load real tree data asynchronously
                        if let Err(e) = browse_screen.load_real_tree().await {
                            log::error!("Failed to load real tree data: {}. Using demo data.", e);
                        }
                        
                        self.browse_screen = Some(browse_screen);                    }                    Err(e) => {
                        log::error!("Failed to connect client manager: {}", e);
                        self.connect_screen.clear_connection().await;
                        // Set client manager to error state
                        if let Ok(mut client) = self.client_manager.try_write() {
                            client.set_connection_status(ConnectionStatus::Error(format!("Connection failed: {}", e)));
                        }
                    }
                }
            }            ConnectionStatus::Connected => {
                // This shouldn't happen anymore since perform_connection returns Connecting
                log::warn!("Received Connected status directly - this should not happen");
                let server_url = self.connect_screen.get_server_url();
                self.app_state = AppState::Connected(server_url.clone());
                let mut browse_screen = BrowseScreen::new(server_url, self.client_manager.clone());
                
                // Load real tree data asynchronously
                if let Err(e) = browse_screen.load_real_tree().await {
                    log::error!("Failed to load real tree data: {}. Using demo data.", e);
                }
                
                self.browse_screen = Some(browse_screen);
            }
            ConnectionStatus::Disconnected => {
                // User cancelled connection or wants to quit
                self.should_quit = true;
            }            ConnectionStatus::Error(error) => {
                // Connection failed, log error but stay on connect screen at current step
                log::error!("Connection failed: {}", error);
                self.connect_screen.clear_connection().await;
                // Set client manager to error state
                if let Ok(mut client) = self.client_manager.try_write() {
                    client.set_connection_status(ConnectionStatus::Error(error));
                }}
        }
    }    fn render_ui(&mut self, f: &mut Frame) {
        let size = f.size();

        match &self.app_state {            AppState::Connecting => {
                // Show connect screen with help line and status bar at bottom
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),    // Connect screen
                        Constraint::Length(1), // Help line
                        Constraint::Length(1), // Status bar at bottom
                    ])
                    .split(size);

                self.connect_screen.render(f, chunks[0]);
                self.connect_screen.render_help_line(f, chunks[1]);
                self.render_connection_status_bar(f, chunks[2]);
            }
            AppState::Connected(_) => {
                // Show browse screen with help line
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),    // Browse screen
                    ])
                    .split(size);

                if let Some(browse_screen) = &mut self.browse_screen {
                    browse_screen.render(f, chunks[0]);
                }
            }
        }
    }

    fn render_connection_status_bar(&mut self, f: &mut Frame, area: Rect) {        let status_text = match self.connect_screen.step {
            ConnectDialogStep::ServerUrl => {
                // Show placeholder on first step
                "Enter valid OPC UA server URL".to_string()
            }ConnectDialogStep::EndpointSelection => {
                // Show the server URL that will be used
                let url = if self.connect_screen.use_original_url {
                    self.connect_screen.server_url_input.value()
                } else {
                    // Show the URL from selected endpoint if available
                    if let Some(endpoint) = self.connect_screen.get_selected_endpoint() {
                        endpoint.original_endpoint.endpoint_url.as_ref()
                    } else {
                        self.connect_screen.server_url_input.value()
                    }
                };
                format!("Server: {}", url)
            }            ConnectDialogStep::SecurityConfiguration => {
                // Show server URL and selected endpoint info
                let url = if self.connect_screen.use_original_url {
                    self.connect_screen.server_url_input.value()
                } else {
                    if let Some(endpoint) = self.connect_screen.get_selected_endpoint() {
                        endpoint.original_endpoint.endpoint_url.as_ref()
                    } else {
                        self.connect_screen.server_url_input.value()
                    }
                };
                
                let endpoint_info = if let Some(endpoint) = self.connect_screen.get_selected_endpoint() {
                    format!(" | Endpoint: [{}, {}]", 
                        match &endpoint.security_policy {
                            crate::screens::connect::SecurityPolicy::None => "None",
                            crate::screens::connect::SecurityPolicy::Basic128Rsa15 => "Basic128Rsa15",
                            crate::screens::connect::SecurityPolicy::Basic256 => "Basic256",
                            crate::screens::connect::SecurityPolicy::Basic256Sha256 => "Basic256Sha256",
                            crate::screens::connect::SecurityPolicy::Aes128Sha256RsaOaep => "Aes128Sha256RsaOaep",
                            crate::screens::connect::SecurityPolicy::Aes256Sha256RsaPss => "Aes256Sha256RsaPss",
                        },
                        format!("{:?}", endpoint.security_mode)
                    )
                } else {
                    " | Endpoint: [None, None]".to_string()
                };
                
                format!("Server: {}{}", url, endpoint_info)
            }            ConnectDialogStep::Authentication => {
                // Show server URL and endpoint info
                let url = if self.connect_screen.use_original_url {
                    self.connect_screen.server_url_input.value()
                } else {
                    if let Some(endpoint) = self.connect_screen.get_selected_endpoint() {
                        endpoint.original_endpoint.endpoint_url.as_ref()
                    } else {
                        self.connect_screen.server_url_input.value()
                    }
                };
                
                let endpoint_info = if let Some(endpoint) = self.connect_screen.get_selected_endpoint() {
                    format!(" | Endpoint: [{}, {}]", 
                        match &endpoint.security_policy {
                            crate::screens::connect::SecurityPolicy::None => "None",
                            crate::screens::connect::SecurityPolicy::Basic128Rsa15 => "Basic128Rsa15",
                            crate::screens::connect::SecurityPolicy::Basic256 => "Basic256",
                            crate::screens::connect::SecurityPolicy::Basic256Sha256 => "Basic256Sha256",
                            crate::screens::connect::SecurityPolicy::Aes128Sha256RsaOaep => "Aes128Sha256RsaOaep",
                            crate::screens::connect::SecurityPolicy::Aes256Sha256RsaPss => "Aes256Sha256RsaPss",
                        },
                        format!("{:?}", endpoint.security_mode)
                    )
                } else {
                    " | Endpoint: [None, None]".to_string()
                };
                
                format!("Server: {}{}", url, endpoint_info)
            }
        };        // Always show the status bar
        let status_bar = Paragraph::new(status_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue));
        f.render_widget(status_bar, area);
    }
}
