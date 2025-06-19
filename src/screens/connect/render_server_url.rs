use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use super::types::*;

impl ConnectScreen {
    pub(super) fn render_server_url_step(&mut self, f: &mut Frame, area: Rect) {        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // URL input
                Constraint::Min(0),     // Space (like other screens)
                Constraint::Length(3),  // Buttons (aligned with other screens)
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 1/3: Server URL")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // URL input
        let input_style = Style::default().fg(Color::Yellow);
        
        // Use tui-input's built-in scrolling and rendering
        let width = chunks[1].width.max(3) - 3; // Account for borders
        let scroll = self.server_url_input.visual_scroll(width as usize);
        
        let input_text = Paragraph::new(self.server_url_input.value())
            .style(input_style)
            .scroll((0, scroll as u16))
            .block(Block::default()
                .title("Server URL")
                .borders(Borders::ALL)
                .title_style(Style::default().fg(Color::Yellow)));
        
        f.render_widget(input_text, chunks[1]);
        
        // Position cursor if editing
        if self.input_mode == InputMode::Editing {
            let cursor_x = self.server_url_input.visual_cursor().max(scroll) - scroll + 1;
            f.set_cursor(chunks[1].x + cursor_x as u16, chunks[1].y + 1);
        }        // Buttons (2 buttons for step 1) - left and right positioning with margin, 50% wider
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space between
                Constraint::Length(18), // Next button (12 * 1.5 = 18)
                Constraint::Length(2),  // Right margin
            ])
            .split(chunks[3]); // Changed from chunks[2] to chunks[3]

        // Update button states based on current progress
        if self.connect_in_progress {
            self.button_manager.set_button_enabled("next", false);
        } else {
            self.button_manager.set_button_enabled("next", true);
        }

        // Render buttons using button manager (use chunks 1 and 3 for left/right positioning with margins)
        let button_rects = &[button_chunks[1], button_chunks[3]];
        self.button_manager.render_buttons(f, button_rects);
    }
}
