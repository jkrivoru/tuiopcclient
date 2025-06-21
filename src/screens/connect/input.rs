use super::types::*;
use crate::client::ConnectionStatus;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;
use tui_logger::TuiWidgetEvent;

impl ConnectScreen {
    pub async fn handle_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        // Handle button input first
        if let Some(button_id) = self.button_manager.handle_key_input(key, modifiers) {
            return self.handle_button_action(&button_id).await;
        }
        match self.step {
            ConnectDialogStep::ServerUrl => self.handle_server_url_input(key, modifiers).await,
            ConnectDialogStep::EndpointSelection => {
                self.handle_endpoint_selection_input(key, modifiers).await
            }
            ConnectDialogStep::SecurityConfiguration => {
                self.handle_security_input(key, modifiers).await
            }
            ConnectDialogStep::Authentication => {
                self.handle_authentication_input(key, modifiers).await
            }
        }
    }
    async fn handle_server_url_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Enter => {
                // Use unified method for consistent behavior with button clicks
                self.advance_to_next_step()?;
                Ok(None)
            }
            KeyCode::Esc => Ok(Some(ConnectionStatus::Disconnected)),
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state
                    .transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state
                    .transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state
                        .transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state
                    .transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            KeyCode::Char(' ') => {
                // Toggle "Use Original URL" checkbox with spacebar
                self.use_original_url = !self.use_original_url;
                Ok(None)
            }
            _ => {
                // Let tui-input handle the key event
                if self.input_mode == InputMode::Editing {
                    self.server_url_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                    // Validate on each keystroke
                    self.validate_server_url();
                }
                Ok(None)
            }
        }
    }

    async fn handle_endpoint_selection_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {            KeyCode::Up => {
                if self.discovered_endpoints.is_empty() {
                    // No endpoints to navigate
                } else if self.selected_endpoint_index > 0 {
                    self.selected_endpoint_index -= 1;
                } else {
                    // Cycle to the bottom when at the top
                    self.selected_endpoint_index = self.discovered_endpoints.len() - 1;
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.discovered_endpoints.is_empty() {
                    // No endpoints to navigate
                } else if self.selected_endpoint_index < self.discovered_endpoints.len() - 1 {
                    self.selected_endpoint_index += 1;
                } else {
                    // Cycle to the top when at the bottom
                    self.selected_endpoint_index = 0;
                }
                Ok(None)
            }
            KeyCode::Enter => {
                // Use unified method for consistent behavior with button clicks
                self.advance_to_next_step()?;
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
                self.logger_widget_state
                    .transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state
                    .transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state
                        .transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state
                    .transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn handle_authentication_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {        match key {
            KeyCode::Up => {
                // Cycle through authentication types backward (up)
                self.cycle_authentication_type_backward();
                Ok(None)
            }
            KeyCode::Down => {
                // Cycle through authentication types forward (down)
                self.cycle_authentication_type();
                Ok(None)
            }
            KeyCode::Tab => {
                self.navigate_auth_fields_forward();
                Ok(None)
            }            KeyCode::Enter => {
                // Connect with selected settings
                self.connect_with_settings().await
            }
            KeyCode::Char('n') if modifiers.contains(KeyModifiers::ALT) => {
                // Alt+N also connects (same as Enter)
                self.connect_with_settings().await
            }
            KeyCode::Esc => {
                // Go back to previous step
                self.navigate_back_from_auth();
                Ok(None)
            }
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {
                self.handle_auth_field_input(key, modifiers);
                Ok(None)
            }
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state
                    .transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state
                    .transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state
                        .transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state
                    .transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    async fn handle_security_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Tab => {
                // Navigate between fields with Tab/Shift-Tab
                if modifiers.contains(KeyModifiers::SHIFT) {
                    self.navigate_security_fields_backward();
                } else {
                    self.navigate_security_fields_forward();
                }
                Ok(None)
            }
            KeyCode::Enter => {
                // Use unified method for consistent behavior with button clicks
                self.advance_to_next_step()?;
                Ok(None)
            }
            KeyCode::Esc => {
                // Go back to endpoint selection
                self.step = ConnectDialogStep::EndpointSelection;
                self.input_mode = InputMode::Normal;
                // Reset validation highlighting when going back
                self.show_security_validation = false;
                self.setup_buttons_for_current_step();
                Ok(None)
            }
            KeyCode::Char(' ') => {
                // Handle space key
                if self.active_security_field == SecurityField::AutoTrustCheckbox {
                    // Toggle auto-trust checkbox when it's focused
                    self.auto_trust_server_cert = !self.auto_trust_server_cert;
                    // If we enabled auto-trust and we're currently on trusted store field, move away
                    if self.auto_trust_server_cert
                        && self.active_security_field == SecurityField::TrustedServerStore
                    {
                        self.active_security_field = SecurityField::ClientCertificate;
                        self.input_mode = InputMode::Editing;
                    }
                } else if self.input_mode == InputMode::Editing {
                    // Handle space character in text input
                    self.handle_security_field_input(key, modifiers);
                }
                Ok(None)
            }
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right => {
                // Handle text input for the active field (only when in editing mode)
                if self.input_mode == InputMode::Editing {
                    self.handle_security_field_input(key, modifiers);
                }
                Ok(None)
            }
            KeyCode::PageUp => {
                // Scroll connection log up
                self.logger_widget_state
                    .transition(TuiWidgetEvent::PrevPageKey);
                Ok(None)
            }
            KeyCode::PageDown => {
                // Scroll connection log down
                self.logger_widget_state
                    .transition(TuiWidgetEvent::NextPageKey);
                Ok(None)
            }
            KeyCode::Home => {
                // Go to the beginning - scroll up multiple pages
                for _ in 0..10 {
                    self.logger_widget_state
                        .transition(TuiWidgetEvent::PrevPageKey);
                }
                Ok(None)
            }
            KeyCode::End => {
                // Go to the end (latest messages) - exit page mode
                self.logger_widget_state
                    .transition(TuiWidgetEvent::EscapeKey);
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
