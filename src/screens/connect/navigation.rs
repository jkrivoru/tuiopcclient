use super::types::*;
use anyhow::Result;
use log::error;

pub struct ConnectStateMachine;

impl ConnectStateMachine {
    pub fn next_step(
        current: ConnectDialogStep,
        needs_security: bool,
    ) -> Option<ConnectDialogStep> {
        match current {
            ConnectDialogStep::ServerUrl => Some(ConnectDialogStep::EndpointSelection),
            ConnectDialogStep::EndpointSelection => {
                if needs_security {
                    Some(ConnectDialogStep::SecurityConfiguration)
                } else {
                    Some(ConnectDialogStep::Authentication)
                }
            }
            ConnectDialogStep::SecurityConfiguration => Some(ConnectDialogStep::Authentication),
            ConnectDialogStep::Authentication => None, // Terminal state
        }
    }

    pub fn previous_step(
        current: ConnectDialogStep,
        needs_security: bool,
    ) -> Option<ConnectDialogStep> {
        match current {
            ConnectDialogStep::ServerUrl => None,
            ConnectDialogStep::EndpointSelection => Some(ConnectDialogStep::ServerUrl),
            ConnectDialogStep::SecurityConfiguration => Some(ConnectDialogStep::EndpointSelection),
            ConnectDialogStep::Authentication => {
                if needs_security {
                    Some(ConnectDialogStep::SecurityConfiguration)
                } else {
                    Some(ConnectDialogStep::EndpointSelection)
                }
            }
        }
    }
}

impl ConnectScreen {
    pub fn advance_to_next_step(&mut self) -> Result<()> {
        match self.step {
            ConnectDialogStep::ServerUrl => {
                self.validate_server_url();
                if self.server_url_validation_error.is_none() {
                    self.connect_in_progress = true;
                    self.pending_discovery = true;
                    self.input_mode = InputMode::Normal;
                } else if let Some(ref error) = self.server_url_validation_error {
                    error!("URL Validation: {error}");
                }
                Ok(())
            }
            ConnectDialogStep::EndpointSelection => {
                if let Some(next_step) = ConnectStateMachine::next_step(
                    self.step.clone(),
                    self.needs_security_configuration(),
                ) {
                    self.step = next_step.clone();
                    match next_step {
                        ConnectDialogStep::SecurityConfiguration => {
                            self.active_security_field = SecurityField::ClientCertificate;
                            self.input_mode = InputMode::Editing;
                            self.show_security_validation = false;
                        }
                        ConnectDialogStep::Authentication => {
                            self.show_auth_validation = false;
                            self.setup_authentication_fields();
                        }
                        _ => {}
                    }
                    self.setup_buttons_for_current_step();
                }
                Ok(())
            }
            ConnectDialogStep::SecurityConfiguration => {
                self.show_security_validation = true;
                let validation_errors = self.validate_security_fields();
                if !validation_errors.is_empty() {
                    for error in &validation_errors {
                        error!("Security Validation: {error}");
                    }
                    return Ok(());
                }

                self.step = ConnectDialogStep::Authentication;
                self.show_auth_validation = false;
                self.setup_authentication_fields();
                self.setup_buttons_for_current_step();
                Ok(())
            }
            ConnectDialogStep::Authentication => Ok(()),
        }
    }

    pub fn navigate_back_from_auth(&mut self) {
        if let Some(prev_step) = ConnectStateMachine::previous_step(
            self.step.clone(),
            self.needs_security_configuration(),
        ) {
            self.step = prev_step.clone();
            match prev_step {
                ConnectDialogStep::SecurityConfiguration => {
                    self.active_security_field = SecurityField::ClientCertificate;
                    self.input_mode = InputMode::Editing;
                    self.show_security_validation = false;
                }
                ConnectDialogStep::EndpointSelection => {
                    self.input_mode = InputMode::Normal;
                }
                _ => {}
            }
            self.show_auth_validation = false;
            self.setup_buttons_for_current_step();
        }
    }

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

    pub fn cycle_authentication_type(&mut self) {
        self.authentication_type = match self.authentication_type {
            AuthenticationType::Anonymous => AuthenticationType::UserPassword,
            AuthenticationType::UserPassword => AuthenticationType::X509Certificate,
            AuthenticationType::X509Certificate => AuthenticationType::Anonymous,
        };
        self.setup_authentication_fields();
    }

    pub fn cycle_authentication_type_backward(&mut self) {
        self.authentication_type = match self.authentication_type {
            AuthenticationType::Anonymous => AuthenticationType::X509Certificate,
            AuthenticationType::UserPassword => AuthenticationType::Anonymous,
            AuthenticationType::X509Certificate => AuthenticationType::UserPassword,
        };
        self.setup_authentication_fields();
    }

    fn get_next_security_field_from_checkbox(&self) -> (SecurityField, InputMode) {
        if !self.auto_trust_server_cert {
            (SecurityField::TrustedServerStore, InputMode::Editing)
        } else {
            (SecurityField::ClientCertificate, InputMode::Editing)
        }
    }

    fn get_prev_security_field_to_checkbox(&self) -> (SecurityField, InputMode) {
        if !self.auto_trust_server_cert {
            (SecurityField::TrustedServerStore, InputMode::Editing)
        } else {
            (SecurityField::AutoTrustCheckbox, InputMode::Normal)
        }
    }
}
