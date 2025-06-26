use super::types::NodeType;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use tui_logger::{TuiLoggerWidget, TuiLoggerLevelOutput};

impl super::BrowseScreen {
    pub fn render(&mut self, f: &mut Frame, area: Rect) -> (Option<Rect>, Option<Rect>, Option<Rect>) {
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
                Constraint::Percentage(50), // Tree view
                Constraint::Percentage(50), // Attributes panel
            ])
            .split(main_chunks[0]);

        // Tree view
        self.render_tree_view(f, content_chunks[0]);

        // Attributes panel
        self.render_attributes_panel(f, content_chunks[1]);

        // Status bar
        self.render_status_bar(f, main_chunks[1]);

        // Render dialogs and return their areas
        let search_dialog_area = if self.search_dialog_open {
            Some(self.render_search_dialog(f, area))
        } else {
            None
        };
        
        let progress_dialog_area = if self.search_progress_open {
            Some(self.render_progress_dialog(f, area))
        } else {
            None
        };
        
        let log_viewer_area = if self.log_viewer_open {
            Some(self.render_log_viewer(f, area))
        } else {
            None
        };
        
        (search_dialog_area, progress_dialog_area, log_viewer_area)
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
            Span::styled(
                "OPC UA Server: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.server_url, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled("Connected", Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(&selected_node_info, Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(&selection_count, Style::default().fg(Color::Magenta)),            Span::raw(
                " | Use ←/→ expand/collapse, ↑/↓ navigate, SPACE select, c clear, F3/Ctrl+F search, F12 logs, q/Esc exit",
            ),
        ];

        let status = Paragraph::new(Line::from(status_text))
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));

        f.render_widget(status, area);
    }

    fn render_tree_view(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        self.current_visible_height = visible_height; // Store current visible height
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
                };                // Create indentation based on level
                let indent = "  ".repeat(node.level);

                // Use consistent width for expand icons
                // Only show expand icons for node types that can actually have children
                let expand_icon = if node.should_show_expand_indicator() {
                    if node.has_children && node.is_expanded {
                        "▼"
                    } else if node.has_children {
                        "▶"
                    } else {
                        " " // Node type can have children but this instance doesn't
                    }
                } else {
                    " " // Node type never has children (e.g., Variables, Methods)
                };

                // Format: [indent][expand_icon] [type_icon] [name]
                let name = format!("{}{} {} {}", indent, expand_icon, icon, node.name);

                let style = if is_selected {
                    if is_selected_for_subscription {
                        Style::default()
                            .bg(Color::Blue)
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .bg(Color::Blue)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    }
                } else if is_selected_for_subscription {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(name)).style(style)
            })
            .collect();

        // Add scroll indicator
        let title = if self.tree_nodes.len() > visible_height {
            format!(
                "OPC UA Node Tree ({}/{} shown)",
                visible_nodes.len(),
                self.tree_nodes.len()
            )
        } else {
            "OPC UA Node Tree".to_string()
        };

        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray)),
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
                        },
                    );
                }
            }
        }
    }    fn render_attributes_panel(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(4) as usize; // Subtract borders and header

        let start_idx = self.attribute_scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.selected_attributes.len());
        let visible_attributes = if !self.selected_attributes.is_empty() {
            &self.selected_attributes[start_idx..end_idx]
        } else {
            &[]
        };

        // Calculate optimal attribute name column width
        let max_attr_name_length = if !self.selected_attributes.is_empty() {
            self.selected_attributes
                .iter()
                .map(|attr| attr.name.len())
                .max()
                .unwrap_or(0)
        } else {
            10 // Default minimum width
        };

        // Calculate percentage based on area width, but cap at 40%
        let available_width = area.width.saturating_sub(3) as usize; // Subtract borders and spacing
        let attr_name_percentage = if available_width > 0 {
            let calculated_percentage = (max_attr_name_length * 100) / available_width;
            calculated_percentage.min(40) // Cap at 40%
        } else {
            40
        };

        let value_percentage = 100 - attr_name_percentage;        let rows: Vec<Row> = visible_attributes
            .iter()
            .map(|attr| {                let value_cell = if attr.name == "Value" {
                    // Color code the Value attribute based on is_value_good
                    if attr.is_value_good {
                        Cell::from(attr.value.as_str()).style(Style::default().fg(Color::Green))
                    } else {
                        Cell::from(attr.value.as_str()).style(Style::default().fg(Color::Red))
                    }                } else {
                    // Dynamic search highlighting - check if search text exists in current attribute
                    if !self.search_input.value().trim().is_empty() {
                        let search_text = self.search_input.value().trim().to_lowercase();
                        
                        // Check if this attribute should be searched based on name and checkbox state
                        let should_search = match attr.name.as_str() {
                            "NodeId" | "BrowseName" | "DisplayName" => true,
                            _ => self.search_include_values, // Only search other attributes if checkbox is checked
                        };
                        
                        if should_search {
                            let value_str = &attr.value;
                            let value_lower = value_str.to_lowercase();
                            
                            if let Some(start_pos) = value_lower.find(&search_text) {
                                let length = search_text.len();
                                
                                // Create highlighted text using Spans for partial highlighting
                                if start_pos < value_str.len() && start_pos + length <= value_str.len() {
                                    let before = &value_str[..start_pos];
                                    let highlighted = &value_str[start_pos..start_pos + length];
                                    let after = &value_str[start_pos + length..];
                                    
                                    // Create spans with different styling - only highlight the matched part
                                    let mut spans = Vec::new();
                                    if !before.is_empty() {
                                        spans.push(Span::styled(before, Style::default().fg(Color::White)));
                                    }
                                    spans.push(Span::styled(highlighted, Style::default().bg(Color::Yellow).fg(Color::Black)));
                                    if !after.is_empty() {
                                        spans.push(Span::styled(after, Style::default().fg(Color::White)));
                                    }
                                    
                                    Cell::from(Line::from(spans))
                                } else {
                                    Cell::from(attr.value.as_str())
                                }
                            } else {
                                Cell::from(attr.value.as_str())
                            }
                        } else {
                            Cell::from(attr.value.as_str())
                        }
                    } else {
                        // Regular styling for other attributes when no search is active
                        Cell::from(attr.value.as_str())
                    }
                };

                Row::new(vec![
                    Cell::from(attr.name.as_str()),
                    value_cell,
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(attr_name_percentage as u16), 
                Constraint::Percentage(value_percentage as u16)
            ],
        )
        .header(
            Row::new(vec!["Attribute", "Value"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .title("Node Attributes")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray)),
        )
        .column_spacing(1);

        f.render_widget(table, area);
    }    fn render_search_dialog(&self, f: &mut Frame, area: Rect) -> Rect {        // Calculate dialog position (centered)
        let dialog_width = 50;
        let dialog_height = 6; // Reduced from 8 to 6 (2 lines smaller)
        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;
        
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);        // Create a semi-transparent overlay only around the borders of the dialog
        let overlay_padding = 1;
        let overlay_area = Rect::new(
            dialog_area.x.saturating_sub(overlay_padding),
            dialog_area.y.saturating_sub(overlay_padding),
            dialog_area.width + (overlay_padding * 2),
            dialog_area.height + (overlay_padding * 2),
        );
        
        let overlay = Block::default()
            .style(Style::default().bg(Color::Black));
        f.render_widget(overlay, overlay_area);
        
        // Clear the dialog area to ensure clean rendering
        let overlay_content = ratatui::widgets::Clear;
        f.render_widget(overlay_content, dialog_area);        // Main dialog box with blue background and white border
        let dialog_block = Block::default()
            .title("Find Node")
            .title_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default().bg(Color::Blue));
        f.render_widget(dialog_block, dialog_area);

        // Inner content area
        let inner_area = Rect::new(
            dialog_area.x + 1,
            dialog_area.y + 1,
            dialog_area.width - 2,
            dialog_area.height - 2,
        );        // Create layout for dialog content
        let dialog_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input field + button row (with borders)
                Constraint::Length(1), // Checkbox (removed spacing above and below)
            ])
            .split(inner_area);

        // Create horizontal layout for input field and button
        let input_button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Input field (70% of width)
                Constraint::Length(1),      // Small spacing
                Constraint::Percentage(29), // Button (29% of width)
            ])
            .split(dialog_chunks[0]);        // Input field styled like connect screen
        let (input_text, input_style) = if self.search_input.value().is_empty() {
            // Show placeholder
            ("Enter NodeId, BrowseName, or DisplayName...".to_string(), Style::default().fg(Color::DarkGray))
        } else {
            // Show actual input
            (self.search_input.value().to_string(), Style::default().fg(Color::White))
        };        // Set border color based on focus
        let input_border_color = if matches!(self.search_dialog_focus, super::types::SearchDialogFocus::Input) {
            Color::Yellow
        } else {
            Color::White
        };

        // Use tui-input's built-in scrolling and rendering
        let width = input_button_chunks[0].width.max(3) - 3; // Account for borders
        let scroll = self.search_input.visual_scroll(width as usize);

        let input_paragraph = Paragraph::new(input_text)
            .style(input_style)
            .scroll((0, scroll as u16))
            .block(
                Block::default()
                    .title("Search text")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(input_border_color))
                    .title_style(Style::default().fg(Color::Yellow)),
            );
        f.render_widget(input_paragraph, input_button_chunks[0]);

        // Position cursor if input is focused and not showing placeholder
        if matches!(self.search_dialog_focus, super::types::SearchDialogFocus::Input) && !self.search_input.value().is_empty() {
            let cursor_x = self.search_input.visual_cursor().max(scroll) - scroll + 1;
            f.set_cursor_position((input_button_chunks[0].x + cursor_x as u16, input_button_chunks[0].y + 1));
        }        // Text-only button: [ Find Next ]
        let button_enabled = !self.search_input.value().trim().is_empty();
        
        // Text-only button area - positioned in the middle of the button area
        let button_text_area = Rect {
            x: input_button_chunks[2].x,
            y: input_button_chunks[2].y + 1, // Center vertically
            width: input_button_chunks[2].width,
            height: 1,
        };          // Button text color based on state (no focus highlighting since not in Tab navigation)
        let button_text_color = if !button_enabled {
            Color::DarkGray
        } else {
            Color::LightGreen // Always bright green when enabled
        };// Text-only button with brackets
        let button_text = "[ Find Next ]";
        
        let button_paragraph = Paragraph::new(button_text)
            .style(Style::default()
                .fg(button_text_color)
                .bg(Color::Blue) // Keep dialog background
                .add_modifier(Modifier::BOLD)) // Bold and underlined for emphasis
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(button_paragraph, button_text_area);// Render "Also look at values" checkbox
        let checkbox_symbol = if self.search_include_values { "☑" } else { "☐" };
        let checkbox_focused = matches!(self.search_dialog_focus, super::types::SearchDialogFocus::Checkbox);
        let checkbox_text = if checkbox_focused {
            format!("> {} Also look at values <", checkbox_symbol)
        } else {
            format!("  {} Also look at values", checkbox_symbol)
        };

        let checkbox_style = if checkbox_focused {
            Style::default().fg(Color::Yellow).bg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::Blue)        };        let checkbox_paragraph = Paragraph::new(checkbox_text).style(checkbox_style);
        f.render_widget(checkbox_paragraph, dialog_chunks[1]);
          // Return the dialog area for mouse handling
        dialog_area
    }
      fn render_progress_dialog(&self, f: &mut Frame, area: Rect) -> Rect {
        // Calculate dialog position (centered, wider than before)
        let dialog_width = 60.min(area.width.saturating_sub(4));
        let dialog_height = 5.min(area.height.saturating_sub(4));
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
        
        // Create a semi-transparent overlay around the dialog
        let overlay_padding = 1;
        let overlay_area = Rect::new(
            dialog_area.x.saturating_sub(overlay_padding),
            dialog_area.y.saturating_sub(overlay_padding),
            dialog_area.width + (overlay_padding * 2),
            dialog_area.height + (overlay_padding * 2),
        );
        
        let overlay = Block::default()
            .style(Style::default().bg(Color::Black));
        f.render_widget(overlay, overlay_area);
        
        // Clear the dialog area to ensure clean rendering
        let clear_widget = ratatui::widgets::Clear;
        f.render_widget(clear_widget, dialog_area);
        
        // Create the dialog block with full blue background
        let dialog_block = Block::default()
            .title(" Search Progress ")
            .title_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default().bg(Color::Blue));
        
        f.render_widget(dialog_block, dialog_area);
        
        // Inner area for content
        let inner_area = Rect::new(
            dialog_area.x + 1,
            dialog_area.y + 1,
            dialog_area.width - 2,
            dialog_area.height - 2,
        );
        
        // Split inner area vertically
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Message
                Constraint::Length(1), // Separator line
                Constraint::Length(1), // Cancel instruction
            ])
            .split(inner_area);
        
        // Render progress message
        let message_paragraph = Paragraph::new(self.search_progress_message.clone())
            .style(Style::default().fg(Color::White).bg(Color::Blue));
        f.render_widget(message_paragraph, chunks[0]);
        
        // Render empty separator line
        let separator_paragraph = Paragraph::new("")
            .style(Style::default().bg(Color::Blue));
        f.render_widget(separator_paragraph, chunks[1]);
        
        // Render cancel instruction
        let cancel_text = "Press ESC to cancel";
        let cancel_paragraph = Paragraph::new(cancel_text)
            .style(Style::default().fg(Color::Yellow).bg(Color::Blue));
        f.render_widget(cancel_paragraph, chunks[2]);
        
        // Return the dialog area for mouse handling
        dialog_area
    }    fn render_log_viewer(&self, f: &mut Frame, area: Rect) -> Rect {
        // Full-screen log viewer overlay
        let log_area = area; // Use the entire screen area

        // Clear the area to ensure clean rendering
        let clear_widget = Clear;
        f.render_widget(clear_widget, log_area);

        // Create the log viewer block with title and borders
        let log_block = Block::default()
            .title(" Log Viewer (F12/ESC to close) ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .style(Style::default().bg(Color::Black).fg(Color::White));

        f.render_widget(log_block, log_area);

        // Inner area for the actual log content
        let inner_area = Rect::new(
            log_area.x + 1,
            log_area.y + 1,
            log_area.width - 2,
            log_area.height.saturating_sub(3), // Leave space for instructions
        );        // Create the TuiLoggerWidget with proper state management
        let tui_logger = TuiLoggerWidget::default()
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
            .state(&self.logger_widget_state) // Use proper state management
            .block(
                Block::default()
                    .borders(Borders::NONE) // No borders since we already have the outer block
                    .style(Style::default().bg(Color::Black).fg(Color::White))
            );

        // Render the logger widget
        f.render_widget(tui_logger, inner_area);

        // Add navigation instructions at the bottom
        let instruction_area = Rect::new(
            log_area.x + 1,
            log_area.y + log_area.height.saturating_sub(2),
            log_area.width.saturating_sub(2),
            1,
        );        let instructions = Paragraph::new("Use ↑/↓, PgUp/PgDown, Home/End to scroll | F12/ESC to close")
            .style(Style::default().fg(Color::Yellow).bg(Color::Black))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(instructions, instruction_area);

        // Return the full area for mouse handling
        log_area
    }
}
