use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, KeyEventKind, MouseEvent, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    io::{self, Stdout},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::client::{OpcUaClientManager, ConnectionStatus};
use crate::menu::{MenuRenderer, MenuType};
use crate::statusbar::{StatusBarRenderer, Screen};
use crate::screens::ConnectScreen;

pub struct App {
    client_manager: Arc<Mutex<OpcUaClientManager>>,
    should_quit: bool,
    
    // Status
    status_message: String,
    connection_status: ConnectionStatus,
    
    // UI components
    menu_renderer: MenuRenderer,
    statusbar_renderer: StatusBarRenderer,
    
    // Screens
    connect_screen: ConnectScreen,
    show_connect_screen: bool,
}

impl App {
    pub fn new(client_manager: Arc<Mutex<OpcUaClientManager>>) -> Self {
        Self {
            client_manager,
            should_quit: false,
            status_message: "Ready".to_string(),
            connection_status: ConnectionStatus::Disconnected,
            menu_renderer: MenuRenderer::new(),
            statusbar_renderer: StatusBarRenderer::new(),
            connect_screen: ConnectScreen::new(),
            show_connect_screen: true, // Show connect screen on startup
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        execute!(terminal.backend_mut(), crossterm::event::EnableMouseCapture)?;

        let result = self.run_app(&mut terminal).await;
        
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture)?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        result
    }

    async fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(250);

