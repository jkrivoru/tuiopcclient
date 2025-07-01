use super::types::TreeNode;
use anyhow::Result;

impl super::BrowseScreen {
    // Centralized path generation for consistency
    pub fn get_node_path(&self, node: &TreeNode) -> String {
        crate::node_utils::NodeUtils::generate_node_path(node)
    }

    // Check if a node can be expanded
    pub fn can_expand(&self, index: usize) -> bool {
        index < self.tree_nodes.len()
            && self.tree_nodes[index].has_children
            && self.tree_nodes[index].should_show_expand_indicator()
            && !self.tree_nodes[index].is_expanded
    }

    // Check if a node can be collapsed
    pub fn can_collapse(&self, index: usize) -> bool {
        index < self.tree_nodes.len() && self.tree_nodes[index].is_expanded
    }

    // New method to track expansion state
    pub fn update_expansion_state(&mut self, index: usize, expanded: bool) {
        if index < self.tree_nodes.len() {
            let node_path = self.get_node_path(&self.tree_nodes[index]);
            self.tree_nodes[index].is_expanded = expanded;

            if expanded {
                self.expanded_nodes.insert(node_path);
            } else {
                self.expanded_nodes.remove(&node_path);
            }
        }
    }

    // Simplified method to restore expansion states
    pub fn restore_child_expansion_states(&self, child_nodes: &mut Vec<TreeNode>) {
        for child in child_nodes.iter_mut() {
            let child_path = self.get_node_path(child);

            // Check if this child was previously expanded
            if self.expanded_nodes.contains(&child_path) {
                child.is_expanded = true;
            }
        }
    }

    // Improved collapse method
    pub fn collapse_node(&mut self, index: usize) {
        if !self.can_collapse(index) {
            return;
        }

        let node_level = self.tree_nodes[index].level;

        // Update expansion state
        self.update_expansion_state(index, false);

        // Find range of children to remove
        let mut end_index = index + 1;
        while end_index < self.tree_nodes.len() && self.tree_nodes[end_index].level > node_level {
            end_index += 1;
        }

        // Adjust selected index if needed
        if self.selected_node_index > index && self.selected_node_index < end_index {
            // Selected node is a child that will be removed
            self.selected_node_index = index;
        } else if self.selected_node_index >= end_index {
            // Selected node is after the removed children
            self.selected_node_index -= end_index - index - 1;
        }

        // Remove children from visual tree (but keep their expansion state for restoration)
        self.tree_nodes.drain(index + 1..end_index);

        // Ensure selected index is valid
        if self.selected_node_index >= self.tree_nodes.len() {
            self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
        }
    } // Toggle expansion state
    #[allow(dead_code)]
    pub async fn toggle_node_async(&mut self, index: usize) -> Result<()> {
        if index >= self.tree_nodes.len() {
            log::warn!("browse: cannot toggle node, index {} out of bounds", index);
            return Ok(());
        }

        if self.tree_nodes[index].is_expanded {
            log::debug!("browse: collapsing node at index {}", index);
            self.collapse_node(index);
        } else {
            log::debug!("browse: expanding node at index {}", index);
            self.expand_node_async(index).await?;
        }

        Ok(())
    }

    // Move to parent node
    pub fn move_to_parent(&mut self) {
        if self.selected_node_index >= self.tree_nodes.len() {
            log::warn!("browse: cannot move to parent, selected index out of bounds");
            return;
        }

        let current_level = self.tree_nodes[self.selected_node_index].level;
        if current_level == 0 {
            log::debug!("browse: already at root level, cannot move to parent");
            return; // Already at root level
        }

        // Find the immediate parent node
        for i in (0..self.selected_node_index).rev() {
            if self.tree_nodes[i].level == current_level - 1 {
                log::debug!("browse: moved to parent node at index {}", i);
                self.selected_node_index = i;
                self.update_scroll();
                break;
            }
        }
    }

    // Update scroll position
    pub fn update_scroll(&mut self) {
        // Use the stored current visible height
        self.update_scroll_with_height(self.current_visible_height);
    }

    pub fn update_scroll_with_height(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // Calculate 25% zones
        let top_25_percent = visible_height / 4;
        let bottom_25_percent = visible_height.saturating_sub(visible_height / 4);

        // Get the current position of selected item relative to visible area
        let current_visible_position = if self.selected_node_index >= self.scroll_offset {
            self.selected_node_index - self.scroll_offset
        } else {
            0
        };

        // Check if item is outside visible area or in the 25% zones
        if self.selected_node_index < self.scroll_offset {
            // Item is above visible area - scroll to position it at 25% from top
            self.scroll_offset = self.selected_node_index.saturating_sub(top_25_percent);
        } else if self.selected_node_index >= self.scroll_offset + visible_height {
            // Item is below visible area - scroll to position it at 75% from top (25% from bottom)
            let target_position = (visible_height * 3) / 4; // 75% from top
            self.scroll_offset = self.selected_node_index.saturating_sub(target_position);
        } else if current_visible_position < top_25_percent {
            // Item is in top 25% - scroll to position it at 25% from top
            self.scroll_offset = self.selected_node_index.saturating_sub(top_25_percent);
        } else if current_visible_position >= bottom_25_percent {
            // Item is in bottom 25% - scroll to position it at 75% from top (25% from bottom)
            let target_position = (visible_height * 3) / 4; // 75% from top
            self.scroll_offset = self.selected_node_index.saturating_sub(target_position);
        }
        // If item is in the middle 50%, no scrolling needed
    }

    // Find next sibling at the same level
    #[allow(dead_code)]
    pub fn move_to_next_sibling(&mut self) {
        if self.selected_node_index >= self.tree_nodes.len() {
            log::warn!("browse: cannot move to next sibling, selected index out of bounds");
            return;
        }

        let current_level = self.tree_nodes[self.selected_node_index].level;

        for i in self.selected_node_index + 1..self.tree_nodes.len() {
            let node_level = self.tree_nodes[i].level;

            if node_level < current_level {
                // We've gone up a level, no more siblings
                break;
            } else if node_level == current_level {
                // Found next sibling
                log::debug!("browse: moved to next sibling at index {}", i);
                self.selected_node_index = i;
                self.update_scroll();
                break;
            }
        }
    }

    // Find previous sibling at the same level
    #[allow(dead_code)]
    pub fn move_to_previous_sibling(&mut self) {
        if self.selected_node_index == 0 {
            log::debug!("browse: already at first node, cannot move to previous sibling");
            return;
        }

        let current_level = self.tree_nodes[self.selected_node_index].level;

        for i in (0..self.selected_node_index).rev() {
            let node_level = self.tree_nodes[i].level;

            if node_level < current_level {
                // We've gone up a level, no more siblings
                break;
            } else if node_level == current_level {
                // Found previous sibling
                log::debug!("browse: moved to previous sibling at index {}", i);
                self.selected_node_index = i;
                self.update_scroll();
                break;
            }
        }
    }

    // Async method that handles real OPC UA data
    pub async fn expand_node_async(&mut self, index: usize) -> Result<()> {
        if !self.can_expand(index) {
            return Ok(());
        }

        // Check if this has real OPC UA data
        let has_real_node_id = self.tree_nodes[index].opcua_node_id.is_some();

        if has_real_node_id {
            // Use real OPC UA data
            self.expand_real_node(index).await?;
        } else {
            // No real data available
            log::warn!("browse: cannot expand node without real OPC UA NodeId");
        }

        Ok(())
    }
}
