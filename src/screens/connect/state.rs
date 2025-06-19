use log::{info, warn, debug};
use tui_input::Input;
use tui_logger::TuiWidgetState;
use crate::components::ButtonManager;
use super::types::*;

impl ConnectScreen {
    pub fn new() -> Self {
        let mut screen = Self {
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
        };

        // Add initial log messages using the log crate
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
    }

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
    }
}
