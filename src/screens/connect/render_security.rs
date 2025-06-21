use super::types::*;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

impl ConnectScreen {
    pub(super) fn render_security_step(&mut self, f: &mut Frame, area: Rect) {
        // Dynamic layout based on auto-trust setting
        let chunks = self.create_security_layout(area);        // Title
        let title_text = format!("Connect to OPC UA Server - Step {}/{}: Security Configuration", 
                                 self.get_current_step_number(), self.get_total_steps());
        let title = Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);// Client Certificate input
        let cert_style =
            self.get_security_field_style(SecurityField::ClientCertificate, "certificate");

        let cert_width = chunks[1].width.max(3) - 3;
        let cert_scroll = self
            .client_certificate_input
            .visual_scroll(cert_width as usize);
        let cert_input = Paragraph::new(self.client_certificate_input.value())
            .style(cert_style)
            .scroll((0, cert_scroll as u16))
            .block(
                Block::default()
                    .title("Client Certificate (.der/.pem)")
                    .borders(Borders::ALL)
                    .border_style(self.get_security_border_style(
                        SecurityField::ClientCertificate,
                        ConnectScreen::has_certificate_validation_error,
                    )),
            );

        f.render_widget(cert_input, chunks[1]);

        // Position cursor for certificate field
        if self.input_mode == InputMode::Editing
            && self.active_security_field == SecurityField::ClientCertificate
        {
            let cursor_x = self
                .client_certificate_input
                .visual_cursor()
                .max(cert_scroll)
                - cert_scroll
                + 1;
            f.set_cursor(chunks[1].x + cursor_x as u16, chunks[1].y + 1);
        } // Client Private Key input
        let key_style = self.get_security_field_style(SecurityField::ClientPrivateKey, "key");

        let key_width = chunks[2].width.max(3) - 3;
        let key_scroll = self
            .client_private_key_input
            .visual_scroll(key_width as usize);
        let key_input = Paragraph::new(self.client_private_key_input.value())
            .style(key_style)
            .scroll((0, key_scroll as u16))
            .block(
                Block::default()
                    .title("Client Private Key (.pem)")
                    .borders(Borders::ALL)
                    .border_style(self.get_security_border_style(
                        SecurityField::ClientPrivateKey,
                        ConnectScreen::has_private_key_validation_error,
                    )),
            );

        f.render_widget(key_input, chunks[2]);

        // Position cursor for private key field
        if self.input_mode == InputMode::Editing
            && self.active_security_field == SecurityField::ClientPrivateKey
        {
            let cursor_x = self
                .client_private_key_input
                .visual_cursor()
                .max(key_scroll)
                - key_scroll
                + 1;
            f.set_cursor(chunks[2].x + cursor_x as u16, chunks[2].y + 1);
        } // Auto-trust server certificate checkbox
        let checkbox_text = if self.auto_trust_server_cert {
            " ☑ Auto-trust server certificate (Space to toggle)"
        } else {
            " ☐ Auto-trust server certificate (Space to toggle)"
        };

        let checkbox_style = if self.active_security_field == SecurityField::AutoTrustCheckbox {
            Style::default().fg(Color::Yellow) // Highlighted when focused
        } else {
            Style::default().fg(Color::White)
        };

        let checkbox = Paragraph::new(checkbox_text).style(checkbox_style);

        f.render_widget(checkbox, chunks[3]); // Trusted Server Store input (only if auto-trust is disabled)
        if !self.auto_trust_server_cert {
            let store_style =
                self.get_security_field_style(SecurityField::TrustedServerStore, "store");

            let store_width = chunks[4].width.max(3) - 3;
            let store_scroll = self
                .trusted_server_store_input
                .visual_scroll(store_width as usize);
            let store_input = Paragraph::new(self.trusted_server_store_input.value())
                .style(store_style)
                .scroll((0, store_scroll as u16))
                .block(
                    Block::default()
                        .title("Trusted Server Certificate Store")
                        .borders(Borders::ALL)
                        .border_style(self.get_security_border_style(
                            SecurityField::TrustedServerStore,
                            ConnectScreen::has_trusted_store_validation_error,
                        )),
                );

            f.render_widget(store_input, chunks[4]);

            // Position cursor for trusted store field
            if self.input_mode == InputMode::Editing
                && self.active_security_field == SecurityField::TrustedServerStore
            {
                let cursor_x = self
                    .trusted_server_store_input
                    .visual_cursor()
                    .max(store_scroll)
                    - store_scroll
                    + 1;
                f.set_cursor(chunks[4].x + cursor_x as u16, chunks[4].y + 1);
            }
        } // Buttons (3 buttons for security step) - left, center, right positioning with margins
        let button_chunk_index = if self.auto_trust_server_cert { 5 } else { 6 };
        let button_chunks = self.create_button_layout(chunks[button_chunk_index]); // Render buttons using button manager
        let button_rects = self.get_button_rects(&button_chunks);
        self.button_manager.render_buttons(f, &button_rects);
    }
    /// Helper method to get security field style based on active state and validation
    fn get_security_field_style(&self, field: SecurityField, field_name: &str) -> Style {
        let is_active =
            self.active_security_field == field && self.input_mode == InputMode::Editing;
        let has_validation_error = self.show_security_validation
            && self
                .validate_security_fields()
                .iter()
                .any(|e| e.contains(field_name));

        Self::get_validation_style(is_active, has_validation_error)
    }

    /// Helper method to get security border style based on active state and validation
    fn get_security_border_style(
        &self,
        field: SecurityField,
        has_error_fn: fn(&Self) -> bool,
    ) -> Style {
        if self.active_security_field == field && self.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else if has_error_fn(self) {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        }
    }
}
