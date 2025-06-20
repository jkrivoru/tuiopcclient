use super::types::*;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

impl ConnectScreen {
    pub fn handle_mouse_down(&mut self, column: u16, row: u16) -> bool {
        self.button_manager.handle_mouse_down(column, row)
    }

    pub fn handle_mouse_up(&mut self, column: u16, row: u16) -> Option<String> {
        self.button_manager.handle_mouse_up(column, row)
    }

    /// Handle mouse click events for the current connect step
    pub fn handle_mouse_click(&mut self, column: u16, row: u16, area: Rect) -> bool {
        match self.step {
            ConnectDialogStep::ServerUrl => self.handle_mouse_click_server_url(column, row, area),
            ConnectDialogStep::EndpointSelection => {
                self.handle_mouse_click_endpoint(column, row, area)
            }
            ConnectDialogStep::SecurityConfiguration => {
                self.handle_mouse_click_security(column, row, area)
            }
            ConnectDialogStep::Authentication => {
                self.handle_mouse_click_authentication(column, row, area)
            }
        }
    }

    /// Handle mouse clicks in the server URL step
    fn handle_mouse_click_server_url(&mut self, column: u16, row: u16, area: Rect) -> bool {
        let chunks = self.create_step_layout(area);
        
        // Check if click is in the server URL input area (chunks[1])
        if self.is_point_in_rect(column, row, chunks[1]) {
            self.input_mode = InputMode::Editing;
            return true;
        }
        
        false
    }

    /// Handle mouse clicks in the endpoint selection step
    fn handle_mouse_click_endpoint(&mut self, column: u16, row: u16, area: Rect) -> bool {
        let chunks = self.create_step_layout(area);
        
        // Check if click is in the endpoint list area (chunks[1])
        if self.is_point_in_rect(column, row, chunks[1]) {
            // Calculate which endpoint was clicked
            let list_area = chunks[1];
            let click_row = row.saturating_sub(list_area.y + 1); // Subtract border
            
            // Calculate visible items
            let list_height = list_area.height.saturating_sub(2) as usize;
            let visible_items = list_height;
            
            if (click_row as usize) < visible_items && (click_row as usize) < self.discovered_endpoints.len() {
                let clicked_index = self.endpoint_scroll_offset + click_row as usize;
                if clicked_index < self.discovered_endpoints.len() {
                    self.selected_endpoint_index = clicked_index;
                    self.update_endpoint_scroll(visible_items);
                    return true;
                }
            }
        }
        
        false
    }

    /// Handle mouse clicks in the security configuration step
    fn handle_mouse_click_security(&mut self, column: u16, row: u16, area: Rect) -> bool {
        let chunks = self.create_security_layout(area);
        
        // Client Certificate field (chunks[1])
        if self.is_point_in_rect(column, row, chunks[1]) {
            self.active_security_field = SecurityField::ClientCertificate;
            self.input_mode = InputMode::Editing;
            return true;
        }
        
        // Client Private Key field (chunks[2])
        if self.is_point_in_rect(column, row, chunks[2]) {
            self.active_security_field = SecurityField::ClientPrivateKey;
            self.input_mode = InputMode::Editing;
            return true;
        }
        
        // Auto-trust checkbox (chunks[3])
        if self.is_point_in_rect(column, row, chunks[3]) {
            self.active_security_field = SecurityField::AutoTrustCheckbox;
            self.input_mode = InputMode::Normal;
            // Toggle the checkbox
            self.auto_trust_server_cert = !self.auto_trust_server_cert;
            // If we enabled auto-trust and we're currently on trusted store field, move away
            if self.auto_trust_server_cert && self.active_security_field == SecurityField::TrustedServerStore {
                self.active_security_field = SecurityField::ClientCertificate;
                self.input_mode = InputMode::Editing;
            }
            return true;
        }
        
        // Trusted Server Store field (chunks[4]) - only if auto-trust is disabled
        if !self.auto_trust_server_cert && chunks.len() > 4 && self.is_point_in_rect(column, row, chunks[4]) {
            self.active_security_field = SecurityField::TrustedServerStore;
            self.input_mode = InputMode::Editing;
            return true;
        }
        
        false
    }

    /// Handle mouse clicks in the authentication step
    fn handle_mouse_click_authentication(&mut self, column: u16, row: u16, area: Rect) -> bool {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Auth type selection
                Constraint::Length(6), // User details (if needed)
                Constraint::Min(0),    // Space
                Constraint::Length(3), // Buttons
            ])
            .split(area);
        
        // Authentication method selection (chunks[1])
        if self.is_point_in_rect(column, row, chunks[1]) {
            let click_row = row.saturating_sub(chunks[1].y + 1); // Subtract border
            
            match click_row {
                0 => {
                    self.authentication_type = AuthenticationType::Anonymous;
                    self.input_mode = InputMode::Normal;
                    return true;
                }
                1 => {
                    self.authentication_type = AuthenticationType::UserPassword;
                    self.active_auth_field = AuthenticationField::Username;
                    self.input_mode = InputMode::Editing;
                    return true;
                }
                2 => {
                    self.authentication_type = AuthenticationType::X509Certificate;
                    self.active_auth_field = AuthenticationField::UserCertificate;
                    self.input_mode = InputMode::Editing;
                    return true;
                }
                _ => {}
            }
        }
        
        // User details fields (chunks[2]) - only if authentication type requires them
        if self.is_point_in_rect(column, row, chunks[2]) {
            match self.authentication_type {
                AuthenticationType::UserPassword => {
                    let user_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // Username
                            Constraint::Length(3), // Password
                        ])
                        .split(chunks[2]);
                    
                    if self.is_point_in_rect(column, row, user_chunks[0]) {
                        self.active_auth_field = AuthenticationField::Username;
                        self.input_mode = InputMode::Editing;
                        return true;
                    } else if self.is_point_in_rect(column, row, user_chunks[1]) {
                        self.active_auth_field = AuthenticationField::Password;
                        self.input_mode = InputMode::Editing;
                        return true;
                    }
                }
                AuthenticationType::X509Certificate => {
                    let cert_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // User Certificate
                            Constraint::Length(3), // User Private Key
                        ])
                        .split(chunks[2]);
                    
                    if self.is_point_in_rect(column, row, cert_chunks[0]) {
                        self.active_auth_field = AuthenticationField::UserCertificate;
                        self.input_mode = InputMode::Editing;
                        return true;
                    } else if self.is_point_in_rect(column, row, cert_chunks[1]) {
                        self.active_auth_field = AuthenticationField::UserPrivateKey;
                        self.input_mode = InputMode::Editing;
                        return true;
                    }
                }
                AuthenticationType::Anonymous => {
                    // No input fields for anonymous authentication
                }
            }
        }
        
        false
    }

    /// Helper method to check if a point is within a rectangle
    fn is_point_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}
