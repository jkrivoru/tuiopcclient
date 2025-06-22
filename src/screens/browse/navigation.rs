use anyhow::Result;
use super::types::TreeNode;

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
        index < self.tree_nodes.len() 
            && self.tree_nodes[index].is_expanded
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

    // Improved expand method for demo data
    pub fn expand_node_demo(&mut self, index: usize) {
        if !self.can_expand(index) {
            return;
        }

        // Update expansion state
        self.update_expansion_state(index, true);

        // Get node info before modifying the vector
        let (node_id, level, parent_path) = {
            let node = &self.tree_nodes[index];
            (node.node_id.clone(), node.level, self.get_node_path(node))
        };

        // Get child nodes
        let mut child_nodes = self.get_demo_children(&node_id, level + 1, &parent_path);
        
        // Restore expansion state for children
        self.restore_child_expansion_states(&mut child_nodes);

        // Insert children after the current node
        self.tree_nodes.splice(index + 1..index + 1, child_nodes);

        // Recursively expand previously expanded children
        let mut i = index + 1;
        let parent_level = level;
        while i < self.tree_nodes.len() && self.tree_nodes[i].level > parent_level {
            if self.tree_nodes[i].is_expanded && self.tree_nodes[i].level == parent_level + 1 {
                // Temporarily set to false to allow expansion
                self.tree_nodes[i].is_expanded = false;
                self.expand_node_demo(i);
            }
            i += 1;
        }
    }    // Improved collapse method
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
            self.selected_node_index = index;        } else if self.selected_node_index >= end_index {
            // Selected node is after the removed children
            self.selected_node_index -= end_index - index - 1;
        }

        // Remove children from visual tree (but keep their expansion state)
        self.tree_nodes.drain(index + 1..end_index);

        // Ensure selected index is valid
        if self.selected_node_index >= self.tree_nodes.len() {
            self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
        }
    }    // Toggle expansion state
    pub async fn toggle_node_async(&mut self, index: usize) -> Result<()> {
        if index >= self.tree_nodes.len() {
            return Ok(());
        }

        if self.tree_nodes[index].is_expanded {
            self.collapse_node(index);
        } else {
            self.expand_node_async(index).await?;
        }

        Ok(())
    }

    // Move to parent node
    pub fn move_to_parent(&mut self) {
        if self.selected_node_index >= self.tree_nodes.len() {
            return;
        }

        let current_level = self.tree_nodes[self.selected_node_index].level;
        if current_level == 0 {
            return; // Already at root level
        }

        // Find the immediate parent node
        for i in (0..self.selected_node_index).rev() {
            if self.tree_nodes[i].level == current_level - 1 {
                self.selected_node_index = i;
                self.update_scroll();
                break;
            }
        }
    }

    // Update scroll position
    pub fn update_scroll(&mut self) {
        // This will be updated with actual visible height in render
        let visible_height = 20;
        self.update_scroll_with_height(visible_height);
    }

    pub fn update_scroll_with_height(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // Keep selected item in view
        if self.selected_node_index < self.scroll_offset {
            self.scroll_offset = self.selected_node_index;
        } else if self.selected_node_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_node_index.saturating_sub(visible_height - 1);
        }
    }

    // Find next sibling at the same level
    pub fn move_to_next_sibling(&mut self) {
        if self.selected_node_index >= self.tree_nodes.len() {
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
                self.selected_node_index = i;
                self.update_scroll();
                break;
            }
        }
    }

    // Find previous sibling at the same level
    pub fn move_to_previous_sibling(&mut self) {
        if self.selected_node_index == 0 {
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
                self.selected_node_index = i;
                self.update_scroll();
                break;
            }
        }
    }    // New async method that handles both demo and real data
    pub async fn expand_node_async(&mut self, index: usize) -> Result<()> {
        if !self.can_expand(index) {
            return Ok(());
        }

        // Check if this is real OPC UA data or demo data
        let has_real_node_id = self.tree_nodes[index].opcua_node_id.is_some();

        if has_real_node_id {
            // Use real OPC UA data
            self.expand_real_node(index).await?;
        } else {
            // Use demo data
            self.expand_node_demo(index);
        }        Ok(())
    }
}
