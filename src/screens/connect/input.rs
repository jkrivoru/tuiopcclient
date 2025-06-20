use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;
use tui_logger::TuiWidgetEvent;
use crate::client::ConnectionStatus;
use super::types::*;

impl ConnectScreen {
    pub async fn handle_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {
        // Handle button input first
        if let Some(button_id) = self.button_manager.handle_key_input(key, modifiers) {
            return self.handle_button_action(&button_id).await;
        }        match self.step {
            ConnectDialogStep::ServerUrl => self.handle_server_url_input(key, modifiers).await,
            ConnectDialogStep::EndpointSelection => self.handle_endpoint_selection_input(key, modifiers).await,
            ConnectDialogStep::SecurityConfiguration => self.handle_security_input(key, modifiers).await,
            ConnectDialogStep::Authentication => self.handle_authentication_input(key, modifiers).await,
        }
    }    async fn handle_server_url_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {        match key {
            KeyCode::Enter => {
                // Validate URL first
                self.validate_server_url();
                if self.server_url_validation_error.is_none() {
                    // Discover endpoints
                    self.discover_endpoints().await?;
                } else {
                    // Show validation error in log
                    if let Some(ref error) = self.server_url_validation_error {
                        log::error!("URL Validation: {}", error);
                    }
                }
                Ok(None)
            }
            KeyCode::Esc => {
                Ok(Some(ConnectionStatus::Disconnected))
            }
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }            _ => {
                // Let tui-input handle the key event
                if self.input_mode == InputMode::Editing {
                    self.server_url_input.handle_event(&crossterm::event::Event::Key(
                        crossterm::event::KeyEvent::new(key, modifiers)
                    ));
                    // Validate on each keystroke
                    self.validate_server_url();
                }
                Ok(None)
            }
        }
    }

    async fn handle_endpoint_selection_input(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Up => {
                if self.selected_endpoint_index > 0 {
                    self.selected_endpoint_index -= 1;
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.selected_endpoint_index < self.discovered_endpoints.len().saturating_sub(1) {
                    self.selected_endpoint_index += 1;
                }
                Ok(None)
            }            KeyCode::Enter => {
                // Check if security configuration is needed
                if self.needs_security_configuration() {
                    // Move to security configuration step
                    self.step = ConnectDialogStep::SecurityConfiguration;
                    self.active_security_field = SecurityField::ClientCertificate;
                    self.input_mode = InputMode::Editing;
                } else {
                    // Skip security and move directly to authentication step
                    self.step = ConnectDialogStep::Authentication;
                    if self.authentication_type == AuthenticationType::UserPassword {
                        self.active_auth_field = AuthenticationField::Username;
                        self.input_mode = InputMode::Editing;
                    } else {
                        self.input_mode = InputMode::Normal;
                    }
                }
                self.setup_buttons_for_current_step();
                Ok(None)
            }
            KeyCode::Esc => {
                // Go back to URL step
                self.step = ConnectDialogStep::ServerUrl;
                self.setup_buttons_for_current_step();
                Ok(None)
            }
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            _ => Ok(None)
        }
    }

    async fn handle_authentication_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {
        match key {            KeyCode::Up | KeyCode::Down => {
                // Cycle through authentication types
                self.authentication_type = match self.authentication_type {
                    AuthenticationType::Anonymous => AuthenticationType::UserPassword,
                    AuthenticationType::UserPassword => AuthenticationType::X509Certificate,
                    AuthenticationType::X509Certificate => AuthenticationType::Anonymous,
                };
                
                // Set appropriate active field and input mode
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
                Ok(None)
            }            KeyCode::Tab => {
                match self.authentication_type {
                    AuthenticationType::UserPassword => {
                        // Switch between username and password fields
                        self.active_auth_field = match self.active_auth_field {
                            AuthenticationField::Username => AuthenticationField::Password,
                            AuthenticationField::Password => AuthenticationField::Username,
                            _ => AuthenticationField::Username,
                        };
                        self.input_mode = InputMode::Editing;
                    }
                    AuthenticationType::X509Certificate => {
                        // Switch between certificate and private key fields
                        self.active_auth_field = match self.active_auth_field {
                            AuthenticationField::UserCertificate => AuthenticationField::UserPrivateKey,
                            AuthenticationField::UserPrivateKey => AuthenticationField::UserCertificate,
                            _ => AuthenticationField::UserCertificate,
                        };
                        self.input_mode = InputMode::Editing;
                    }
                    AuthenticationType::Anonymous => {
                        // No fields to switch between
                    }
                }
                Ok(None)
            }
            KeyCode::Enter => {
                // Connect with selected settings
                self.connect_with_settings().await
            }
            KeyCode::Esc => {
                // Go back to endpoint selection
                self.step = ConnectDialogStep::EndpointSelection;
                self.input_mode = InputMode::Normal;
                self.setup_buttons_for_current_step();
                Ok(None)
            }            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {
                match self.authentication_type {
                    AuthenticationType::UserPassword => {
                        match self.active_auth_field {
                            AuthenticationField::Username => {
                                self.username_input.handle_event(&crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new(key, modifiers)
                                ));
                            }
                            AuthenticationField::Password => {
                                self.password_input.handle_event(&crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new(key, modifiers)
                                ));
                            }
                            _ => {}
                        }
                    }
                    AuthenticationType::X509Certificate => {
                        match self.active_auth_field {
                            AuthenticationField::UserCertificate => {
                                self.user_certificate_input.handle_event(&crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new(key, modifiers)
                                ));
                            }
                            AuthenticationField::UserPrivateKey => {
                                self.user_private_key_input.handle_event(&crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new(key, modifiers)
                                ));
                            }
                            _ => {}
                        }
                    }
                    AuthenticationType::Anonymous => {
                        // No input fields for anonymous authentication
                    }
                }
                Ok(None)
            }
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            _ => Ok(None)
        }
    }    async fn handle_security_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Tab => {
                // Navigate between fields with Tab/Shift-Tab
                if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    // Shift+Tab: go to previous field
                    match self.active_security_field {
                        SecurityField::ClientCertificate => {
                            if !self.auto_trust_server_cert {
                                self.active_security_field = SecurityField::TrustedServerStore;
                                self.input_mode = InputMode::Editing;
                            } else {
                                self.active_security_field = SecurityField::AutoTrustCheckbox;
                                self.input_mode = InputMode::Normal;
                            }
                        }
                        SecurityField::ClientPrivateKey => {
                            self.active_security_field = SecurityField::ClientCertificate;
                            self.input_mode = InputMode::Editing;
                        }
                        SecurityField::AutoTrustCheckbox => {
                            self.active_security_field = SecurityField::ClientPrivateKey;
                            self.input_mode = InputMode::Editing;
                        }
                        SecurityField::TrustedServerStore => {
                            self.active_security_field = SecurityField::AutoTrustCheckbox;
                            self.input_mode = InputMode::Normal;
                        }
                    }
                } else {
                    // Tab: go to next field
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
                            if !self.auto_trust_server_cert {
                                self.active_security_field = SecurityField::TrustedServerStore;
                                self.input_mode = InputMode::Editing;
                            } else {
                                self.active_security_field = SecurityField::ClientCertificate;
                                self.input_mode = InputMode::Editing;
                            }
                        }
                        SecurityField::TrustedServerStore => {
                            self.active_security_field = SecurityField::ClientCertificate;
                            self.input_mode = InputMode::Editing;
                        }
                    }
                }
                Ok(None)
            }KeyCode::Enter => {
                // Validate security fields before proceeding
                let validation_errors = self.validate_security_fields();
                if !validation_errors.is_empty() {
                    // Log validation errors
                    for error in &validation_errors {
                        log::error!("Security Validation: {}", error);
                    }
                    // Don't proceed if there are validation errors
                    return Ok(None);
                }
                
                // Move to authentication step
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
            KeyCode::Esc => {
                // Go back to endpoint selection
                self.step = ConnectDialogStep::EndpointSelection;
                self.input_mode = InputMode::Normal;
                self.setup_buttons_for_current_step();
                Ok(None)
            }            KeyCode::Char(' ') => {
                // Handle space key
                if self.active_security_field == SecurityField::AutoTrustCheckbox {
                    // Toggle auto-trust checkbox when it's focused
                    self.auto_trust_server_cert = !self.auto_trust_server_cert;
                    // If we enabled auto-trust and we're currently on trusted store field, move away
                    if self.auto_trust_server_cert && self.active_security_field == SecurityField::TrustedServerStore {
                        self.active_security_field = SecurityField::ClientCertificate;
                        self.input_mode = InputMode::Editing;
                    }
                } else if self.input_mode == InputMode::Editing {
                    // Handle space character in text input
                    match self.active_security_field {
                        SecurityField::ClientCertificate => {
                            self.client_certificate_input.handle_event(&crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new(key, modifiers)
                            ));
                        }
                        SecurityField::ClientPrivateKey => {
                            self.client_private_key_input.handle_event(&crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new(key, modifiers)
                            ));
                        }
                        SecurityField::TrustedServerStore => {
                            if !self.auto_trust_server_cert {
                                self.trusted_server_store_input.handle_event(&crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new(key, modifiers)
                                ));
                            }
                        }
                        SecurityField::AutoTrustCheckbox => {
                            // Already handled above
                        }
                    }
                }
                Ok(None)
            }            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {
                // Handle text input for the active field (only when in editing mode)
                if self.input_mode == InputMode::Editing {
                    match self.active_security_field {
                        SecurityField::ClientCertificate => {
                            self.client_certificate_input.handle_event(&crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new(key, modifiers)
                            ));
                        }
                        SecurityField::ClientPrivateKey => {
                            self.client_private_key_input.handle_event(&crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new(key, modifiers)
                            ));
                        }
                        SecurityField::TrustedServerStore => {
                            if !self.auto_trust_server_cert {
                                self.trusted_server_store_input.handle_event(&crossterm::event::Event::Key(
                                    crossterm::event::KeyEvent::new(key, modifiers)
                                ));
                            }
                        }
                        SecurityField::AutoTrustCheckbox => {
                            // Checkbox doesn't handle text input
                        }
                    }
                }
                Ok(None)
            }
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state.transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state.transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state.transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            _ => Ok(None)
        }
    }
}
