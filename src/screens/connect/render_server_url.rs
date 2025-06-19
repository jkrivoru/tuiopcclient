use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use super::types::*;

impl ConnectScreen {    pub(super) fn render_server_url_step(&mut self, f: &mut Frame, area: Rect) {
        // Always use the same layout to prevent button jumping
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // URL input
                Constraint::Length(2),  // Error message space (always reserved)
                Constraint::Min(0),     // Space
                Constraint::Length(3),  // Buttons
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 1/3: Server URL")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);        // URL input with placeholder and validation styling
        let (input_text, input_style) = if self.server_url_input.value().is_empty() && self.input_mode == InputMode::Editing {
            // Show placeholder
            ("opc.tcp://localhost:4840".to_string(), Style::default().fg(Color::DarkGray))
        } else {
            // Show actual input
            (self.server_url_input.value().to_string(), Style::default().fg(Color::White))
        };
        
        // Set border color based on validation
        let border_color = if self.server_url_validation_error.is_some() {
            Color::Red
        } else {
            Color::Yellow
        };
        
        // Use tui-input's built-in scrolling and rendering
        let width = chunks[1].width.max(3) - 3; // Account for borders
        let scroll = self.server_url_input.visual_scroll(width as usize);
        
        let input_paragraph = Paragraph::new(input_text)
            .style(input_style)
            .scroll((0, scroll as u16))
            .block(Block::default()
                .title("Server URL")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title_style(Style::default().fg(Color::Yellow)));
        
        f.render_widget(input_paragraph, chunks[1]);
        
        // Position cursor if editing and not showing placeholder
        if self.input_mode == InputMode::Editing && !self.server_url_input.value().is_empty() {
            let cursor_x = self.server_url_input.visual_cursor().max(scroll) - scroll + 1;
            f.set_cursor(chunks[1].x + cursor_x as u16, chunks[1].y + 1);
        }
        
        // Show validation error if present (always use chunk[2])
        if let Some(ref error) = self.server_url_validation_error {
            let error_text = Paragraph::new(format!("⚠ {}", error))
                .style(Style::default().fg(Color::Red));
            f.render_widget(error_text, chunks[2]);
        }

        // Buttons (2 buttons for step 1) - always use chunk[4] to prevent jumping
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space between
                Constraint::Length(18), // Next button (12 * 1.5 = 18)
                Constraint::Length(2),  // Right margin
            ])
            .split(chunks[4]);        // Update button states based on current progress and validation
        if self.connect_in_progress || self.server_url_validation_error.is_some() {
            self.button_manager.set_button_enabled("next", false);
        } else {
            self.button_manager.set_button_enabled("next", true);
        }

        // Render buttons using button manager (use chunks 1 and 3 for left/right positioning with margins)
        let button_rects = &[button_chunks[1], button_chunks[3]];
        self.button_manager.render_buttons(f, button_rects);
    }
}
