use super::types::*;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

impl ConnectScreen {
    pub fn render_auth_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Auth type selection
                Constraint::Length(6), // User details (if needed)
                Constraint::Min(0),    // Space (removed help text)
                Constraint::Length(3), // Buttons
            ])
            .split(area);        // Title
        let title_text = format!("Connect to OPC UA Server - Step {}/{}: Authentication", 
                                 self.get_current_step_number(), self.get_total_steps());
        let title = Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);// Authentication type selection
        let auth_items = [
            if self.authentication_type == AuthenticationType::Anonymous {
                "▶ Anonymous (No credentials required)"
            } else {
                "  Anonymous (No credentials required)"
            },
            if self.authentication_type == AuthenticationType::UserPassword {
                "▶ Username & Password"
            } else {
                "  Username & Password"
            },
            if self.authentication_type == AuthenticationType::X509Certificate {
                "▶ X.509 Certificate"
            } else {
                "  X.509 Certificate"
            },
        ];

        let auth_text = auth_items.join("\n");
        let auth_block = Paragraph::new(auth_text)
            .block(
                Block::default()
                    .title("Authentication Method")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(auth_block, chunks[1]); // User details (if username/password or certificate is selected)
        if self.authentication_type == AuthenticationType::UserPassword {
            let user_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Username
                    Constraint::Length(3), // Password
                ])
                .split(chunks[2]); // Username field with validation styling
            let username_style = self.get_field_style(AuthenticationField::Username, "username");

            let width = user_chunks[0].width.max(3) - 3;
            let scroll = self.username_input.visual_scroll(width as usize);
            let username_text = Paragraph::new(self.username_input.value())
                .style(username_style)
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .title("Username")
                        .borders(Borders::ALL)
                        .border_style(self.get_border_style(
                            AuthenticationField::Username,
                            ConnectScreen::has_username_validation_error,
                        )),
                );
            f.render_widget(username_text, user_chunks[0]);

            // Position cursor if editing username
            if self.active_auth_field == AuthenticationField::Username
                && self.input_mode == InputMode::Editing
            {
                let cursor_x = self.username_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(user_chunks[0].x + cursor_x as u16, user_chunks[0].y + 1);
            } // Password field
            let password_style = self.get_field_style(AuthenticationField::Password, "password");

            let width = user_chunks[1].width.max(3) - 3;
            let scroll = self.password_input.visual_scroll(width as usize);
            let password_display = "*".repeat(self.password_input.value().len());
            let password_text = Paragraph::new(password_display)
                .style(password_style)
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .title("Password")
                        .borders(Borders::ALL)
                        .border_style(self.get_border_style(
                            AuthenticationField::Password,
                            ConnectScreen::has_password_validation_error,
                        )),
                );
            f.render_widget(password_text, user_chunks[1]);

            // Position cursor if editing password
            if self.active_auth_field == AuthenticationField::Password
                && self.input_mode == InputMode::Editing
            {
                let cursor_x = self.password_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(user_chunks[1].x + cursor_x as u16, user_chunks[1].y + 1);
            }
        } else if self.authentication_type == AuthenticationType::X509Certificate {
            let cert_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // User Certificate
                    Constraint::Length(3), // User Private Key
                ])
                .split(chunks[2]); // User Certificate field
            let cert_style =
                self.get_field_style(AuthenticationField::UserCertificate, "certificate");

            let width = cert_chunks[0].width.max(3) - 3;
            let scroll = self.user_certificate_input.visual_scroll(width as usize);
            let cert_text = Paragraph::new(self.user_certificate_input.value())
                .style(cert_style)
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .title("User Certificate (.der/.pem)")
                        .borders(Borders::ALL)
                        .border_style(self.get_border_style(
                            AuthenticationField::UserCertificate,
                            ConnectScreen::has_user_certificate_validation_error,
                        )),
                );
            f.render_widget(cert_text, cert_chunks[0]);

            // Position cursor if editing user certificate
            if self.active_auth_field == AuthenticationField::UserCertificate
                && self.input_mode == InputMode::Editing
            {
                let cursor_x = self.user_certificate_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(cert_chunks[0].x + cursor_x as u16, cert_chunks[0].y + 1);
            } // User Private Key field
            let key_style = self.get_field_style(AuthenticationField::UserPrivateKey, "private");

            let width = cert_chunks[1].width.max(3) - 3;
            let scroll = self.user_private_key_input.visual_scroll(width as usize);
            let key_text = Paragraph::new(self.user_private_key_input.value())
                .style(key_style)
                .scroll((0, scroll as u16))
                .block(
                    Block::default()
                        .title("User Private Key (.pem)")
                        .borders(Borders::ALL)
                        .border_style(self.get_border_style(
                            AuthenticationField::UserPrivateKey,
                            ConnectScreen::has_user_private_key_validation_error,
                        )),
                );
            f.render_widget(key_text, cert_chunks[1]);

            // Position cursor if editing user private key
            if self.active_auth_field == AuthenticationField::UserPrivateKey
                && self.input_mode == InputMode::Editing
            {
                let cursor_x = self.user_private_key_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(cert_chunks[1].x + cursor_x as u16, cert_chunks[1].y + 1);
            }
        } // Buttons (3 buttons for step 3) - left, center, right positioning with margins, 50% wider
        let button_chunks = self.create_button_layout(chunks[4]);

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("connect", false);
        } else {
            self.button_manager.set_button_enabled("connect", true);
        } // Render buttons using button manager (use chunks 1, 3, 5 for left/center/right positioning with margins)
        let button_rects = self.get_button_rects(&button_chunks);
        self.button_manager.render_buttons(f, &button_rects);
    }
    /// Helper method to get field style based on active state and validation
    fn get_field_style(&self, field: AuthenticationField, field_name: &str) -> Style {
        let is_active = self.active_auth_field == field && self.input_mode == InputMode::Editing;
        let has_validation_error = self.show_auth_validation
            && self
                .validate_authentication_fields()
                .iter()
                .any(|e| e.contains(field_name));

        Self::get_validation_style(is_active, has_validation_error)
    }
    /// Helper method to get border style based on active state and validation
    fn get_border_style(
        &self,
        field: AuthenticationField,
        has_error_fn: fn(&Self) -> bool,
    ) -> Style {
        if self.active_auth_field == field && self.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else if has_error_fn(self) {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        }
    }
}
