use crate::components::ButtonManager;
use opcua::client::prelude::*;
use opcua::types::EndpointDescription;
use parking_lot::RwLock;
use std::sync::Arc;
use tui_input::Input;
use tui_logger::TuiWidgetState;

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
    X509Certificate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthenticationField {
    Username,
    Password,
    UserCertificate,
    UserPrivateKey,
}

#[derive(Debug, Clone)]
pub struct EndpointInfo {
    pub security_policy: SecurityPolicy,
    pub security_mode: SecurityMode,
    pub display_name: String,
    pub original_endpoint: EndpointDescription, // Store the original OPC UA endpoint
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
    pub use_original_url: bool, // New field for forcing original URL usage
    pub discovered_endpoints: Vec<EndpointInfo>,
    pub selected_endpoint_index: usize,
    pub endpoint_scroll_offset: usize, // New field for scrolling
    pub current_visible_endpoints_count: usize, // Store actual visible endpoints from render

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
    pub user_certificate_input: Input,
    pub user_private_key_input: Input,
    pub connect_in_progress: bool,
    pub pending_discovery: bool, // New field to track if discovery should happen
    pub pending_connection: bool, // New field to track if connection should happen
    pub show_security_validation: bool, // Track whether to show validation highlighting
    pub show_auth_validation: bool, // Track whether to show authentication validation highlighting

    // OPC UA connection state
    pub client: Option<Client>,
    pub session: Option<Arc<RwLock<Session>>>,

    // Input handling
    pub input_mode: InputMode,

    // Logger widget state
    pub logger_widget_state: TuiWidgetState,

    // Button management
    pub button_manager: ButtonManager,
}

impl ConnectScreen {
    pub fn validate_server_url(&mut self) {
        let url = self.server_url_input.value();

        if url.is_empty() {
            self.server_url_validation_error = Some("Server URL cannot be empty".to_string());
            return;
        }

        // Create case-insensitive regex for validation
        let case_insensitive_regex = regex::RegexBuilder::new(
            r"^opc\.tcp://([a-zA-Z0-9.-]+|\d{1,3}(\.\d{1,3}){3})(:\d{1,5})?$",
        )
        .case_insensitive(true)
        .build()
        .expect("Invalid regex pattern");

        if !case_insensitive_regex.is_match(url) {
            self.server_url_validation_error = Some(
                "Invalid URL format. Expected: opc.tcp://hostname:port or opc.tcp://ip:port"
                    .to_string(),
            );
        } else {
            self.server_url_validation_error = None;
        }
    }

