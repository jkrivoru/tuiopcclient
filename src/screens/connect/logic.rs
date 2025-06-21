use super::types::*;
use crate::client::ConnectionStatus;
use anyhow::Result;
use tui_input::backend::crossterm::EventHandler;

impl ConnectScreen {
    pub async fn handle_pending_operations(&mut self) -> Result<Option<ConnectionStatus>> {
        if self.pending_discovery {
            self.pending_discovery = false;

            match self.discover_endpoints().await {
                Ok(()) => {
                    self.connect_in_progress = false;
                    self.step = ConnectDialogStep::EndpointSelection;
                    self.setup_buttons_for_current_step();
                    self.input_mode = InputMode::Normal;
                }
                Err(_) => {
                    self.connect_in_progress = false;
                    self.input_mode = InputMode::Editing;
                    self.setup_buttons_for_current_step();
                }
            }
        }

        if self.pending_connection {
            self.pending_connection = false;
            let connection_result = self.perform_connection().await?;
            self.connect_in_progress = false;
            return Ok(connection_result);
        }

        Ok(None)
    }

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
            AuthenticationType::Anonymous => {}
        }
    }

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
            SecurityField::AutoTrustCheckbox => {}
        }
    }

    pub fn get_selected_endpoint(&self) -> Option<&EndpointInfo> {
        if self.discovered_endpoints.is_empty() {
            return None;
        }
        
        let index = self.selected_endpoint_index.min(self.discovered_endpoints.len() - 1);
        self.discovered_endpoints.get(index)
    }
}
