use super::types::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::rc::Rc;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

impl ConnectScreen {
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Move events from hot buffer to main buffer
        tui_logger::move_events();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main connect area
                Constraint::Length(8), // Connection logs
            ])
            .split(area);
        match self.step {
            ConnectDialogStep::ServerUrl => self.render_server_url_step(f, chunks[0]),
            ConnectDialogStep::EndpointSelection => self.render_endpoint_step(f, chunks[0]),
            ConnectDialogStep::SecurityConfiguration => self.render_security_step(f, chunks[0]),
            ConnectDialogStep::Authentication => self.render_auth_step(f, chunks[0]),
        } // Connection logs with scrolling support - using TuiLoggerWidget
          // Move events from hot buffer to main buffer
        tui_logger::move_events();

        let logger_widget = TuiLoggerWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Long))
            .output_target(true)
            .output_file(false)
            .output_line(false)
            .state(&self.logger_widget_state)
            .block(
                Block::default()
                    .title("Connection Log (PgUp\\PgDown)")
                    .borders(Borders::ALL),
            );

        f.render_widget(logger_widget, chunks[1]);
        // Show connecting popup if discovery or connection is in progress
        if self.connect_in_progress {
            if self.step == ConnectDialogStep::ServerUrl {
                self.render_connecting_popup(f, area, "Discovering Endpoints");
            } else if self.step == ConnectDialogStep::Authentication {
                self.render_connecting_popup(f, area, "Connecting to Server");
            }
        }
    }
    pub fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.step {
            ConnectDialogStep::ServerUrl => {
                "Space - toggle URL override | Esc/Alt+C - Cancel | Enter/Alt+N - Next"
            }
            ConnectDialogStep::EndpointSelection => {
                "↑↓ - Select endpoint | Alt+C - Cancel | Esc/Alt+B - Back | Enter/Alt+N - Next"
            }
            ConnectDialogStep::SecurityConfiguration => {
                "Tab - Next field | Space - Toggle auto-trust | Alt+C - Cancel | Esc/Alt+B - Back | Enter/Alt+N - Next"
            }
            ConnectDialogStep::Authentication => {
                if self.authentication_type == AuthenticationType::UserPassword {
                    "↑↓ - Change auth type | Tab - Next field | Alt+C - Cancel | Esc/Alt+B - Back | Enter/Alt+N - Connect"
                } else {
                    "↑↓ - Change auth type | Alt+C - Cancel | Esc/Alt+B - Back | Enter/Alt+N - Connect"
                }
            }
        };

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help_paragraph, area);
    }
    fn render_connecting_popup(&self, f: &mut Frame, area: Rect, message: &str) {
        // Calculate popup size and position (centered)
        let popup_width = 30;
        let popup_height = 5;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the background area
        f.render_widget(
            Paragraph::new("")
                .style(Style::default().bg(Color::Black))
                .block(Block::default()),
            popup_area,
        );

        // Render the popup with the provided message
        let popup = Paragraph::new(format!("\n{message}"))
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("Please Wait")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White).bg(Color::Blue)),
            );
        f.render_widget(popup, popup_area);
    }
    /// Helper method to create a standard step layout with title, content, and buttons
    pub fn create_step_layout(&self, area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Buttons
            ])
            .split(area)
    }

    /// Helper method to create a standard 3-button layout with margins
    pub fn create_button_layout(&self, area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Left button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Center button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Right button (12 * 1.5 = 18)
                Constraint::Length(2),  // Right margin
            ])
            .split(area)
    }
    /// Helper method to get button rectangles from layout (indices 1, 3, 5)
    pub fn get_button_rects(&self, button_chunks: &[Rect]) -> [Rect; 3] {
        [button_chunks[1], button_chunks[3], button_chunks[5]]
    }
    /// Helper method to create security step layout with conditional trusted store field
    pub fn create_security_layout(&self, area: Rect) -> Rc<[Rect]> {
        let constraints = if self.auto_trust_server_cert {
            // Layout without trusted server store
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // Client Certificate
                Constraint::Length(3), // Client Private Key
                Constraint::Length(1), // Auto-trust checkbox (no border, less space)
                Constraint::Min(0),    // Space
                Constraint::Length(3), // Buttons
            ]
        } else {
            // Layout with trusted server store
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // Client Certificate
                Constraint::Length(3), // Client Private Key
                Constraint::Length(1), // Auto-trust checkbox (no border, less space)
                Constraint::Length(3), // Trusted Server Store (normal height)
                Constraint::Min(0),    // Space
                Constraint::Length(3), // Buttons
            ]
        };

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
    }
    /// Helper method to create the server URL step layout (with checkbox)
    pub fn create_server_url_layout(&self, area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // URL input
                Constraint::Length(3), // Use original URL checkbox
                Constraint::Length(2), // Error message space (always reserved)
                Constraint::Min(0),    // Space
                Constraint::Length(3), // Buttons
            ])
            .split(area)
    }
    /// Common helper method for validation-based styling
    pub fn get_validation_style(is_active: bool, has_validation_error: bool) -> Style {
        if has_validation_error {
            Style::default().fg(Color::Red)
        } else if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        }
    }
}
