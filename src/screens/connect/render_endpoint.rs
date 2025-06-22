use super::types::*;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

impl ConnectScreen {
    pub fn render_endpoint_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = self.create_step_layout(area); // Title
        let title_text = format!(
            "Connect to OPC UA Server - Step {}/{}: Select Endpoint",
            self.get_current_step_number(),
            self.get_total_steps()
        );
        let title = crate::ui_utils::LayoutUtils::create_title_paragraph(&title_text);
        f.render_widget(title, chunks[0]);

        // Calculate actual visible items based on UI height
        // Subtract 2 for borders, then limit to what can actually be displayed
        let list_height = chunks[1].height.saturating_sub(2) as usize;
        // Each endpoint takes 1 line, so visible items = available height
        let max_visible_items = list_height;
        // But we can't show more endpoints than we actually have
        let actual_visible_items = max_visible_items.min(self.discovered_endpoints.len());

        // Update scroll position to keep selected item visible using actual visible items
        self.update_endpoint_scroll(actual_visible_items);

        // Get the visible slice of endpoints after updating scroll
        let start_idx = self.endpoint_scroll_offset;
        let end_idx = (start_idx + actual_visible_items).min(self.discovered_endpoints.len());
        let visible_endpoints = &self.discovered_endpoints[start_idx..end_idx];
        // Store the actual visible count for use by mouse handler
        self.current_visible_endpoints_count = visible_endpoints.len();

        // Endpoint list with improved formatting and scrolling
        let items: Vec<ListItem> = visible_endpoints
            .iter()
            .enumerate()
            .map(|(visible_idx, endpoint)| {
                let actual_idx = start_idx + visible_idx;
                let prefix = if actual_idx == self.selected_endpoint_index {
                    "▶ "
                } else {
                    "  "
                };

                // Add security level indicator
                let security_indicator = match (&endpoint.security_policy, &endpoint.security_mode)
                {
                    (SecurityPolicy::None, SecurityMode::None) => "🔴", // Red circle for no security
                    (_, SecurityMode::Sign) => "🟡", // Yellow circle for sign only
                    (_, SecurityMode::SignAndEncrypt) => "🟢", // Green circle for sign & encrypt
                    _ => "⚪",                       // White circle for unknown
                };

                // Format the display text more cleanly
                let display_text =
                    format!("{}{} {}", prefix, security_indicator, endpoint.display_name);

                // Use default styling for all items - only the security circle provides color
                ListItem::new(display_text)
            })
            .collect(); // Create title with scroll indicators
        let has_above = self.has_endpoints_above();
        let has_below = self.has_endpoints_below(actual_visible_items);
        let scroll_indicators = match (has_above, has_below) {
            (true, true) => " ↑↓",
            (true, false) => " ↑",
            (false, true) => " ↓",
            (false, false) => "",
        };
        let title_text = format!(
            "Available Endpoints ({}/{} shown){}",
            actual_visible_items,
            self.discovered_endpoints.len(),
            scroll_indicators
        );
        let endpoint_list = List::new(items)
            .block(
                Block::default()
                    .title(title_text)
                    .borders(Borders::ALL)
                    .title_style(Style::default().fg(Color::White)),
            )
            .highlight_style(Style::default().bg(Color::Blue));
        f.render_widget(endpoint_list, chunks[1]);

        // Buttons (3 buttons for step 2) - left, center, right positioning with margins, 50% wider
        let button_chunks = self.create_button_layout(chunks[2]); // Render buttons using button manager (use chunks 1, 3, 5 for left/center/right positioning with margins)
        let button_rects = self.get_button_rects(&button_chunks);
        self.button_manager.render_buttons(f, &button_rects);
    }
}
