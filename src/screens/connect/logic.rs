use super::types::*;
use crate::client::ConnectionStatus;
use crate::components::{Button, ButtonColor};
use anyhow::{Result, anyhow};
use log::{error, info, warn};
use opcua::client::prelude::*;
use opcua::types::{EndpointDescription, MessageSecurityMode, UAString, ByteString, ApplicationDescription};
use opcua::crypto::SecurityPolicy as OpcUaSecurityPolicy;
use tui_input::backend::crossterm::EventHandler;
use std::sync::Arc;
use parking_lot::RwLock;

impl ConnectScreen {
    pub async fn discover_endpoints(&mut self) -> Result<()> {
        info!("Discovering endpoints...");
        
        // Get the server URL
        let url = self.get_server_url();
        if !url.starts_with("opc.tcp://") {
            error!("Invalid OPC UA server URL: must start with 'opc.tcp://'");
            return Err(anyhow!("Invalid URL format"));
        }

        info!("Querying OPC UA server for available endpoints: {}", url);        // Use spawn_blocking to avoid runtime conflicts
        let url_clone = url.clone();
        let endpoints_result = tokio::task::spawn_blocking(move || -> Result<Vec<EndpointDescription>> {
            // Create a simple client for endpoint discovery
            let client_builder = ClientBuilder::new()
                .application_name("OPC UA TUI Client - Discovery")
                .application_uri("urn:opcua-tui-client-discovery")
                .create_sample_keypair(true)
                .trust_server_certs(true)
                .session_retry_limit(1)
                .session_timeout(5000);
                
            let client = client_builder.client().ok_or_else(|| anyhow!("Failed to create discovery client"))?;            // Get endpoints using get_server_endpoints_from_url (instance method)
            match client.get_server_endpoints_from_url(&url_clone) {
                Ok(endpoints) => {
                    info!("Successfully discovered {} endpoints from server", endpoints.len());
                    Ok(endpoints)
                }
                Err(e) => {
                    error!("Failed to discover endpoints from server: {}", e);
                    Err(anyhow!("Failed to discover endpoints: {}", e))
                }
            }
        }).await??;// Convert OPC UA endpoints to our internal format
        if !endpoints_result.is_empty() {
            self.discovered_endpoints = endpoints_result
                .into_iter()
                .filter_map(|endpoint| {
                    // Convert OPC UA security policy and mode to our types
                    let security_policy = match endpoint.security_policy_uri.as_ref() {
                        "http://opcfoundation.org/UA/SecurityPolicy#None" => crate::screens::connect::types::SecurityPolicy::None,
                        "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15" => crate::screens::connect::types::SecurityPolicy::Basic128Rsa15,
                        "http://opcfoundation.org/UA/SecurityPolicy#Basic256" => crate::screens::connect::types::SecurityPolicy::Basic256,
                        "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256" => crate::screens::connect::types::SecurityPolicy::Basic256Sha256,
                        "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep" => crate::screens::connect::types::SecurityPolicy::Aes128Sha256RsaOaep,
                        "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss" => crate::screens::connect::types::SecurityPolicy::Aes256Sha256RsaPss,
                        _ => {
                            warn!("Unknown security policy: {}", endpoint.security_policy_uri);
                            return None; // Skip unknown policies
                        }
                    };

                    let security_mode = match endpoint.security_mode {
                        MessageSecurityMode::None => crate::screens::connect::types::SecurityMode::None,
                        MessageSecurityMode::Sign => crate::screens::connect::types::SecurityMode::Sign,
                        MessageSecurityMode::SignAndEncrypt => crate::screens::connect::types::SecurityMode::SignAndEncrypt,
                        _ => {
                            warn!("Unknown security mode: {:?}", endpoint.security_mode);
                            return None; // Skip unknown modes
                        }
                    };

                    // Create a display name
                    let display_name = match (&security_policy, &security_mode) {
                        (crate::screens::connect::types::SecurityPolicy::None, crate::screens::connect::types::SecurityMode::None) => "None - No Security".to_string(),
                        (policy, mode) => format!("{:?} - {:?}", policy, mode),
                    };                    Some(EndpointInfo {
                        security_policy,
                        security_mode,
                        display_name,
                        original_endpoint: endpoint, // Store the original endpoint
                    })
                })
                .collect();            info!("Processed {} valid endpoints from server", self.discovered_endpoints.len());
        } else {
            error!("Server returned no endpoints");
            return Err(anyhow!("Server returned no endpoints"));
        }

        // Ensure we have at least one valid endpoint
        if self.discovered_endpoints.is_empty() {
            error!("No valid endpoints found after filtering");
            return Err(anyhow!("No valid endpoints found"));
        }        // Log discovered endpoints
        for (i, endpoint) in self.discovered_endpoints.iter().enumerate() {
            info!("Endpoint {}: {}", i + 1, endpoint.display_name);
        }        
        Ok(())
    }

