use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_logger::{TuiLoggerWidget, TuiLoggerLevelOutput};
use super::types::*;

impl ConnectScreen {
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Move events from hot buffer to main buffer
        tui_logger::move_events();
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // Main connect area
                Constraint::Length(8),  // Connection logs
            ])
            .split(area);

        match self.step {
            ConnectDialogStep::ServerUrl => self.render_server_url_step(f, chunks[0]),
            ConnectDialogStep::EndpointSelection => self.render_endpoint_step(f, chunks[0]),
            ConnectDialogStep::Authentication => self.render_auth_step(f, chunks[0]),
        }

        // Connection logs with scrolling support
        let logger_widget = TuiLoggerWidget::default()
            .block(
                Block::default()
                    .title("Connection Log")
                    .borders(Borders::ALL)
            )
            // Custom formatting: datetime + severity only, no callstack
            .output_timestamp(Some("%Y-%m-%d %H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Long))
            .output_target(false)  // Disable target/module name
            .output_file(false)    // Disable file name
            .output_line(false)    // Disable line number
            .output_separator(' ') // Use space instead of colon
            // Color coding: Info - standard (white), Warning - yellow, Error - red
            .style_info(Style::default().fg(Color::White))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::DarkGray))
            .style_trace(Style::default().fg(Color::Gray))
            .state(&self.logger_widget_state);        f.render_widget(logger_widget, chunks[1]);
        
        // Show connecting popup if discovery is in progress
        if self.connect_in_progress && self.step == ConnectDialogStep::ServerUrl {
            self.render_connecting_popup(f, area);
        }
    }pub fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.step {
            ConnectDialogStep::ServerUrl => {
                "PageUp/PageDown - scroll log | Esc/Alt+C - Cancel | Enter/Alt+N - Next"
            }
            ConnectDialogStep::EndpointSelection => {
                "↑↓ - Select endpoint | PageUp/PageDown - scroll log | Esc/Alt+B - Back | Enter/Alt+N - Next | Alt+C - Cancel"
            }
            ConnectDialogStep::Authentication => {
                if self.authentication_type == AuthenticationType::UserPassword {
                    "↑↓ - Change auth type | Tab - Switch fields | Alt+O - Connect | Alt+B - Back | Alt+C - Cancel"
                } else {
                    "↑↓ - Change auth type | Alt+O - Connect | Alt+B - Back | Alt+C - Cancel"
                }
            }
        };
        
        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(help_paragraph, area);
    }
    fn render_connecting_popup(&self, f: &mut Frame, area: Rect) {
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
        );        // Render the popup
        let popup = Paragraph::new("\nDiscovering Endpoints")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("Please Wait")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White).bg(Color::Blue))
            );
        
        f.render_widget(popup, popup_area);
    }
}
