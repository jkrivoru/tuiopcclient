use tui_input::Input;
use tui_logger::TuiWidgetState;
use crate::components::ButtonManager;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectDialogStep {
    ServerUrl,
    EndpointSelection,
    SecurityConfiguration, // New step for security settings
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

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityField {
    ClientCertificate,
    ClientPrivateKey,
    AutoTrustCheckbox,
    TrustedServerStore,
}

pub struct ConnectScreen {
    // Connection dialog state
    pub step: ConnectDialogStep,
    pub server_url_input: Input,
    pub server_url_validation_error: Option<String>,
    pub discovered_endpoints: Vec<EndpointInfo>,
    pub selected_endpoint_index: usize,
    pub endpoint_scroll_offset: usize, // New field for scrolling
    
    // Security configuration
    pub client_certificate_input: Input,
    pub client_private_key_input: Input,
    pub auto_trust_server_cert: bool,
    pub trusted_server_store_input: Input,
    pub active_security_field: SecurityField,
    
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
    
    pub fn update_endpoint_scroll(&mut self, visible_items: usize) {
        if self.discovered_endpoints.is_empty() {
            return;
        }
        
        // Ensure the selected item is visible
        if self.selected_endpoint_index < self.endpoint_scroll_offset {
            // Selected item is above the visible area - scroll up
            self.endpoint_scroll_offset = self.selected_endpoint_index;
        } else if self.selected_endpoint_index >= self.endpoint_scroll_offset + visible_items {
            // Selected item is below the visible area - scroll down
            self.endpoint_scroll_offset = self.selected_endpoint_index - visible_items + 1;
        }
        
        // Ensure we don't scroll past the end
        let max_scroll = if self.discovered_endpoints.len() > visible_items {
            self.discovered_endpoints.len() - visible_items
        } else {
            0
        };
        
        if self.endpoint_scroll_offset > max_scroll {
            self.endpoint_scroll_offset = max_scroll;
        }
    }
    
    pub fn has_endpoints_above(&self) -> bool {
        self.endpoint_scroll_offset > 0
    }
    
    pub fn has_endpoints_below(&self, visible_items: usize) -> bool {
        self.endpoint_scroll_offset + visible_items < self.discovered_endpoints.len()
    }
    
    pub fn needs_security_configuration(&self) -> bool {
        if self.discovered_endpoints.is_empty() {
            return false;
        }
        
        let selected_endpoint = &self.discovered_endpoints[self.selected_endpoint_index];
        !matches!(
            (&selected_endpoint.security_policy, &selected_endpoint.security_mode),
            (SecurityPolicy::None, SecurityMode::None)
        )
    }
    
    pub fn validate_security_fields(&self) -> Vec<String> {
        let mut errors = Vec::new();
        
        let cert_path = self.client_certificate_input.value().trim();
        let key_path = self.client_private_key_input.value().trim();
        let store_path = self.trusted_server_store_input.value().trim();
        
        // Validate client certificate if provided
        if !cert_path.is_empty() {
            if !std::path::Path::new(cert_path).exists() {
                errors.push(format!("Client certificate file not found: {}", cert_path));
            } else {
                let ext = std::path::Path::new(cert_path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if !["der", "pem", "crt", "cer"].contains(&ext.to_lowercase().as_str()) {
                    errors.push("Client certificate must be a .der, .pem, .crt, or .cer file".to_string());
                }
            }
        }
        
        // Validate private key if provided
        if !key_path.is_empty() {
            if !std::path::Path::new(key_path).exists() {
                errors.push(format!("Client private key file not found: {}", key_path));
            } else {
                let ext = std::path::Path::new(key_path)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if !["pem", "key"].contains(&ext.to_lowercase().as_str()) {
                    errors.push("Client private key must be a .pem or .key file".to_string());
                }
            }
        }
        
        // Validate trusted server store if not auto-trusting and provided
        if !self.auto_trust_server_cert && !store_path.is_empty() {
            if !std::path::Path::new(store_path).exists() {
                errors.push(format!("Trusted server store path not found: {}", store_path));
            }
        }
        
        // Check if both certificate and key are provided together
        if (!cert_path.is_empty() && key_path.is_empty()) || (cert_path.is_empty() && !key_path.is_empty()) {
            errors.push("Both client certificate and private key must be provided together, or both left empty".to_string());
        }
        
        errors
    }
    
    pub fn get_security_validation_summary(&self) -> Option<String> {
        let errors = self.validate_security_fields();
        if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        }
    }
}
