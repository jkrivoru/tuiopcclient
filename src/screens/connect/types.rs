use tui_input::Input;
use tui_logger::TuiWidgetState;
use crate::components::ButtonManager;

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
    pub server_url_input: Input,
    pub server_url_validation_error: Option<String>,
    pub discovered_endpoints: Vec<EndpointInfo>,
    pub selected_endpoint_index: usize,
    pub authentication_type: AuthenticationType,
    pub active_auth_field: AuthenticationField,
    pub username_input: Input,
    pub password_input: Input,
    pub connect_in_progress: bool,
    pub pending_discovery: bool, // New field to track if discovery should happen
    
    // Input handling
    pub input_mode: InputMode,
    
    // Logger widget state
    pub logger_widget_state: TuiWidgetState,
    
    // Button management
    pub button_manager: ButtonManager,
}

impl ConnectScreen {    pub fn validate_server_url(&mut self) {
        let url = self.server_url_input.value();
        
        if url.is_empty() {
            self.server_url_validation_error = Some("Server URL cannot be empty".to_string());
            return;
        }
        
        // Create case-insensitive regex for validation
        let case_insensitive_regex = regex::RegexBuilder::new(r"^opc\.tcp://([a-zA-Z0-9.-]+|\d{1,3}(\.\d{1,3}){3})(:\d{1,5})?$")
            .case_insensitive(true)
            .build()
            .expect("Invalid regex pattern");
        
        if !case_insensitive_regex.is_match(url) {
            self.server_url_validation_error = Some("Invalid URL format. Expected: opc.tcp://hostname:port or opc.tcp://ip:port".to_string());
        } else {
            self.server_url_validation_error = None;
        }
    }
    
    pub fn get_server_url(&self) -> String {
        self.server_url_input.value().to_string()
    }
    
    pub fn show_placeholder(&self) -> bool {
        self.server_url_input.value().is_empty() && self.input_mode == InputMode::Editing
    }
}