    pub fn get_server_url(&self) -> String {
        self.server_url_input.value().to_string()
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
    /// Center the selected endpoint in the visible area when possible
    pub fn center_endpoint_in_view(&mut self, visible_items: usize) {
        if self.discovered_endpoints.is_empty() || visible_items == 0 {
            return;
        }

        // Calculate the ideal scroll offset to center the selected item
        let ideal_center_offset = self
            .selected_endpoint_index
            .saturating_sub(visible_items / 2);

        // Ensure we don't scroll past the end
        let max_scroll = if self.discovered_endpoints.len() > visible_items {
            self.discovered_endpoints.len() - visible_items
        } else {
            0
        }; // Set the scroll offset to center the item, or as close as possible
        self.endpoint_scroll_offset = ideal_center_offset.min(max_scroll);
    }

    pub fn needs_security_configuration(&self) -> bool {
        if self.discovered_endpoints.is_empty() {
            return false;
        }

        let selected_endpoint = &self.discovered_endpoints[self.selected_endpoint_index];
        !matches!(
            (
                &selected_endpoint.security_policy,
                &selected_endpoint.security_mode
            ),
            (SecurityPolicy::None, SecurityMode::None)
        )
    }

    pub fn validate_security_fields(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let cert_path = self.client_certificate_input.value().trim();
        let key_path = self.client_private_key_input.value().trim();
        let store_path = self.trusted_server_store_input.value().trim();

        // Client certificate is mandatory
        if cert_path.is_empty() {
            errors.push("Client certificate path is required".to_string());
        } else if !std::path::Path::new(cert_path).exists() {
            errors.push(format!("Client certificate file not found: {cert_path}"));
        } else {
            let ext = std::path::Path::new(cert_path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if !["der", "pem", "crt", "cer"].contains(&ext.to_lowercase().as_str()) {
                errors.push(
                    "Client certificate must be a .der, .pem, .crt, or .cer file".to_string(),
                );
            }
        }

        // Private key is mandatory
        if key_path.is_empty() {
            errors.push("Client private key path is required".to_string());
        } else if !std::path::Path::new(key_path).exists() {
            errors.push(format!("Client private key file not found: {key_path}"));
        } else {
            let ext = std::path::Path::new(key_path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if !["pem", "key"].contains(&ext.to_lowercase().as_str()) {
                errors.push("Client private key must be a .pem or .key file".to_string());
            }
        }

        // Trusted server store is mandatory when auto-trust is disabled
        if !self.auto_trust_server_cert
            && !store_path.is_empty()
            && !std::path::Path::new(store_path).exists()
        {
            errors.push(format!(
                "Trusted server store path not found: {store_path}"
            ));
        }

        errors
    }
    pub fn has_certificate_validation_error(&self) -> bool {
        if !self.show_security_validation {
            return false;
        }
        let cert_path = self.client_certificate_input.value().trim();
        if cert_path.is_empty() {
            return true;
        }
        if !std::path::Path::new(cert_path).exists() {
            return true;
        }
        let ext = std::path::Path::new(cert_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        !["der", "pem", "crt", "cer"].contains(&ext.to_lowercase().as_str())
    }

    pub fn has_private_key_validation_error(&self) -> bool {
        if !self.show_security_validation {
            return false;
        }
        let key_path = self.client_private_key_input.value().trim();
        if key_path.is_empty() {
            return true;
        }
        if !std::path::Path::new(key_path).exists() {
            return true;
        }
        let ext = std::path::Path::new(key_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        !["pem", "key"].contains(&ext.to_lowercase().as_str())
    }

    pub fn has_trusted_store_validation_error(&self) -> bool {
        if !self.show_security_validation {
            return false;
        }
        if self.auto_trust_server_cert {
            return false; // Not required when auto-trust is enabled
        }
        let store_path = self.trusted_server_store_input.value().trim();
        if store_path.is_empty() {
            return false;
        }
        !std::path::Path::new(store_path).exists()
    }

    // Authentication validation helper methods
    pub fn has_username_validation_error(&self) -> bool {
        if !self.show_auth_validation {
            return false;
        }
        if self.authentication_type != AuthenticationType::UserPassword {
            return false;
        }
        self.username_input.value().trim().is_empty()
    }

    pub fn has_password_validation_error(&self) -> bool {
        if !self.show_auth_validation {
            return false;
        }
        if self.authentication_type != AuthenticationType::UserPassword {
            return false;
        }
        self.password_input.value().trim().is_empty()
    }

    pub fn has_user_certificate_validation_error(&self) -> bool {
        if !self.show_auth_validation {
            return false;
        }
        if self.authentication_type != AuthenticationType::X509Certificate {
            return false;
        }
        let cert_path = self.user_certificate_input.value().trim();
        if cert_path.is_empty() {
            return true;
        }
        if !std::path::Path::new(cert_path).exists() {
            return true;
        }
        let ext = std::path::Path::new(cert_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        !["der", "pem", "crt", "cer"].contains(&ext.to_lowercase().as_str())
    }

    pub fn has_user_private_key_validation_error(&self) -> bool {
        if !self.show_auth_validation {
            return false;
        }
        if self.authentication_type != AuthenticationType::X509Certificate {
            return false;
        }
        let key_path = self.user_private_key_input.value().trim();
        if key_path.is_empty() {
            return true;
        }
        if !std::path::Path::new(key_path).exists() {
            return true;
        }
        let ext = std::path::Path::new(key_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        !["pem", "key"].contains(&ext.to_lowercase().as_str())
    }

    // Validate authentication fields
    pub fn validate_authentication_fields(&self) -> Vec<String> {
        let mut errors = Vec::new();

        match self.authentication_type {
            AuthenticationType::UserPassword => {
                let username = self.username_input.value().trim();
                let password = self.password_input.value().trim();

                if username.is_empty() {
                    errors.push("Username is required".to_string());
                }
                if password.is_empty() {
                    errors.push("Password is required".to_string());
                }
            }
            AuthenticationType::X509Certificate => {
                let cert_path = self.user_certificate_input.value().trim();
                let key_path = self.user_private_key_input.value().trim();

                // User certificate is mandatory
                if cert_path.is_empty() {
                    errors.push("User certificate path is required".to_string());
                } else if !std::path::Path::new(cert_path).exists() {
                    errors.push(format!("User certificate file not found: {cert_path}"));
                } else {
                    let ext = std::path::Path::new(cert_path)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if !["der", "pem", "crt", "cer"].contains(&ext.to_lowercase().as_str()) {
                        errors.push(
                            "User certificate must be a .der, .pem, .crt, or .cer file".to_string(),
                        );
                    }
                }

                // User private key is mandatory
                if key_path.is_empty() {
                    errors.push("User private key path is required".to_string());
                } else if !std::path::Path::new(key_path).exists() {
                    errors.push(format!("User private key file not found: {key_path}"));
                } else {
                    let ext = std::path::Path::new(key_path)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if !["pem", "key"].contains(&ext.to_lowercase().as_str()) {
                        errors.push("User private key must be a .pem or .key file".to_string());
                    }
                }
            }
            AuthenticationType::Anonymous => {
                // No validation needed for anonymous
            }
        }
        errors
    }

    /// Get total number of steps in the connect flow
    pub fn get_total_steps(&self) -> u8 {
        if self.needs_security_configuration() {
            4 // Server URL -> Endpoint -> Security -> Authentication
        } else {
            3 // Server URL -> Endpoint -> Authentication
        }
    }

    /// Get current step number (1-based)
    pub fn get_current_step_number(&self) -> u8 {
        match self.step {
            ConnectDialogStep::ServerUrl => 1,
            ConnectDialogStep::EndpointSelection => 2,
            ConnectDialogStep::SecurityConfiguration => 3,
            ConnectDialogStep::Authentication => {
                if self.needs_security_configuration() {
                    4
                } else {
                    3
                }
            }
        }
    }
}
