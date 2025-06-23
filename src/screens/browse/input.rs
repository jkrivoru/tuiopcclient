use crate::client::ConnectionStatus;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::time::{Duration, Instant};

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
                    if let Err(e) = self.update_selected_attributes_async().await {
                        log::error!("Failed to update attributes: {}", e);
                    }
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.selected_node_index < self.tree_nodes.len().saturating_sub(1) {
                    self.selected_node_index += 1;
                    self.update_scroll();
                    if let Err(e) = self.update_selected_attributes_async().await {
                        log::error!("Failed to update attributes: {}", e);
                    }
                }
                Ok(None)
            }            KeyCode::Right | KeyCode::Enter => {
                // Expand node if it supports expansion (based on node type) and has children
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.should_show_expand_indicator() && node.has_children && !node.is_expanded {
                        if let Err(e) = self.expand_node_async(self.selected_node_index).await {
                            log::error!("Failed to expand node: {}", e);
                        }
                        if let Err(e) = self.update_selected_attributes_async().await {
                            log::error!("Failed to update attributes: {}", e);
                        }
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
                        if let Err(e) = self.update_selected_attributes_async().await {
                            log::error!("Failed to update attributes: {}", e);
                        }
                    } else if node.level > 0 {
                        // Move to immediate parent node
                        self.move_to_parent();
                        if let Err(e) = self.update_selected_attributes_async().await {
                            log::error!("Failed to update attributes: {}", e);
                        }
                    }
                }
                Ok(None)
            }
            KeyCode::PageUp => {
                let page_size = 10;
                self.selected_node_index = self.selected_node_index.saturating_sub(page_size);
                self.update_scroll();
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("Failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::PageDown => {
                let page_size = 10;
                self.selected_node_index = (self.selected_node_index + page_size)
                    .min(self.tree_nodes.len().saturating_sub(1));
                self.update_scroll();
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("Failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::Home => {
                self.selected_node_index = 0;
                self.scroll_offset = 0;
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("Failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::End => {
                self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
                self.update_scroll();
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("Failed to update attributes: {}", e);
                }
                Ok(None)
            }            KeyCode::Char('r') => {
                // Refresh/reload real OPC UA data
                if let Err(e) = self.load_real_tree().await {
                    log::error!("Failed to load real OPC UA data: {}", e);
                }
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("Failed to update attributes: {}", e);
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    pub async fn handle_mouse_input(
        &mut self,
        mouse: MouseEvent,
        tree_area: Rect,
    ) -> Result<Option<ConnectionStatus>> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_left_click(mouse.column, mouse.row, tree_area)
                    .await
            }
            MouseEventKind::Down(MouseButton::Right) => {
                self.handle_right_click(mouse.column, mouse.row, tree_area)
                    .await
            }
            _ => Ok(None),
        }
    }

    async fn handle_left_click(
        &mut self,
        x: u16,
        y: u16,
        tree_area: Rect,
    ) -> Result<Option<ConnectionStatus>> {
        // Check if click is within the tree area
        if x >= tree_area.x
            && x < tree_area.x + tree_area.width
            && y >= tree_area.y
            && y < tree_area.y + tree_area.height
        {
            let relative_y = y.saturating_sub(tree_area.y);
            let clicked_index = (relative_y as usize).saturating_add(self.scroll_offset);

            if clicked_index < self.tree_nodes.len() {
                let now = Instant::now();
                let is_double_click = self.is_double_click(x, y, now);

                if is_double_click {
                    // Double-click: expand/collapse node
                    self.handle_double_click(clicked_index).await
                } else {
                    // Single click: navigate to node
                    self.handle_single_click(clicked_index).await
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn handle_right_click(
        &mut self,
        x: u16,
        y: u16,
        tree_area: Rect,
    ) -> Result<Option<ConnectionStatus>> {
        // Check if right-click is within the tree area
        if x >= tree_area.x
            && x < tree_area.x + tree_area.width
            && y >= tree_area.y
            && y < tree_area.y + tree_area.height
        {
            let relative_y = y.saturating_sub(tree_area.y);
            let clicked_index = (relative_y as usize).saturating_add(self.scroll_offset);

            if clicked_index < self.tree_nodes.len() {
                // Right-click: toggle selection
                self.toggle_node_selection(clicked_index);
            }
        }
        Ok(None)
    }
    async fn handle_single_click(&mut self, index: usize) -> Result<Option<ConnectionStatus>> {
        // Navigate to the clicked node
        self.selected_node_index = index;
        self.update_scroll();
        if let Err(e) = self.update_selected_attributes_async().await {
            log::error!("Failed to update attributes: {}", e);
        }
        Ok(None)
    }    async fn handle_double_click(&mut self, index: usize) -> Result<Option<ConnectionStatus>> {
        // First, navigate to the node
        self.selected_node_index = index;
        self.update_scroll();

        // Then expand/collapse if it supports expansion and has children
        if index < self.tree_nodes.len() {
            let node = &self.tree_nodes[index];
            if node.should_show_expand_indicator() && node.has_children {
                if node.is_expanded {
                    self.collapse_node(index);
                } else {
                    if let Err(e) = self.expand_node_async(index).await {
                        log::error!("Failed to expand node: {}", e);
                    }
                }
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("Failed to update attributes: {}", e);
                }
            }
        }
        Ok(None)
    }

    fn is_double_click(&mut self, x: u16, y: u16, now: Instant) -> bool {
        const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);
        const DOUBLE_CLICK_DISTANCE: u16 = 2; // pixels

        if let (Some(last_time), Some(last_pos)) = (self.last_click_time, self.last_click_position)
        {
            let time_diff = now.duration_since(last_time);
            let distance = ((x as i32 - last_pos.0 as i32).abs()
                + (y as i32 - last_pos.1 as i32).abs()) as u16;

            let is_double =
                time_diff <= DOUBLE_CLICK_THRESHOLD && distance <= DOUBLE_CLICK_DISTANCE;

            if is_double {
                // Reset click tracking after detecting double-click
                self.last_click_time = None;
                self.last_click_position = None;
                return true;
            }
        }

        // Update last click info for next time
        self.last_click_time = Some(now);
        self.last_click_position = Some((x, y));
        false
    }
}
