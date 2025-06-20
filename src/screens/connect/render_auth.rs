use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use super::types::*;

impl ConnectScreen {
    pub fn render_auth_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(5),  // Auth type selection
                Constraint::Length(6),  // User details (if needed)
                Constraint::Min(0),     // Space (removed help text)
                Constraint::Length(3),  // Buttons
            ])
            .split(area);        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 4/4: Authentication")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);        // Authentication type selection
        let auth_items = vec![
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
            }
        ];

        let auth_text = auth_items.join("\n");
        let auth_block = Paragraph::new(auth_text)
            .block(Block::default()
                .title("Authentication Method")
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(auth_block, chunks[1]);        // User details (if username/password or certificate is selected)
        if self.authentication_type == AuthenticationType::UserPassword {
            let user_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Username
                    Constraint::Length(3),  // Password
                ])
                .split(chunks[2]);

            // Username field
            let username_style = if self.active_auth_field == AuthenticationField::Username && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let width = user_chunks[0].width.max(3) - 3;
            let scroll = self.username_input.visual_scroll(width as usize);
              let username_text = Paragraph::new(self.username_input.value())
                .style(username_style)
                .scroll((0, scroll as u16))
                .block(Block::default()
                    .title("Username")
                    .borders(Borders::ALL)
                    .border_style(
                        if self.active_auth_field == AuthenticationField::Username && self.input_mode == InputMode::Editing {
                            Style::default().fg(Color::Yellow)
                        } else if self.has_username_validation_error() {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default()
                        }
                    ));
            f.render_widget(username_text, user_chunks[0]);
            
            // Position cursor if editing username
            if self.active_auth_field == AuthenticationField::Username && self.input_mode == InputMode::Editing {
                let cursor_x = self.username_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(user_chunks[0].x + cursor_x as u16, user_chunks[0].y + 1);
            }

            // Password field
            let password_style = if self.active_auth_field == AuthenticationField::Password && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let width = user_chunks[1].width.max(3) - 3;
            let scroll = self.password_input.visual_scroll(width as usize);
            let password_display = "*".repeat(self.password_input.value().len());
              let password_text = Paragraph::new(password_display)
                .style(password_style)
                .scroll((0, scroll as u16))
                .block(Block::default()
                    .title("Password")
                    .borders(Borders::ALL)
                    .border_style(
                        if self.active_auth_field == AuthenticationField::Password && self.input_mode == InputMode::Editing {
                            Style::default().fg(Color::Yellow)
                        } else if self.has_password_validation_error() {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default()
                        }
                    ));
            f.render_widget(password_text, user_chunks[1]);
            
            // Position cursor if editing password
            if self.active_auth_field == AuthenticationField::Password && self.input_mode == InputMode::Editing {
                let cursor_x = self.password_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(user_chunks[1].x + cursor_x as u16, user_chunks[1].y + 1);
            }
        } else if self.authentication_type == AuthenticationType::X509Certificate {
            let cert_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // User Certificate
                    Constraint::Length(3),  // User Private Key
                ])
                .split(chunks[2]);

            // User Certificate field
            let cert_style = if self.active_auth_field == AuthenticationField::UserCertificate && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let width = cert_chunks[0].width.max(3) - 3;
            let scroll = self.user_certificate_input.visual_scroll(width as usize);
              let cert_text = Paragraph::new(self.user_certificate_input.value())
                .style(cert_style)
                .scroll((0, scroll as u16))
                .block(Block::default()
                    .title("User Certificate (.der/.pem)")
                    .borders(Borders::ALL)
                    .border_style(
                        if self.active_auth_field == AuthenticationField::UserCertificate && self.input_mode == InputMode::Editing {
                            Style::default().fg(Color::Yellow)
                        } else if self.has_user_certificate_validation_error() {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default()
                        }
                    ));
            f.render_widget(cert_text, cert_chunks[0]);
            
            // Position cursor if editing user certificate
            if self.active_auth_field == AuthenticationField::UserCertificate && self.input_mode == InputMode::Editing {
                let cursor_x = self.user_certificate_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(cert_chunks[0].x + cursor_x as u16, cert_chunks[0].y + 1);
            }

            // User Private Key field
            let key_style = if self.active_auth_field == AuthenticationField::UserPrivateKey && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let width = cert_chunks[1].width.max(3) - 3;
            let scroll = self.user_private_key_input.visual_scroll(width as usize);
              let key_text = Paragraph::new(self.user_private_key_input.value())
                .style(key_style)
                .scroll((0, scroll as u16))
                .block(Block::default()
                    .title("User Private Key (.pem)")
                    .borders(Borders::ALL)
                    .border_style(
                        if self.active_auth_field == AuthenticationField::UserPrivateKey && self.input_mode == InputMode::Editing {
                            Style::default().fg(Color::Yellow)
                        } else if self.has_user_private_key_validation_error() {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default()
                        }
                    ));
            f.render_widget(key_text, cert_chunks[1]);
            
            // Position cursor if editing user private key
            if self.active_auth_field == AuthenticationField::UserPrivateKey && self.input_mode == InputMode::Editing {
                let cursor_x = self.user_private_key_input.visual_cursor().max(scroll) - scroll + 1;
                f.set_cursor(cert_chunks[1].x + cursor_x as u16, cert_chunks[1].y + 1);
            }
        }

        // Buttons (3 buttons for step 3) - left, center, right positioning with margins, 50% wider
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Back button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Connect button (12 * 1.5 = 18)
                Constraint::Length(2),  // Right margin
            ])
            .split(chunks[4]);

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("connect", false);
        } else {
            self.button_manager.set_button_enabled("connect", true);
        }

        // Render buttons using button manager (use chunks 1, 3, 5 for left/center/right positioning with margins)
        let button_rects = &[button_chunks[1], button_chunks[3], button_chunks[5]];
        self.button_manager.render_buttons(f, button_rects);
    }
}
