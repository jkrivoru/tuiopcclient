use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use super::types::*;

impl ConnectScreen {
    pub fn render_endpoint_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Endpoint list
                Constraint::Length(3),  // Buttons
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Connect to OPC UA Server - Step 2/3: Select Endpoint")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Endpoint list
        let items: Vec<ListItem> = self.discovered_endpoints
            .iter()
            .enumerate()
            .map(|(i, endpoint)| {
                let prefix = if i == self.selected_endpoint_index { "▶ " } else { "  " };
                let security_desc = format!("{:?} - {:?}", endpoint.security_policy, endpoint.security_mode);
                ListItem::new(format!("{}{}  [{}]", prefix, endpoint.display_name, security_desc))
            })
            .collect();

        let endpoint_list = List::new(items)
            .block(Block::default()
                .title("Available Endpoints")
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
        
        f.render_widget(endpoint_list, chunks[1]);        // Buttons (3 buttons for step 2) - left, center, right positioning with margins, 50% wider
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Cancel button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Back button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space
                Constraint::Length(18), // Next button (12 * 1.5 = 18)
                Constraint::Length(2),  // Right margin
            ])
            .split(chunks[2]);

        // Render buttons using button manager (use chunks 1, 3, 5 for left/center/right positioning with margins)
        let button_rects = &[button_chunks[1], button_chunks[3], button_chunks[5]];
        self.button_manager.render_buttons(f, button_rects);
    }
}
