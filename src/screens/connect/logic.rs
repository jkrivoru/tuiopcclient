use anyhow::Result;
use log::{info, warn, error};
use std::time::Duration;
use tokio::time;
use crate::client::ConnectionStatus;
use crate::components::{Button, ButtonColor};
use super::types::*;

impl ConnectScreen {    pub async fn discover_endpoints(&mut self) -> Result<()> {
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
          info!("Found {} endpoints with various security configurations", self.discovered_endpoints.len());
        info!("Endpoints range from no security to high-security configurations");
        info!("Use Up/Down arrows to navigate, Enter to select endpoint");
        
        Ok(())
    }    pub async fn connect_with_settings(&mut self) -> Result<Option<ConnectionStatus>> {
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
        
        self.connect_in_progress = true;
          let auth_desc = match self.authentication_type {
            AuthenticationType::Anonymous => "Anonymous".to_string(),
            AuthenticationType::UserPassword => format!("User: {}", self.username_input.value()),
            AuthenticationType::X509Certificate => format!("Certificate: {}", 
                std::path::Path::new(self.user_certificate_input.value())
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
            ),
        };
        
        info!("Connecting with {}", auth_desc);
        
        // Simulate connection process
        time::sleep(Duration::from_millis(1000)).await;
        
        // For demo purposes, simulate successful connection
        self.connect_in_progress = false;
        info!("Connected successfully!");
        
        Ok(Some(ConnectionStatus::Connected))
    }    pub async fn handle_button_action(&mut self, button_id: &str) -> Result<Option<ConnectionStatus>> {
        match button_id {
            "cancel" => Ok(Some(ConnectionStatus::Disconnected)),            "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl => {
                        // Set flags for showing popup and triggering discovery
                        self.connect_in_progress = true;
                        self.pending_discovery = true;
                        Ok(None) // Return immediately to show the popup
                    }
                    ConnectDialogStep::EndpointSelection => {                        // Check if security configuration is needed
                        if self.needs_security_configuration() {
                            // Move to security configuration step
                            self.step = ConnectDialogStep::SecurityConfiguration;
                            self.active_security_field = SecurityField::ClientCertificate;
                            self.input_mode = InputMode::Editing;
                            // Reset validation highlighting when entering security step
                            self.show_security_validation = false;                        } else {                            // Skip security and move directly to authentication step
                            self.step = ConnectDialogStep::Authentication;
                            // Reset authentication validation highlighting when entering auth step
                            self.show_auth_validation = false;
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
                        self.setup_buttons_for_current_step();
                        Ok(None)
                    }                    ConnectDialogStep::SecurityConfiguration => {
                        // Show validation highlighting when Next is clicked
                        self.show_security_validation = true;
                        
                        // Validate security fields before proceeding
                        let validation_errors = self.validate_security_fields();
                        if !validation_errors.is_empty() {
                            // Log validation errors
                            for error in &validation_errors {
                                log::error!("Security Validation: {}", error);
                            }
                            // Don't proceed if there are validation errors
                            return Ok(None);
                        }                        // Move to authentication step
                        self.step = ConnectDialogStep::Authentication;
                        // Reset authentication validation highlighting when entering auth step
                        self.show_auth_validation = false;
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
                        self.setup_buttons_for_current_step();
                        Ok(None)
                    }
                    _ => Ok(None)
                }
            }            "back" => {
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
                    }                    ConnectDialogStep::Authentication => {
                        // Check if we came from security configuration or endpoint selection
                        if self.needs_security_configuration() {
                            self.step = ConnectDialogStep::SecurityConfiguration;
                            self.active_security_field = SecurityField::ClientCertificate;
                            self.input_mode = InputMode::Editing;
                            // Reset validation highlighting when going back
                            self.show_security_validation = false;
                        } else {
                            self.step = ConnectDialogStep::EndpointSelection;
                            self.input_mode = InputMode::Normal;
                        }
                        self.setup_buttons_for_current_step();
                        Ok(None)
                    }
                    _ => Ok(None)
                }
            }
            "connect" => {
                self.connect_with_settings().await
            }
            _ => Ok(None)
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
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.connect_in_progress)
                );
            }            ConnectDialogStep::EndpointSelection => {
                // Step 2: Cancel, Back, Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue)
                );
                
                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green)
                );
            }
            ConnectDialogStep::SecurityConfiguration => {
                // Step 3: Cancel, Back, Next
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue)
                );
                
                self.button_manager.add_button(
                    Button::new("next", "Next")
                        .with_hotkey('n')
                        .with_color(ButtonColor::Green)
                );
            }
            ConnectDialogStep::Authentication => {
                // Step 4: Cancel, Back, Connect
                self.button_manager.add_button(
                    Button::new("cancel", "Cancel")
                        .with_hotkey('c')
                        .with_color(ButtonColor::Red)
                );
                
                self.button_manager.add_button(
                    Button::new("back", "Back")
                        .with_hotkey('b')
                        .with_color(ButtonColor::Blue)
                );
                
                self.button_manager.add_button(
                    Button::new("connect", "Connect")
                        .with_hotkey('o')
                        .with_color(ButtonColor::Green)
                        .with_enabled(!self.connect_in_progress)
                );
            }
        }
    }

    // Method to be called from the main UI loop to handle pending operations
    pub async fn handle_pending_operations(&mut self) -> Result<()> {
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
        Ok(())
    }
}
