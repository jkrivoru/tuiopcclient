use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use super::types::*;

impl ConnectScreen {    pub(super) fn render_security_step(&mut self, f: &mut Frame, area: Rect) {
        // Dynamic layout based on auto-trust setting
        let constraints = if self.auto_trust_server_cert {
            // Layout without trusted server store
            vec![
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Client Certificate
                Constraint::Length(3),  // Client Private Key
                Constraint::Length(1),  // Auto-trust checkbox (no border, less space)
                Constraint::Min(0),     // Space
                Constraint::Length(3),  // Buttons
            ]
        } else {
            // Layout with trusted server store
            vec![
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // Client Certificate
                Constraint::Length(3),  // Client Private Key
                Constraint::Length(1),  // Auto-trust checkbox (no border, less space)
                Constraint::Length(3),  // Trusted Server Store (normal height)
                Constraint::Min(0),     // Space
                Constraint::Length(3),  // Buttons
            ]
        };
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 3/4: Security Configuration")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Client Certificate input
        let cert_style = if self.active_security_field == SecurityField::ClientCertificate && self.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let cert_width = chunks[1].width.max(3) - 3;
        let cert_scroll = self.client_certificate_input.visual_scroll(cert_width as usize);
        
        let cert_input = Paragraph::new(self.client_certificate_input.value())
            .style(cert_style)
            .scroll((0, cert_scroll as u16))
            .block(Block::default()
                .title("Client Certificate (.der/.pem)")
                .borders(Borders::ALL)
                .border_style(if self.active_security_field == SecurityField::ClientCertificate { 
                    Style::default().fg(Color::Yellow) 
                } else { 
                    Style::default() 
                }));
        
        f.render_widget(cert_input, chunks[1]);
        
        // Position cursor for certificate field
        if self.input_mode == InputMode::Editing && self.active_security_field == SecurityField::ClientCertificate {
            let cursor_x = self.client_certificate_input.visual_cursor().max(cert_scroll) - cert_scroll + 1;
            f.set_cursor(chunks[1].x + cursor_x as u16, chunks[1].y + 1);
        }

        // Client Private Key input
        let key_style = if self.active_security_field == SecurityField::ClientPrivateKey && self.input_mode == InputMode::Editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        
        let key_width = chunks[2].width.max(3) - 3;
        let key_scroll = self.client_private_key_input.visual_scroll(key_width as usize);
        
        let key_input = Paragraph::new(self.client_private_key_input.value())
            .style(key_style)
            .scroll((0, key_scroll as u16))
            .block(Block::default()
                .title("Client Private Key (.pem)")
                .borders(Borders::ALL)
                .border_style(if self.active_security_field == SecurityField::ClientPrivateKey { 
                    Style::default().fg(Color::Yellow) 
                } else { 
                    Style::default() 
                }));
        
        f.render_widget(key_input, chunks[2]);
        
        // Position cursor for private key field
        if self.input_mode == InputMode::Editing && self.active_security_field == SecurityField::ClientPrivateKey {
            let cursor_x = self.client_private_key_input.visual_cursor().max(key_scroll) - key_scroll + 1;
            f.set_cursor(chunks[2].x + cursor_x as u16, chunks[2].y + 1);
        }        // Auto-trust server certificate checkbox
        let checkbox_text = if self.auto_trust_server_cert {
            " ☑ Auto-trust server certificate (Space to toggle)"
        } else {
            " ☐ Auto-trust server certificate (Space to toggle)"
        };
        
        let checkbox_style = if self.active_security_field == SecurityField::AutoTrustCheckbox {
            Style::default().fg(Color::Yellow)  // Highlighted when focused
        } else {
            Style::default().fg(Color::White)
        };
        
        let checkbox = Paragraph::new(checkbox_text)
            .style(checkbox_style);
        
        f.render_widget(checkbox, chunks[3]);        // Trusted Server Store input (only if auto-trust is disabled)
        if !self.auto_trust_server_cert {
            let store_style = if self.active_security_field == SecurityField::TrustedServerStore && self.input_mode == InputMode::Editing {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };
            
            let store_width = chunks[4].width.max(3) - 3;
            let store_scroll = self.trusted_server_store_input.visual_scroll(store_width as usize);
            
            let store_input = Paragraph::new(self.trusted_server_store_input.value())
                .style(store_style)
                .scroll((0, store_scroll as u16))
                .block(Block::default()
                    .title("Trusted Server Certificate Store")
                    .borders(Borders::ALL)
                    .border_style(if self.active_security_field == SecurityField::TrustedServerStore { 
                        Style::default().fg(Color::Yellow) 
                    } else { 
                        Style::default() 
                    }));
            
            f.render_widget(store_input, chunks[4]);
            
            // Position cursor for trusted store field
            if self.input_mode == InputMode::Editing && self.active_security_field == SecurityField::TrustedServerStore {
                let cursor_x = self.trusted_server_store_input.visual_cursor().max(store_scroll) - store_scroll + 1;
                f.set_cursor(chunks[4].x + cursor_x as u16, chunks[4].y + 1);
            }
        }        // Buttons (3 buttons for security step) - left, center, right positioning with margins
        let button_chunk_index = if self.auto_trust_server_cert { 5 } else { 6 };
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Cancel button
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Back button
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Next button
                Constraint::Length(2),  // Right margin
            ])
            .split(chunks[button_chunk_index]);

        // Render buttons using button manager
        let button_rects = &[button_chunks[1], button_chunks[3], button_chunks[5]];
        self.button_manager.render_buttons(f, button_rects);
    }
}
