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
        
        // Simulate endpoint discovery with a longer delay to show popup
        time::sleep(Duration::from_millis(1500)).await;
          // Mock discovered endpoints based on URL
        self.discovered_endpoints = vec![
            EndpointInfo {
                endpoint_url: self.get_server_url(),
                security_policy: SecurityPolicy::None,
                security_mode: SecurityMode::None,
                display_name: "None - No Security".to_string(),
            },
            EndpointInfo {
                endpoint_url: self.get_server_url(),
                security_policy: SecurityPolicy::Basic256Sha256,
                security_mode: SecurityMode::SignAndEncrypt,
                display_name: "Basic256Sha256 - Sign & Encrypt".to_string(),
            },
        ];
        
        info!("Found {} endpoints", self.discovered_endpoints.len());
        
        Ok(())
    }

    pub async fn connect_with_settings(&mut self) -> Result<Option<ConnectionStatus>> {
        self.connect_in_progress = true;
        
        let auth_desc = match self.authentication_type {
            AuthenticationType::Anonymous => "Anonymous".to_string(),
            AuthenticationType::UserPassword => format!("User: {}", self.username_input.value()),
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
            "cancel" => Ok(Some(ConnectionStatus::Disconnected)),
            "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl => {
                        // Set flags for showing popup and triggering discovery
                        self.connect_in_progress = true;
                        self.pending_discovery = true;
                        Ok(None) // Return immediately to show the popup
                    }
                    ConnectDialogStep::EndpointSelection => {
                        self.step = ConnectDialogStep::Authentication;
                        if self.authentication_type == AuthenticationType::UserPassword {
                            self.active_auth_field = AuthenticationField::Username;
                            self.input_mode = InputMode::Editing;
                        } else {
                            self.input_mode = InputMode::Normal;
                        }
                        self.setup_buttons_for_current_step();
                        Ok(None)
                    }
                    _ => Ok(None)
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
                    ConnectDialogStep::Authentication => {
                        self.step = ConnectDialogStep::EndpointSelection;
                        self.input_mode = InputMode::Normal;
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
            }
            ConnectDialogStep::EndpointSelection => {
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
            ConnectDialogStep::Authentication => {
                // Step 3: Cancel, Back, Connect
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