        loop {
            terminal.draw(|f| self.ui(f))?;

            let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));
            
            if crossterm::event::poll(timeout)? {                match event::read()? {
                    Event::Key(key) => {
                        // Only process key press events, not key release
                        if key.kind == KeyEventKind::Press {
                            self.handle_input(key.code, key.modifiers).await?;
                        }
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
    }    async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {        // Handle connect screen inputs when it's shown
        if self.show_connect_screen {
            if let Some(connection_result) = self.connect_screen.handle_input(key, modifiers).await? {
                match connection_result {
                    ConnectionStatus::Connected => {
                        self.connection_status = ConnectionStatus::Connected;
                        self.show_connect_screen = false;
                        self.status_message = "Connected to OPC UA server".to_string();
                    }
                    ConnectionStatus::Disconnected => {
                        self.show_connect_screen = false;
                        self.status_message = "Connection cancelled".to_string();
                    }
                    _ => {}
                }
            }
            return Ok(()); // Early return to prevent double processing
        }

        // Global hotkeys
        if modifiers.contains(KeyModifiers::ALT) {
            match key {
                KeyCode::Char('f') => {
                    // File menu
                    self.open_connect_dialog();
                }
                KeyCode::Char('x') => {
                    self.should_quit = true;
                }
                _ => {}
            }
            return Ok(());
        }        // Regular key handling
        match key {
            KeyCode::Esc => {
                // Close any open menus
                self.menu_renderer.close_menu();
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_connect_dialog();
            }
            _ => {}
        }

        Ok(())
    }    async fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        // Ignore mouse move events to prevent spam
        if let MouseEventKind::Moved = mouse.kind {
            return Ok(());
        }
        
        match mouse.kind {MouseEventKind::Down(MouseButton::Left) => {
                // Handle connect screen mouse down when shown
                if self.show_connect_screen {
                    self.connect_screen.handle_mouse_down(mouse.column, mouse.row);
                    return Ok(());
                }
                
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
            MouseEventKind::Up(MouseButton::Left) => {
                // Handle connect screen mouse up when shown
                if self.show_connect_screen {
                    if let Some(button_id) = self.connect_screen.handle_mouse_up(mouse.column, mouse.row) {
                        // Handle button action
                        if let Some(connection_result) = self.connect_screen.handle_button_action(&button_id).await? {
                            match connection_result {
                                ConnectionStatus::Connected => {
                                    self.connection_status = ConnectionStatus::Connected;
                                    self.show_connect_screen = false;
                                    self.status_message = "Connected to OPC UA server".to_string();
                                }
                                ConnectionStatus::Disconnected => {
                                    self.show_connect_screen = false;
                                    self.status_message = "Connection cancelled".to_string();
                                }
                                _ => {}
                            }
                        }
                    }
                    return Ok(());
                }
            }
            _ => {} // Ignore other mouse events like Move
        }
        Ok(())
    }

    async fn handle_menu_click(&mut self, column: u16) -> Result<()> {
        // Update menu renderer connection status first
        self.menu_renderer.set_connection_status(self.connection_status.clone());
        
        if let Some(_menu_type) = self.menu_renderer.handle_menu_click(column) {
            // Menu state updated in renderer
        }
        Ok(())
    }

    async fn handle_dropdown_click(&mut self, menu_type: MenuType, column: u16, row: u16) -> Result<()> {
        match menu_type {
            MenuType::File => {
                // File dropdown: x=1, y=1, width=25, height=6
                if column >= 1 && column <= 25 && row >= 1 && row <= 6 {
                    match row {
                        2 => {
                            // "Connect..." clicked
                            self.open_connect_dialog();
                            self.menu_renderer.close_menu();
                        }
                        3 => {
                            // "Server Info..." clicked
                            self.status_message = "Server info not implemented yet".to_string();
                            self.menu_renderer.close_menu();
                        }
                        5 => {
                            // "Exit" clicked (after separator line)
                            self.should_quit = true;
                            self.menu_renderer.close_menu();
                        }
                        _ => {}
                    }
                } else {
                    // Click outside dropdown closes it
                    self.menu_renderer.close_menu();
                }
            }
            MenuType::Browse => {
                // Browse dropdown: x=9, y=1, width=20, height=4
                if column >= 9 && column <= 28 && row >= 1 && row <= 4 {
                    match row {
                        2 => {
                            // "Browse Server" clicked
                            self.status_message = "Browse functionality not implemented yet".to_string();
                            self.menu_renderer.close_menu();
                        }
                        3 => {
                            // "Refresh" clicked
                            self.status_message = "Refresh functionality not implemented yet".to_string();
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
    }

    async fn on_tick(&mut self) {
        // Update connection status
        if let Ok(client) = self.client_manager.try_lock() {
            self.connection_status = client.get_connection_status();
        }
        
        // Update renderers with current state
        self.menu_renderer.set_connection_status(self.connection_status.clone());
        self.statusbar_renderer.set_connection_status(self.connection_status.clone());
        self.statusbar_renderer.set_current_screen(Screen::Main);
        self.statusbar_renderer.set_status_message(self.status_message.clone());
    }

    fn ui(&mut self, f: &mut Frame) {
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
        if self.show_connect_screen {
            self.connect_screen.render(f, chunks[1]);
        } else {
            self.render_main_screen(f, chunks[1]);
        }

        // Status bar
        self.statusbar_renderer.render_status_bar(f, chunks[2], self.menu_renderer.get_active_menu().as_ref());

        // Render dropdown menus (must be last to appear on top)
        if let Some(menu_type) = self.menu_renderer.get_active_menu() {
            self.menu_renderer.render_dropdown_menu(f, menu_type);
        }
    }

    fn render_main_screen(&self, f: &mut Frame, area: Rect) {
        let welcome_text = vec![
            Span::raw("Welcome to OPC UA Client\n\n"),
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),            Span::raw(match self.connection_status {
                ConnectionStatus::Connected => "Connected",
                ConnectionStatus::Disconnected => "Disconnected",
                ConnectionStatus::Connecting => "Connecting...",
                ConnectionStatus::Error => "Error",
            }),
            Span::raw("\n\n"),
            Span::styled("Quick Actions:\n", Style::default().fg(Color::Green)),
            Span::raw("• Ctrl+C or Alt+F → Connect to server\n"),
            Span::raw("• Alt+X → Exit application\n"),
            Span::raw("\nUse the menu above for more options."),
        ];

        let welcome = Paragraph::new(Line::from(welcome_text))
            .block(Block::default()
                .title("OPC UA Client")
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        
        f.render_widget(welcome, area);
    }

    fn open_connect_dialog(&mut self) {
        self.show_connect_screen = true;
        self.connect_screen.reset();
        self.status_message = "Opening connection dialog...".to_string();
    }
}
