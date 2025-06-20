use anyhow::Result;
use crossterm::event::KeyCode;
use crate::client::ConnectionStatus;

impl super::BrowseScreen {
    pub async fn handle_input(
        &mut self,
        key: KeyCode,
        _modifiers: crossterm::event::KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Disconnect and return to connect screen
                Ok(Some(ConnectionStatus::Disconnected))
            }
            KeyCode::Up => {
                if self.selected_node_index > 0 {
                    self.selected_node_index -= 1;
                    self.update_scroll();
                    self.update_selected_attributes();
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.selected_node_index < self.tree_nodes.len().saturating_sub(1) {
                    self.selected_node_index += 1;
                    self.update_scroll();
                    self.update_selected_attributes();
                }
                Ok(None)
            }
            KeyCode::Right | KeyCode::Enter => {
                // Expand node if it has children
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.has_children && !node.is_expanded {
                        self.expand_node(self.selected_node_index);
                        self.update_selected_attributes();
                    }
                }
                Ok(None)
            }
            KeyCode::Char(' ') => {
                // Toggle selection of current node
                if self.selected_node_index < self.tree_nodes.len() {
                    self.toggle_node_selection(self.selected_node_index);
                }
                Ok(None)
            }
            KeyCode::Char('c') => {
                // Clear all selections
                self.clear_selections();
                Ok(None)
            }
            KeyCode::Left => {
                // Left key behavior:
                // 1. If current node is expanded, collapse it
                // 2. If current node is not expanded, move to parent
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.is_expanded {
                        // Collapse the current node
                        self.collapse_node(self.selected_node_index);
                        self.update_selected_attributes();
                    } else if node.level > 0 {
                        // Move to immediate parent node
                        self.move_to_parent();
                        self.update_selected_attributes();
                    }
                }
                Ok(None)
            }
            KeyCode::PageUp => {
                let page_size = 10;
                self.selected_node_index = self.selected_node_index.saturating_sub(page_size);
                self.update_scroll();
                self.update_selected_attributes();
                Ok(None)
            }
            KeyCode::PageDown => {
                let page_size = 10;
                self.selected_node_index = (self.selected_node_index + page_size)
                    .min(self.tree_nodes.len().saturating_sub(1));
                self.update_scroll();
                self.update_selected_attributes();
                Ok(None)
            }
            KeyCode::Home => {
                self.selected_node_index = 0;
                self.scroll_offset = 0;
                self.update_selected_attributes();
                Ok(None)
            }
            KeyCode::End => {
                self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
                self.update_scroll();
                self.update_selected_attributes();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}
