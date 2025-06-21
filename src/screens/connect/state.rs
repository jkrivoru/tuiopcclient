use super::types::*;
use crate::components::ButtonManager;
use log::{debug, info, warn};
use tui_input::Input;
use tui_logger::TuiWidgetState;

impl ConnectScreen {    pub fn new() -> Self {
        let mut screen = Self {
            step: ConnectDialogStep::ServerUrl,
            server_url_input: Input::default().with_value("opc.tcp://localhost:4840".to_string()),
            server_url_validation_error: None,
            discovered_endpoints: Vec::new(),
            selected_endpoint_index: usize::default(),
            endpoint_scroll_offset: 0,
            current_visible_endpoints_count: 0,

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
            user_private_key_input: Input::default(),            connect_in_progress: false,
            pending_discovery: false,
            pending_connection: false,
            show_security_validation: false,
            show_auth_validation: false,

            // OPC UA connection state
            client: None,
            session: None,

            input_mode: InputMode::Editing,
            logger_widget_state: TuiWidgetState::new(),
            button_manager: ButtonManager::new(),
        };

        // Add initial log messages using the log crate        info!("OPC UA Client initialized");
        info!("Loading configuration from config.json");
        warn!("Configuration file not found, using defaults");
        info!("Default server URL loaded");
        debug!("Button manager created with hotkeys");
        debug!("Input handlers configured");

        screen.setup_buttons_for_current_step();
        screen
    }
    pub fn reset(&mut self) {
        self.step = ConnectDialogStep::ServerUrl;
        self.server_url_input = Input::default().with_value("opc.tcp://localhost:4840".to_string());
        self.server_url_validation_error = None;
        self.discovered_endpoints.clear();
        self.selected_endpoint_index = 0;
        self.endpoint_scroll_offset = 0;

        // Reset security configuration
        self.client_certificate_input.reset();
        self.client_private_key_input.reset();
        self.auto_trust_server_cert = true;
        self.trusted_server_store_input.reset();
        self.active_security_field = SecurityField::ClientCertificate;        self.authentication_type = AuthenticationType::Anonymous;
        self.username_input.reset();
        self.password_input.reset();
        self.connect_in_progress = false;
        self.pending_discovery = false;
        self.pending_connection = false;

        // Clean up OPC UA connection
        if let Some(session) = &self.session {
            session.write().disconnect();
        }
        self.client = None;
        self.session = None;

        self.input_mode = InputMode::Editing;
        self.setup_buttons_for_current_step();
    }
}
