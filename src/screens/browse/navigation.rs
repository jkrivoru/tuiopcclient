use anyhow::Result;

impl super::BrowseScreen {
    // Renamed existing method for demo data
    pub fn expand_node_demo(&mut self, index: usize) {
        if index < self.tree_nodes.len() && self.tree_nodes[index].has_children {
            let node_path =
                crate::node_utils::NodeUtils::generate_node_path(&self.tree_nodes[index]);

            // Get node info before modifying the vector
            let (node_id, level, parent_path) = {
                let node = &self.tree_nodes[index];
                (node.node_id.clone(), node.level, node_path.clone())
            };

            self.tree_nodes[index].is_expanded = true;

            // Add child nodes (demo data)
            let mut child_nodes = self.get_demo_children(&node_id, level + 1, &parent_path); // Restore expanded state for child nodes that were previously expanded
            for child in &mut child_nodes {
                let child_path =
                    crate::node_utils::NodeUtils::generate_path(&child.parent_path, &child.name);

                if self.expanded_nodes.contains(&child_path) {
                    child.is_expanded = true;
                }
            }

            // Insert children after the current node
            for (i, child) in child_nodes.into_iter().enumerate() {
                self.tree_nodes.insert(index + 1 + i, child);
            } // Recursively expand any child nodes that should be expanded
            self.restore_child_expansions(index + 1, level + 1);
        }
    }

    pub fn collapse_node(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let (node_path, node_level) = {
                let node = &self.tree_nodes[index];
                let path = crate::node_utils::NodeUtils::generate_node_path(node);
                (path, node.level)
            };

            if self.expanded_nodes.contains(&node_path) {
                // Remove the current node from expanded set
                self.expanded_nodes.remove(&node_path);
                self.tree_nodes[index].is_expanded = false;

                // Store the current selected node info before removing children
                let was_selected_child_removed = self.selected_node_index > index;

                // NOTE: We intentionally DO NOT remove child paths from expanded_nodes
                // This preserves the expanded state of child nodes for when the parent is re-expanded

                // Count and remove all child nodes from the visual tree
                let mut removed_count = 0;
                let i = index + 1;
                while i < self.tree_nodes.len() && self.tree_nodes[i].level > node_level {
                    self.tree_nodes.remove(i);
                    removed_count += 1;
                    // Don't increment i since we removed an element
                }

                // Adjust selected index more carefully
                if was_selected_child_removed && self.selected_node_index > index {
                    if self.selected_node_index <= index + removed_count {
                        // Selected node was removed, stay on the collapsed parent
                        self.selected_node_index = index;
                    } else {
                        // Selected node was after the removed children, adjust index
                        self.selected_node_index -= removed_count;
                    }
                }

                // Ensure selected index is still valid
                if self.selected_node_index >= self.tree_nodes.len() {
                    self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
                }
            }
        }
    }

    fn restore_child_expansions(&mut self, start_index: usize, current_level: usize) {
        let mut i = start_index;
        while i < self.tree_nodes.len() && self.tree_nodes[i].level >= current_level {
            if self.tree_nodes[i].level == current_level {
                let node = &self.tree_nodes[i];
                if node.has_children && node.is_expanded {
                    // This child was previously expanded, so expand it again
                    let (node_id, level, parent_path) = {
                        let node = &self.tree_nodes[i];
                        let path = if node.parent_path.is_empty() {
                            node.name.clone()
                        } else {
                            format!("{}/{}", node.parent_path, node.name)
                        };
                        (node.node_id.clone(), node.level, path)
                    };

                    // Add child nodes for this expanded node
                    let mut child_nodes = self.get_demo_children(&node_id, level + 1, &parent_path); // Restore expanded state for grandchildren
                    for child in &mut child_nodes {
                        let child_path = crate::node_utils::NodeUtils::generate_path(
                            &child.parent_path,
                            &child.name,
                        );

                        if self.expanded_nodes.contains(&child_path) {
                            child.is_expanded = true;
                        }
                    }

                    // Insert children after the current node
                    for (j, child) in child_nodes.into_iter().enumerate() {
                        self.tree_nodes.insert(i + 1 + j, child);
                    }

                    // Skip over the newly inserted children and continue
                    let children_count = self
                        .get_demo_children(&node_id, level + 1, &parent_path)
                        .len();
                    i += children_count + 1;

                    // Recursively restore expansions for the newly added children
                    if children_count > 0 {
                        self.restore_child_expansions(i - children_count, level + 1);
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    pub fn move_to_parent(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let current_level = self.tree_nodes[self.selected_node_index].level;

            if current_level > 0 {
                // Find the immediate parent node (level = current_level - 1)
                for i in (0..self.selected_node_index).rev() {
                    if self.tree_nodes[i].level == current_level - 1 {
                        self.selected_node_index = i;
                        self.update_scroll();
                        break;
                    }
                }
            }
        }
    }

    pub fn update_scroll(&mut self) {
        // This will be updated with actual visible height in render
        let visible_height = 20;
        self.update_scroll_with_height(visible_height);
    }

    pub fn update_scroll_with_height(&mut self, visible_height: usize) {
        if self.selected_node_index < self.scroll_offset {
            self.scroll_offset = self.selected_node_index;
        } else if self.selected_node_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_node_index.saturating_sub(visible_height - 1);
        }
    }

    // New async method that handles both demo and real data
    pub async fn expand_node_async(&mut self, index: usize) -> Result<()> {
        if index >= self.tree_nodes.len() || !self.tree_nodes[index].has_children {
            return Ok(());
        }
        let node_path = crate::node_utils::NodeUtils::generate_node_path(&self.tree_nodes[index]);

        if self.expanded_nodes.contains(&node_path) {
            return Ok(()); // Already expanded
        }

        self.expanded_nodes.insert(node_path.clone());

        // Check if this is real OPC UA data or demo data
        let has_real_node_id = self.tree_nodes[index].opcua_node_id.is_some();

        if has_real_node_id {
            // Use real OPC UA data
            self.expand_real_node(index).await?;
        } else {
            // Use demo data (fallback to existing sync method)
            self.expand_node_demo(index);
        }

        Ok(())
    }
}
