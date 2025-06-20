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
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use std::{
    io::{self, Stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::client::{ConnectionStatus, OpcUaClientManager};
use crate::screens::{BrowseScreen, ConnectScreen};

pub struct App {
    client_manager: Arc<Mutex<OpcUaClientManager>>,
    should_quit: bool,

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
    pub fn new(client_manager: Arc<Mutex<OpcUaClientManager>>) -> Self {
        Self {
            client_manager,
            should_quit: false,
            app_state: AppState::Connecting,
            connect_screen: ConnectScreen::new(),
            browse_screen: None,
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

            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        // Only process key press events, not key release
                        if key.kind == KeyEventKind::Press {
                            self.handle_key_input(key.code, key.modifiers).await?;
                        }
                    }                    Event::Mouse(mouse) => {
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
        }

        Ok(())
    }
    async fn handle_key_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match &self.app_state {
            AppState::Connecting => {
                // Handle connect screen input
                if let Some(connection_result) =
                    self.connect_screen.handle_input(key, modifiers).await?
                {
                    self.handle_connection_result(connection_result);
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
                }
            }
        }

        Ok(())
    }    async fn handle_mouse_input(&mut self, mouse: MouseEvent, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
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
                                self.handle_connection_result(connection_result);
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
            }
            AppState::Connected(_) => {
                // Browse screen doesn't handle mouse events yet
                // Could add mouse support for node selection in the future
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
                        self.handle_connection_result(connection_result);
                    }
                    Ok(None) => {
                        // No change, continue as normal
                    }
                    Err(e) => {
                        log::error!("Error handling connect screen operations: {}", e);
                    }
                }
            }
            AppState::Connected(_) => {
                // Update connection status from client manager
                if let Ok(client) = self.client_manager.try_lock() {
                    let status = client.get_connection_status();
                    if status == ConnectionStatus::Disconnected {
                        // Connection was lost, go back to connect screen
                        log::warn!("Lost connection to server, returning to connect screen");
                        self.app_state = AppState::Connecting;
                        self.browse_screen = None;
                        self.connect_screen.reset();
                    }
                }
            }
        }
    }

    /// Helper method to handle connection results consistently
    fn handle_connection_result(&mut self, connection_result: ConnectionStatus) {
        match connection_result {
            ConnectionStatus::Connected => {
                // Get the server URL from the connect screen
                let server_url = self.connect_screen.get_server_url();
                log::info!("Successfully connected to: {}", server_url);

                // Update client manager status
                if let Ok(mut client) = self.client_manager.try_lock() {
                    client.set_connection_status(ConnectionStatus::Connected);
                }

                // Transition to browse screen
                self.app_state = AppState::Connected(server_url.clone());
                self.browse_screen = Some(BrowseScreen::new(server_url));
            }
            ConnectionStatus::Disconnected => {
                // User cancelled connection or wants to quit
                self.should_quit = true;
            }
        }
    }

    fn render_ui(&mut self, f: &mut Frame) {
        let size = f.size();

        match &self.app_state {
            AppState::Connecting => {
                // Show connect screen with help line
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),    // Connect screen
                        Constraint::Length(1), // Help line
                    ])
                    .split(size);

                self.connect_screen.render(f, chunks[0]);
                self.connect_screen.render_help_line(f, chunks[1]);
            }
            AppState::Connected(_) => {
                // Show browse screen with help line
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),    // Browse screen
                        Constraint::Length(1), // Help line
                    ])
                    .split(size);

                if let Some(browse_screen) = &mut self.browse_screen {
                    browse_screen.render(f, chunks[0]);
                    browse_screen.render_help_line(f, chunks[1]);
                }
            }
        }
    }
}
