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
            .state(&self.logger_widget_state);
        f.render_widget(logger_widget, chunks[1]);
    }

    pub fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help_text = match self.step {
            ConnectDialogStep::ServerUrl => {
                "PageUp/PageDown - scroll log | Esc - Cancel | Enter/Alt+N - Next | Alt+C - Cancel"
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
}
