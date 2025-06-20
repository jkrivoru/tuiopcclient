use super::types::*;
use crate::client::ConnectionStatus;
use crate::components::{Button, ButtonColor};
use anyhow::Result;
use log::{error, info, warn};
use std::time::Duration;
use tokio::time;
use tui_input::backend::crossterm::EventHandler;

impl ConnectScreen {
    pub async fn discover_endpoints(&mut self) -> Result<()> {
        info!("Discovering endpoints...");
        warn!("Using demo mode - actual endpoint discovery not implemented yet");
        // Check for invalid URL to demonstrate error logging
        let url = self.get_server_url();
        if !url.starts_with("opc.tcp://") {
            error!("Invalid OPC UA server URL: must start with 'opc.tcp://'");
            return Ok(());
        }

        // Simulate endpoint discovery with a longer delay to show popup        time::sleep(Duration::from_millis(1500)).await;

        // Mock discovered endpoints - comprehensive test data
        self.discovered_endpoints = vec![
            // No Security
            EndpointInfo {
                security_policy: SecurityPolicy::None,
                security_mode: SecurityMode::None,
                display_name: "None - No Security".to_string(),
            },
            // Basic128Rsa15 combinations
            EndpointInfo {
                security_policy: SecurityPolicy::Basic128Rsa15,
                security_mode: SecurityMode::Sign,
                display_name: "Basic128Rsa15 - Sign Only".to_string(),
            },
            EndpointInfo {
                security_policy: SecurityPolicy::Basic128Rsa15,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Basic128Rsa15 - Sign & Encrypt".to_string(),
            },
            // Basic256 combinations
            EndpointInfo {
                security_policy: SecurityPolicy::Basic256,
                security_mode: SecurityMode::Sign,
                display_name: "Basic256 - Sign Only".to_string(),
            },
            EndpointInfo {
                security_policy: SecurityPolicy::Basic256,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Basic256 - Sign & Encrypt".to_string(),
            },
            // Basic256Sha256 combinations (most common)
            EndpointInfo {
                security_policy: SecurityPolicy::Basic256Sha256,
                security_mode: SecurityMode::Sign,
                display_name: "Basic256Sha256 - Sign Only".to_string(),
            },
            EndpointInfo {
                security_policy: SecurityPolicy::Basic256Sha256,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Basic256Sha256 - Sign & Encrypt".to_string(),
            },
            // Aes128Sha256RsaOaep combinations
            EndpointInfo {
                security_policy: SecurityPolicy::Aes128Sha256RsaOaep,
                security_mode: SecurityMode::Sign,
                display_name: "Aes128Sha256RsaOaep - Sign Only".to_string(),
            },
            EndpointInfo {
                security_policy: SecurityPolicy::Aes128Sha256RsaOaep,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Aes128Sha256RsaOaep - Sign & Encrypt".to_string(),
            },
            // Aes256Sha256RsaPss combinations (most secure)
            EndpointInfo {
                security_policy: SecurityPolicy::Aes256Sha256RsaPss,
                security_mode: SecurityMode::Sign,
                display_name: "Aes256Sha256RsaPss - Sign Only".to_string(),
            },
            EndpointInfo {
                security_policy: SecurityPolicy::Aes256Sha256RsaPss,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Aes256Sha256RsaPss - Sign & Encrypt".to_string(),
            },
        ];
        info!(
            "Found {} endpoints with various security configurations",
            self.discovered_endpoints.len()
        );
        info!("Endpoints range from no security to high-security configurations");
        info!("Use Up/Down arrows to navigate, Enter to select endpoint");

        Ok(())
    }
    pub async fn connect_with_settings(&mut self) -> Result<Option<ConnectionStatus>> {
        // Show validation highlighting when Connect is clicked
        self.show_auth_validation = true;

        // Validate authentication fields before proceeding
        let validation_errors = self.validate_authentication_fields();
        if !validation_errors.is_empty() {
            // Log validation errors
            for error in &validation_errors {
                log::error!("Authentication Validation: {}", error);
            }
            // Don't proceed if there are validation errors
            return Ok(None);
        }

        // Set flags for showing popup and triggering connection
        self.connect_in_progress = true;
        self.pending_connection = true;
        Ok(None) // Return immediately to show the popup
    }    pub async fn perform_connection(&mut self) -> Result<Option<ConnectionStatus>> {
        info!("Starting connection process...");

        let auth_desc = match self.authentication_type {
            AuthenticationType::Anonymous => "Anonymous".to_string(),
            AuthenticationType::UserPassword => format!("User: {}", self.username_input.value()),
            AuthenticationType::X509Certificate => format!(
                "Certificate: {}",
                std::path::Path::new(self.user_certificate_input.value())
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
            ),
        };

        info!("Connecting with {}", auth_desc);

        // Get the server URL from input
        let server_url = self.server_url_input.value().trim();
        
        if server_url.is_empty() {
            log::error!("Server URL is empty");
            return Ok(Some(ConnectionStatus::Error("Server URL is required".to_string())));
        }

        info!("Attempting to connect to: {}", server_url);

        // Validate the server URL format
        if !server_url.starts_with("opc.tcp://") {
            log::error!("Invalid OPC UA server URL: must start with 'opc.tcp://'");
            return Ok(Some(ConnectionStatus::Error("Invalid URL format. Must start with 'opc.tcp://'".to_string())));
        }

        // Return Connecting status - the actual connection will be handled by the UI layer
        // This ensures we don't show "Connected" until the real connection succeeds
        info!("Connection parameters validated, initiating connection...");
        Ok(Some(ConnectionStatus::Connecting))
    }
    pub async fn handle_button_action(
        &mut self,
        button_id: &str,
    ) -> Result<Option<ConnectionStatus>> {
        match button_id {
            "cancel" => Ok(Some(ConnectionStatus::Disconnected)),
            "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl => {
                        // Use unified method for consistent behavior
                        self.advance_to_next_step()?;
                        Ok(None) // Return immediately to show the popup
                    }
                    ConnectDialogStep::EndpointSelection => {
                        // Use unified method for consistent behavior
                        self.advance_to_next_step()?;
                        Ok(None)
                    }
                    ConnectDialogStep::SecurityConfiguration => {
                        // Use unified method for consistent behavior
                        self.advance_to_next_step()?;
                        Ok(None)
                    }
                    _ => Ok(None),
                }
            }
            "back" => {
                match self.step {
                    ConnectDialogStep::EndpointSelection => {
                        self.step = ConnectDialogStep::ServerUrl;
                        self.input_mode = InputMode::Editing;
                        self.setup_buttons_for_current_step();
                        Ok(None)
                    }
                    ConnectDialogStep::SecurityConfiguration => {
                        self.step = ConnectDialogStep::EndpointSelection;
                        self.input_mode = InputMode::Normal;
                        self.setup_buttons_for_current_step();
                        Ok(None)
                    }
                    ConnectDialogStep::Authentication => {
                        // Check if we came from security configuration or endpoint selection
                        self.navigate_back_from_auth();
                        Ok(None)
                    }
                    _ => Ok(None),
                }
            }
            "connect" => self.connect_with_settings().await,
            _ => Ok(None),
        }
    }

    pub fn setup_buttons_for_current_step(&mut self) {
        self.button_manager.clear();

        match self.step {
            ConnectDialogStep::ServerUrl => {
                // Step 1: Only Cancel and Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red),
                );

                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.connect_in_progress),
                );
            }
            ConnectDialogStep::EndpointSelection => {
                // Step 2: Cancel, Back, Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red),
                );

                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue),
                );

                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green),
                );
            }
            ConnectDialogStep::SecurityConfiguration => {
                // Step 3: Cancel, Back, Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red),
                );

                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue),
                );

                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green),
                );
            }
            ConnectDialogStep::Authentication => {
                // Step 4: Cancel, Back, Connect
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red),
                );

                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue),
                );

                self.button_manager.add_button(
                    Button::new("connect", "Connect")
                        .with_hotkey('o')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.connect_in_progress),
                );
            }
        }
    }
    /// Unified method for advancing to the next step from any step
    /// Ensures consistent validation and state management regardless of how the user continues (button or Enter)
    pub fn advance_to_next_step(&mut self) -> Result<()> {
        match self.step {
            ConnectDialogStep::ServerUrl => {
                // Validate URL first
                self.validate_server_url();
                if self.server_url_validation_error.is_none() {
                    // Set flags for showing popup and triggering discovery
                    self.connect_in_progress = true;
                    self.pending_discovery = true;
                } else {
                    // Show validation error in log
                    if let Some(ref error) = self.server_url_validation_error {
                        log::error!("URL Validation: {}", error);
                    }
                }
                Ok(())
            }
            ConnectDialogStep::EndpointSelection => {
                // Check if security configuration is needed
                if self.needs_security_configuration() {
                    // Move to security configuration step
                    self.step = ConnectDialogStep::SecurityConfiguration;
                    self.active_security_field = SecurityField::ClientCertificate;
                    self.input_mode = InputMode::Editing;
                    // Reset validation highlighting when entering security step
                    self.show_security_validation = false;
                } else {
                    // Skip security and move directly to authentication step
                    self.step = ConnectDialogStep::Authentication;
                    // Reset authentication validation highlighting when entering auth step
                    self.show_auth_validation = false;
                    self.setup_authentication_fields();
                }
                self.setup_buttons_for_current_step();
                Ok(())
            }
            ConnectDialogStep::SecurityConfiguration => {
                // Show validation highlighting when proceeding
                self.show_security_validation = true;

                // Validate security fields before proceeding
                let validation_errors = self.validate_security_fields();
                if !validation_errors.is_empty() {
                    // Log validation errors
                    for error in &validation_errors {
                        log::error!("Security Validation: {}", error);
                    }
                    // Don't proceed if there are validation errors
                    return Ok(());
                }

                // Move to authentication step
                self.step = ConnectDialogStep::Authentication;
                // Reset authentication validation highlighting when entering auth step
                self.show_auth_validation = false;
                self.setup_authentication_fields();
                self.setup_buttons_for_current_step();
                Ok(())
            }
            ConnectDialogStep::Authentication => {
                // This case is handled separately in connect_with_settings due to async connection process
                Ok(())
            }
        }
    } // Method to be called from the main UI loop to handle pending operations
    pub async fn handle_pending_operations(&mut self) -> Result<Option<ConnectionStatus>> {
        if self.pending_discovery {
            self.pending_discovery = false;

            // Perform the actual discovery
            self.discover_endpoints().await?;

            // After discovery, hide popup and transition to next step
            self.connect_in_progress = false;
            self.step = ConnectDialogStep::EndpointSelection;
            self.setup_buttons_for_current_step();
            self.input_mode = InputMode::Normal;
        }

        if self.pending_connection {
            self.pending_connection = false;

            // Perform the actual connection
            let connection_result = self.perform_connection().await?;

            // After connection attempt, hide popup
            self.connect_in_progress = false;

            // Return the connection result so the main UI can handle the transition
            return Ok(connection_result);
        }

        Ok(None)
    }
    /// Helper method to setup authentication fields based on type
    fn setup_authentication_fields(&mut self) {
        match self.authentication_type {
            AuthenticationType::UserPassword => {
                self.active_auth_field = AuthenticationField::Username;
                self.input_mode = InputMode::Editing;
            }
            AuthenticationType::X509Certificate => {
                self.active_auth_field = AuthenticationField::UserCertificate;
                self.input_mode = InputMode::Editing;
            }
            AuthenticationType::Anonymous => {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    /// Helper method to cycle authentication types
    pub fn cycle_authentication_type(&mut self) {
        self.authentication_type = match self.authentication_type {
            AuthenticationType::Anonymous => AuthenticationType::UserPassword,
            AuthenticationType::UserPassword => AuthenticationType::X509Certificate,
            AuthenticationType::X509Certificate => AuthenticationType::Anonymous,
        };
        self.setup_authentication_fields();
    }

    /// Helper method to cycle authentication types backward (up)
    pub fn cycle_authentication_type_backward(&mut self) {
        self.authentication_type = match self.authentication_type {
            AuthenticationType::Anonymous => AuthenticationType::X509Certificate,
            AuthenticationType::UserPassword => AuthenticationType::Anonymous,
            AuthenticationType::X509Certificate => AuthenticationType::UserPassword,
        };
        self.setup_authentication_fields();
    }

    /// Helper method to navigate authentication fields with Tab
    pub fn navigate_auth_fields_forward(&mut self) {
        match self.authentication_type {
            AuthenticationType::UserPassword => {
                self.active_auth_field = match self.active_auth_field {
                    AuthenticationField::Username => AuthenticationField::Password,
                    AuthenticationField::Password => AuthenticationField::Username,
                    _ => AuthenticationField::Username,
                };
                self.input_mode = InputMode::Editing;
            }
            AuthenticationType::X509Certificate => {
                self.active_auth_field = match self.active_auth_field {
                    AuthenticationField::UserCertificate => AuthenticationField::UserPrivateKey,
                    AuthenticationField::UserPrivateKey => AuthenticationField::UserCertificate,
                    _ => AuthenticationField::UserCertificate,
                };
                self.input_mode = InputMode::Editing;
            }
            AuthenticationType::Anonymous => {
                // No fields to navigate
            }
        }
    }

    /// Helper method to navigate security fields forward (Tab)
    pub fn navigate_security_fields_forward(&mut self) {
        match self.active_security_field {
            SecurityField::ClientCertificate => {
                self.active_security_field = SecurityField::ClientPrivateKey;
                self.input_mode = InputMode::Editing;
            }
            SecurityField::ClientPrivateKey => {
                self.active_security_field = SecurityField::AutoTrustCheckbox;
                self.input_mode = InputMode::Normal;
            }
            SecurityField::AutoTrustCheckbox => {
                let (next_field, mode) = self.get_next_security_field_from_checkbox();
                self.active_security_field = next_field;
                self.input_mode = mode;
            }
            SecurityField::TrustedServerStore => {
                self.active_security_field = SecurityField::ClientCertificate;
                self.input_mode = InputMode::Editing;
            }
        }
    }

    /// Helper method to navigate security fields backward (Shift+Tab)
    pub fn navigate_security_fields_backward(&mut self) {
        match self.active_security_field {
            SecurityField::ClientCertificate => {
                let (prev_field, mode) = self.get_prev_security_field_to_checkbox();
                self.active_security_field = prev_field;
                self.input_mode = mode;
            }
            SecurityField::ClientPrivateKey => {
                self.active_security_field = SecurityField::ClientCertificate;
                self.input_mode = InputMode::Editing;
            }
            SecurityField::AutoTrustCheckbox => {
                let (prev_field, mode) = self.get_prev_security_field_to_checkbox();
                self.active_security_field = prev_field;
                self.input_mode = mode;
            }
            SecurityField::TrustedServerStore => {
                self.active_security_field = SecurityField::AutoTrustCheckbox;
                self.input_mode = InputMode::Normal;
            }
        }
    }

    /// Helper method to handle authentication field character input
    pub fn handle_auth_field_input(
        &mut self,
        key: crossterm::event::KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) {
        match self.authentication_type {
            AuthenticationType::UserPassword => match self.active_auth_field {
                AuthenticationField::Username => {
                    self.username_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                }
                AuthenticationField::Password => {
                    self.password_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                }
                _ => {}
            },
            AuthenticationType::X509Certificate => match self.active_auth_field {
                AuthenticationField::UserCertificate => {
                    self.user_certificate_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                }
                AuthenticationField::UserPrivateKey => {
                    self.user_private_key_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                }
                _ => {}
            },
            AuthenticationType::Anonymous => {
                // No input fields for anonymous
            }
        }
    }
    /// Helper method to handle security field character input
    pub fn handle_security_field_input(
        &mut self,
        key: crossterm::event::KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) {
        match self.active_security_field {
            SecurityField::ClientCertificate => {
                self.client_certificate_input
                    .handle_event(&crossterm::event::Event::Key(
                        crossterm::event::KeyEvent::new(key, modifiers),
                    ));
            }
            SecurityField::ClientPrivateKey => {
                self.client_private_key_input
                    .handle_event(&crossterm::event::Event::Key(
                        crossterm::event::KeyEvent::new(key, modifiers),
                    ));
            }
            SecurityField::TrustedServerStore => {
                if !self.auto_trust_server_cert {
                    self.trusted_server_store_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                }
            }
            SecurityField::AutoTrustCheckbox => {
                // Checkbox doesn't handle character input
            }
        }
    }

    /// Helper method to navigate back to the previous step from authentication
    pub fn navigate_back_from_auth(&mut self) {
        if self.needs_security_configuration() {
            self.step = ConnectDialogStep::SecurityConfiguration;
            self.active_security_field = SecurityField::ClientCertificate;
            self.input_mode = InputMode::Editing;
            // Reset security validation highlighting when going back
            self.show_security_validation = false;
        } else {
            self.step = ConnectDialogStep::EndpointSelection;
            self.input_mode = InputMode::Normal;
        }
        // Reset authentication validation highlighting when going back
        self.show_auth_validation = false;
        self.setup_buttons_for_current_step();
    }

    /// Helper method to get the next security field after auto-trust checkbox
    fn get_next_security_field_from_checkbox(&self) -> (SecurityField, InputMode) {
        if !self.auto_trust_server_cert {
            (SecurityField::TrustedServerStore, InputMode::Editing)
        } else {
            (SecurityField::ClientCertificate, InputMode::Editing)
        }
    }

    /// Helper method to get the previous security field before auto-trust checkbox  
    fn get_prev_security_field_to_checkbox(&self) -> (SecurityField, InputMode) {
        if !self.auto_trust_server_cert {
            (SecurityField::TrustedServerStore, InputMode::Editing)
        } else {
            (SecurityField::AutoTrustCheckbox, InputMode::Normal)
        }
    }
}
