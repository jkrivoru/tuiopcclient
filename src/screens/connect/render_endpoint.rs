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

        // Calculate visible items (subtract 2 for borders)
        let list_height = chunks[1].height.saturating_sub(2) as usize;
        let visible_items = list_height;
        
        // Update scroll position to keep selected item visible
        self.update_endpoint_scroll(visible_items);
        
        // Get the visible slice of endpoints
        let start_idx = self.endpoint_scroll_offset;
        let end_idx = (start_idx + visible_items).min(self.discovered_endpoints.len());
        let visible_endpoints = &self.discovered_endpoints[start_idx..end_idx];        // Endpoint list with improved formatting and scrolling
        let items: Vec<ListItem> = visible_endpoints
            .iter()
            .enumerate()
            .map(|(visible_idx, endpoint)| {
                let actual_idx = start_idx + visible_idx;
                let prefix = if actual_idx == self.selected_endpoint_index { "▶ " } else { "  " };
                
                // Add security level indicator
                let security_indicator = match (&endpoint.security_policy, &endpoint.security_mode) {
                    (SecurityPolicy::None, SecurityMode::None) => "🔴", // Red circle for no security
                    (_, SecurityMode::Sign) => "🟡", // Yellow circle for sign only
                    (_, SecurityMode::SignAndEncrypt) => "🟢", // Green circle for sign & encrypt
                    _ => "⚪", // White circle for unknown
                };
                
                // Format the display text more cleanly
                let display_text = format!("{}{} {}", prefix, security_indicator, endpoint.display_name);
                
                // Use default styling for all items - only the security circle provides color
                ListItem::new(display_text)
            })
            .collect();

        // Create title with scroll indicators
        let has_above = self.has_endpoints_above();
        let has_below = self.has_endpoints_below(visible_items);
        let scroll_indicators = match (has_above, has_below) {
            (true, true) => " ↑↓",
            (true, false) => " ↑",
            (false, true) => " ↓",
            (false, false) => "",
        };
        
        let title_text = format!(
            "Available Endpoints ({}/{} shown){}", 
            visible_endpoints.len(),
            self.discovered_endpoints.len(),
            scroll_indicators
        );        let endpoint_list = List::new(items)
            .block(Block::default()
                .title(title_text)
                .borders(Borders::ALL)
                .title_style(Style::default().fg(Color::Cyan)))
            .highlight_style(Style::default().bg(Color::Blue));
        
        f.render_widget(endpoint_list, chunks[1]);

        // Buttons (3 buttons for step 2) - left, center, right positioning with margins, 50% wider
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
