use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Table, Row, Cell},
    Frame,
};
use super::types::NodeType;

impl super::BrowseScreen {
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        // Main content area: Tree view on left, attributes on right
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Tree view
                Constraint::Percentage(30), // Attributes panel
            ])
            .split(main_chunks[0]);

        // Tree view
        self.render_tree_view(f, content_chunks[0]);

        // Attributes panel
        self.render_attributes_panel(f, content_chunks[1]);

        // Status bar
        self.render_status_bar(f, main_chunks[1]);
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let selected_node_info = if self.selected_node_index < self.tree_nodes.len() {
            let node = &self.tree_nodes[self.selected_node_index];
            format!("Selected: {} | NodeId: {}", node.name, node.node_id)
        } else {
            "No node selected".to_string()
        };

        let selection_count = format!("{} selected", self.selected_items.len());

        let status_text = vec![
            Span::styled("OPC UA Server: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(&self.server_url, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled("Connected", Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(&selected_node_info, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(&selection_count, Style::default().fg(Color::Magenta)),
            Span::raw(" | Use ←/→ expand/collapse, ↑/↓ navigate, SPACE select, c clear, q/Esc exit"),
        ];

        let status = Paragraph::new(Line::from(status_text))
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));

        f.render_widget(status, area);
    }

    fn render_tree_view(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        self.update_scroll_with_height(visible_height);
        
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.tree_nodes.len());
        let visible_nodes = if start_idx < self.tree_nodes.len() {
            &self.tree_nodes[start_idx..end_idx]
        } else {
            &[]
        };

        let items: Vec<ListItem> = visible_nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let actual_index = start_idx + i;
                let is_selected = actual_index == self.selected_node_index;
                let is_selected_for_subscription = self.selected_items.contains(&node.node_id);
                
                let icon = match node.node_type {
                    NodeType::Object => "📁",
                    NodeType::Variable => "📊", 
                    NodeType::Method => "⚙️",
                    NodeType::View => "👁️",
                    NodeType::ObjectType => "🏷️",
                    NodeType::VariableType => "🔧",
                    NodeType::DataType => "📝",
                    NodeType::ReferenceType => "🔗",
                };

                // Create indentation based on level
                let indent = "  ".repeat(node.level);
                
                // Use consistent width for expand icons
                let expand_icon = if node.has_children {
                    if node.is_expanded { "▼" } else { "▶" }
                } else {
                    " "
                };

                // Format: [indent][expand_icon] [type_icon] [name]
                let name = format!("{}{} {} {}", indent, expand_icon, icon, node.name);
                
                let style = if is_selected {
                    if is_selected_for_subscription {
                        Style::default().bg(Color::Blue).fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                    }
                } else if is_selected_for_subscription {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(name)).style(style)
            })
            .collect();

        // Add scroll indicator
        let title = if self.tree_nodes.len() > visible_height {
            format!("OPC UA Node Tree ({}/{} shown)", visible_nodes.len(), self.tree_nodes.len())
        } else {
            "OPC UA Node Tree".to_string()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            );

        f.render_widget(list, area);
        
        // Render scrollbar if needed
        if self.tree_nodes.len() > visible_height {
            self.render_tree_scrollbar(f, area, visible_height);
        }
    }

    fn render_tree_scrollbar(&self, f: &mut Frame, area: Rect, visible_height: usize) {
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };

        if scrollbar_area.height > 0 {
            let total_items = self.tree_nodes.len();
            let scrollbar_height = scrollbar_area.height as usize;
            
            // Calculate thumb position and size
            let thumb_size = ((visible_height * scrollbar_height) / total_items).max(1);
            let thumb_position = (self.scroll_offset * scrollbar_height) / total_items;
            
            // Render scrollbar track
            for y in 0..scrollbar_height {
                let is_thumb = y >= thumb_position && y < thumb_position + thumb_size;
                let symbol = if is_thumb { "█" } else { "│" };
                let style = if is_thumb {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                
                if scrollbar_area.y + (y as u16) < f.area().height {
                    f.render_widget(
                        Paragraph::new(symbol).style(style),
                        Rect {
                            x: scrollbar_area.x,
                            y: scrollbar_area.y + y as u16,
                            width: 1,
                            height: 1,
                        }
                    );
                }
            }
        }
    }

    fn render_attributes_panel(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(4) as usize; // Subtract borders and header
        
        let start_idx = self.attribute_scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.selected_attributes.len());
        let visible_attributes = if !self.selected_attributes.is_empty() {
            &self.selected_attributes[start_idx..end_idx]
        } else {
            &[]
        };

        let rows: Vec<Row> = visible_attributes
            .iter()
            .map(|attr| {
                Row::new(vec![
                    Cell::from(attr.name.as_str()),
                    Cell::from(attr.value.as_str()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ]
        )
            .header(
                Row::new(vec!["Attribute", "Value"])
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            )
            .block(
                Block::default()
                    .title("Node Attributes")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            )
            .column_spacing(1);

        f.render_widget(table, area);
    }
}