    /// Fallback method to use demo endpoints when real discovery fails
    fn use_demo_endpoints(&mut self) {
        let server_url = self.get_server_url();
        
        self.discovered_endpoints = vec![
            // No Security
            EndpointInfo {
                security_policy: crate::screens::connect::types::SecurityPolicy::None,
                security_mode: crate::screens::connect::types::SecurityMode::None,
                display_name: "None - No Security".to_string(),
                original_endpoint: EndpointDescription {
                    endpoint_url: UAString::from(server_url.clone()),
                    security_mode: MessageSecurityMode::None,
                    security_policy_uri: OpcUaSecurityPolicy::None.to_uri().into(),
                    server_certificate: ByteString::null(),
                    user_identity_tokens: None,
                    transport_profile_uri: UAString::null(),
                    security_level: 0,
                    server: ApplicationDescription::default(),
                },
            },
            // Basic128Rsa15 combinations
            EndpointInfo {
                security_policy: crate::screens::connect::types::SecurityPolicy::Basic128Rsa15,
                security_mode: crate::screens::connect::types::SecurityMode::Sign,
                display_name: "Basic128Rsa15 - Sign Only".to_string(),                original_endpoint: EndpointDescription {
                    endpoint_url: UAString::from(server_url.clone()),
                    security_mode: MessageSecurityMode::Sign,
                    security_policy_uri: OpcUaSecurityPolicy::Basic128Rsa15.to_uri().into(),
                    server_certificate: ByteString::null(),
                    user_identity_tokens: None,
                    transport_profile_uri: UAString::null(),
                    security_level: 1,
                    server: ApplicationDescription::default(),
                },
            },
            EndpointInfo {
                security_policy: crate::screens::connect::types::SecurityPolicy::Basic128Rsa15,
                security_mode: crate::screens::connect::types::SecurityMode::SignAndEncrypt,
                display_name: "Basic128Rsa15 - Sign & Encrypt".to_string(),                original_endpoint: EndpointDescription {
                    endpoint_url: UAString::from(server_url.clone()),
                    security_mode: MessageSecurityMode::SignAndEncrypt,
                    security_policy_uri: OpcUaSecurityPolicy::Basic128Rsa15.to_uri().into(),
                    server_certificate: ByteString::null(),
                    user_identity_tokens: None,
                    transport_profile_uri: UAString::null(),
                    security_level: 2,
                    server: ApplicationDescription::default(),
                },
            },
            // Basic256Sha256 combinations (most common)
            EndpointInfo {
                security_policy: crate::screens::connect::types::SecurityPolicy::Basic256Sha256,
                security_mode: crate::screens::connect::types::SecurityMode::Sign,
                display_name: "Basic256Sha256 - Sign Only".to_string(),                original_endpoint: EndpointDescription {
                    endpoint_url: UAString::from(server_url.clone()),
                    security_mode: MessageSecurityMode::Sign,
                    security_policy_uri: OpcUaSecurityPolicy::Basic256Sha256.to_uri().into(),
                    server_certificate: ByteString::null(),
                    user_identity_tokens: None,
                    transport_profile_uri: UAString::null(),
                    security_level: 3,
                    server: ApplicationDescription::default(),
                },
            },
            EndpointInfo {
                security_policy: crate::screens::connect::types::SecurityPolicy::Basic256Sha256,
                security_mode: crate::screens::connect::types::SecurityMode::SignAndEncrypt,
                display_name: "Basic256Sha256 - Sign & Encrypt".to_string(),                original_endpoint: EndpointDescription {
                    endpoint_url: UAString::from(server_url.clone()),
                    security_mode: MessageSecurityMode::SignAndEncrypt,
                    security_policy_uri: OpcUaSecurityPolicy::Basic256Sha256.to_uri().into(),
                    server_certificate: ByteString::null(),
                    user_identity_tokens: None,
                    transport_profile_uri: UAString::null(),
                    security_level: 4,
                    server: ApplicationDescription::default(),
                },
            },        ];
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
        self.pending_connection = true;        Ok(None) // Return immediately to show the popup
    }

