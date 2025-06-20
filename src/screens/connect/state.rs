use log::{info, warn, debug};
use tui_input::Input;
use tui_logger::TuiWidgetState;
use crate::components::ButtonManager;
use super::types::*;

impl ConnectScreen {    pub fn new() -> Self {        let mut screen = Self {
            step: ConnectDialogStep::ServerUrl,
            server_url_input: Input::default().with_value("opc.tcp://localhost:4840".to_string()),
            server_url_validation_error: None,
            discovered_endpoints: Vec::new(),
            selected_endpoint_index: usize::default(),
            endpoint_scroll_offset: 0,
            
            // Security configuration
            client_certificate_input: Input::default(),
            client_private_key_input: Input::default(),
            auto_trust_server_cert: true,
            trusted_server_store_input: Input::default(),
            active_security_field: SecurityField::ClientCertificate,
              authentication_type: AuthenticationType::Anonymous,
            active_auth_field: AuthenticationField::Username,
            username_input: Input::default(),
            password_input: Input::default(),
            user_certificate_input: Input::default(),
            user_private_key_input: Input::default(),
            connect_in_progress: false,            pending_discovery: false,
            show_security_validation: false,
            show_auth_validation: false,
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
    }    pub fn reset(&mut self) {
        self.step = ConnectDialogStep::ServerUrl;
        self.server_url_input = Input::default().with_value("opc.tcp://localhost:4840".to_string());
        self.server_url_validation_error = None;        self.discovered_endpoints.clear();
        self.selected_endpoint_index = 0;
        self.endpoint_scroll_offset = 0;
        
        // Reset security configuration
        self.client_certificate_input.reset();
        self.client_private_key_input.reset();
        self.auto_trust_server_cert = true;
        self.trusted_server_store_input.reset();
        self.active_security_field = SecurityField::ClientCertificate;
        
        self.authentication_type = AuthenticationType::Anonymous;
        self.username_input.reset();
        self.password_input.reset();
        self.connect_in_progress = false;
        self.pending_discovery = false;
        self.input_mode = InputMode::Editing;        self.setup_buttons_for_current_step();
    }
}
