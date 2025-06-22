impl super::BrowseScreen {
    pub fn toggle_node_selection(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let node = &self.tree_nodes[index];
            let node_id = &node.node_id;

            if self.selected_items.contains(node_id) {
                // Unselect this node and all its children
                self.unselect_node_and_children(index);
            } else {
                // Select this node and all its children
                self.select_node_and_children(index);
            }

            // Update parent selection state
            self.update_parent_selection_state(index);
        }
    }
    fn select_node_and_children(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let (node_id, node_name, parent_path, has_children) =
                crate::node_utils::NodeUtils::extract_node_info(&self.tree_nodes[index]);

            // Select the current node
            self.selected_items.insert(node_id.clone());

            // If this node has children, select all children (recursively)
            if has_children {
                self.select_all_children_recursive(&node_id, &node_name, &parent_path);
            }
        }
    }
    fn unselect_node_and_children(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let (node_id, node_name, parent_path, has_children) =
                crate::node_utils::NodeUtils::extract_node_info(&self.tree_nodes[index]);

            // Unselect the current node
            self.selected_items.remove(&node_id);

            // If this node has children, unselect all children (recursively)
            if has_children {
                self.unselect_all_children_recursive(&node_id, &node_name, &parent_path);
            }
        }
    }
    fn select_all_children_recursive(
        &mut self,
        parent_node_id: &str,
        parent_name: &str,
        parent_path: &str,
    ) {
        // Get demo children for this parent
        let current_path = crate::node_utils::NodeUtils::generate_path(parent_path, parent_name);

        let children = self.get_demo_children(parent_node_id, 0, &current_path); // level doesn't matter for selection

        for child in children {
            // Select this child
            self.selected_items.insert(child.node_id.clone());

            // If this child has children, recursively select them too
            if child.has_children {
                self.select_all_children_recursive(&child.node_id, &child.name, &child.parent_path);
            }
        }
    }
    fn unselect_all_children_recursive(
        &mut self,
        parent_node_id: &str,
        parent_name: &str,
        parent_path: &str,
    ) {
        // Get demo children for this parent
        let current_path = crate::node_utils::NodeUtils::generate_path(parent_path, parent_name);

        let children = self.get_demo_children(parent_node_id, 0, &current_path); // level doesn't matter for selection

        for child in children {
            // Unselect this child
            self.selected_items.remove(&child.node_id);

            // If this child has children, recursively unselect them too
            if child.has_children {
                self.unselect_all_children_recursive(
                    &child.node_id,
                    &child.name,
                    &child.parent_path,
                );
            }
        }
    }

    fn update_parent_selection_state(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let node = &self.tree_nodes[index];

            // Find the parent node if it exists
            if node.level > 0 {
                if let Some(parent_index) = self.find_parent_node(index) {
                    let parent_node = &self.tree_nodes[parent_index];
                    let parent_node_id = parent_node.node_id.clone();

                    // Check if all children of the parent are selected
                    let all_children_selected = self.are_all_children_selected(
                        &parent_node.node_id,
                        &parent_node.name,
                        &parent_node.parent_path,
                    );

                    if all_children_selected {
                        // All children are selected, so select the parent
                        self.selected_items.insert(parent_node_id);
                    } else {
                        // Not all children are selected, so unselect the parent
                        self.selected_items.remove(&parent_node_id);
                    }

                    // Recursively update parent's parent
                    self.update_parent_selection_state(parent_index);
                }
            }
        }
    }

    fn find_parent_node(&self, index: usize) -> Option<usize> {
        if index < self.tree_nodes.len() {
            let current_level = self.tree_nodes[index].level;

            // Find parent node (move backwards to find a node with level - 1)
            for i in (0..index).rev() {
                if self.tree_nodes[i].level == current_level - 1 {
                    return Some(i);
                }
            }
        }
        None
    }
    fn are_all_children_selected(
        &self,
        parent_node_id: &str,
        parent_name: &str,
        parent_path: &str,
    ) -> bool {
        let current_path = crate::node_utils::NodeUtils::generate_path(parent_path, parent_name);

        let children = self.get_demo_children(parent_node_id, 0, &current_path);

        for child in children {
            if !self.selected_items.contains(&child.node_id) {
                return false; // Found an unselected child
            }

            // If this child has children, check them recursively
            if child.has_children {
                if !self.are_all_children_selected(&child.node_id, &child.name, &child.parent_path)
                {
                    return false;
                }
            }
        }

        true // All children are selected
    }
}