    pub async fn perform_connection(&mut self) -> Result<Option<ConnectionStatus>> {
        info!("Starting connection process...");

        // Get the selected endpoint
        if self.discovered_endpoints.is_empty() {
            error!("No endpoints available for connection");
            return Ok(Some(ConnectionStatus::Error("No endpoints available".to_string())));
        }

        if self.selected_endpoint_index >= self.discovered_endpoints.len() {
            error!("Invalid endpoint selection");
            return Ok(Some(ConnectionStatus::Error("Invalid endpoint selection".to_string())));
        }

        let selected_endpoint = &self.discovered_endpoints[self.selected_endpoint_index];
        let endpoint = selected_endpoint.original_endpoint.clone();

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

        info!("Connecting to endpoint: {} with {}", selected_endpoint.display_name, auth_desc);

        // Prepare authentication identity token
        let identity_token = match self.authentication_type {
            AuthenticationType::Anonymous => IdentityToken::Anonymous,
            AuthenticationType::UserPassword => {
                let username = self.username_input.value().trim();
                let password = self.password_input.value();
                
                if username.is_empty() {
                    error!("Username is required for user/password authentication");
                    return Ok(Some(ConnectionStatus::Error("Username is required".to_string())));
                }
                
                IdentityToken::UserName(username.to_string(), password.to_string())
            }
            AuthenticationType::X509Certificate => {
                let cert_path = self.user_certificate_input.value().trim();
                let key_path = self.user_private_key_input.value().trim();
                
                if cert_path.is_empty() || key_path.is_empty() {
                    error!("Certificate and private key paths are required for X509 authentication");
                    return Ok(Some(ConnectionStatus::Error("Certificate and private key paths are required".to_string())));
                }
                
                // Note: This is simplified - in a real implementation you'd need to load the certificate and key
                // For now, we'll return an error since X509 implementation would require additional complexity
                error!("X509 certificate authentication not yet implemented");
                return Ok(Some(ConnectionStatus::Error("X509 authentication not yet implemented".to_string())));
            }
        };

        // Use spawn_blocking to avoid runtime conflicts with the synchronous OPC UA library
        let identity_token_clone = identity_token.clone();
        let endpoint_clone = endpoint.clone();
        let auto_trust = self.auto_trust_server_cert;
        let client_cert_path = self.client_certificate_input.value().trim().to_string();
        let client_key_path = self.client_private_key_input.value().trim().to_string();

        let connection_result = match tokio::time::timeout(
            tokio::time::Duration::from_secs(15), // 15 second timeout
            tokio::task::spawn_blocking(move || -> Result<(Client, Arc<RwLock<Session>>)> {
                // Create client configuration
                let mut client_builder = ClientBuilder::new()
                    .application_name("OPC UA TUI Client")
                    .application_uri("urn:opcua-tui-client")
                    .session_retry_limit(1) // Reduce retries to fail faster
                    .session_timeout(10000) // 10 second session timeout
                    .session_retry_interval(1000); // 1 second retry interval

                // Configure security based on the selected endpoint
                if endpoint_clone.security_mode != MessageSecurityMode::None {
                    if auto_trust {
                        client_builder = client_builder.trust_server_certs(true);
                    }
                    
                    // If security is required and cert/key paths are provided, use them
                    if !client_cert_path.is_empty() && !client_key_path.is_empty() {
                        info!("Using client certificate: {}", client_cert_path);
                        // Note: In a real implementation, you'd load the certificate and key files
                        // For now, we'll use the sample keypair
                        client_builder = client_builder.create_sample_keypair(true);
                    } else {
                        // Use sample keypair for security
                        client_builder = client_builder.create_sample_keypair(true);
                    }
                } else {
                    // No security required
                    client_builder = client_builder.trust_server_certs(true);
                }

                let mut client = client_builder.client().ok_or_else(|| anyhow!("Failed to create client"))?;
                
                info!("Attempting to connect to endpoint: {}", endpoint_clone.endpoint_url);
                
                // Connect to the server
                let session = client.connect_to_endpoint(endpoint_clone, identity_token_clone)
                    .map_err(|e| anyhow!("Failed to connect to endpoint: {}", e))?;
                
                info!("Successfully connected to OPC UA server");
                Ok((client, session))
            })
        ).await {
            Ok(spawn_result) => match spawn_result {
                Ok(result) => result,
                Err(join_error) => {
                    error!("Connection task failed: {}", join_error);
                    return Ok(Some(ConnectionStatus::Error("Connection task failed".to_string())));
                }
            },
            Err(_timeout) => {
                error!("Connection timed out after 15 seconds");
                return Ok(Some(ConnectionStatus::Error("Connection timed out".to_string())));
            }
        };
        
        let (client, session) = match connection_result {
            Ok(result) => result,
            Err(e) => {
                error!("Connection failed: {}", e);
                return Ok(Some(ConnectionStatus::Error(format!("Connection failed: {}", e))));
            }
        };
        
        // Store the client and session
        self.client = Some(client);
        self.session = Some(session);
        
        info!("OPC UA connection established successfully");
        Ok(Some(ConnectionStatus::Connected))
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
        match self.step {            ConnectDialogStep::ServerUrl => {
                // Validate URL first
                self.validate_server_url();
                if self.server_url_validation_error.is_none() {
                    // Set flags for showing popup and triggering discovery
                    self.connect_in_progress = true;
                    self.pending_discovery = true;
                    // Disable input while discovery is in progress
                    self.input_mode = InputMode::Normal;
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
    }    // Method to be called from the main UI loop to handle pending operations
    pub async fn handle_pending_operations(&mut self) -> Result<Option<ConnectionStatus>> {
        if self.pending_discovery {
            self.pending_discovery = false;

            // Perform the actual discovery
            match self.discover_endpoints().await {
                Ok(()) => {
                    // After successful discovery, hide popup and transition to next step
                    self.connect_in_progress = false;
                    self.step = ConnectDialogStep::EndpointSelection;
                    self.setup_buttons_for_current_step();
                    self.input_mode = InputMode::Normal;
                }
                Err(_) => {
                    // After discovery failure, hide popup and re-enable Server URL input
                    self.connect_in_progress = false;
                    self.input_mode = InputMode::Editing; // Re-enable Server URL input
                    self.setup_buttons_for_current_step(); // Re-enable Next button
                    // Stay on the Server URL step so user can correct the URL and retry
                }
            }
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

    /// Get the connected OPC UA client, if available
    pub fn get_client(&self) -> Option<&Client> {
        self.client.as_ref()
    }

    /// Get the connected OPC UA session, if available
    pub fn get_session(&self) -> Option<&Arc<RwLock<Session>>> {
        self.session.as_ref()
    }

    /// Check if we have an active OPC UA connection
    pub fn is_connected(&self) -> bool {
        self.client.is_some() && self.session.is_some()
    }

    /// Disconnect from the OPC UA server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(session) = &self.session {
            session.write().disconnect();
        }
        
        self.client = None;
        self.session = None;
        
        info!("Disconnected from OPC UA server");
        Ok(())
    }
}
