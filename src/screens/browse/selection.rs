impl super::BrowseScreen {
    pub fn toggle_node_selection(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let node = &self.tree_nodes[index];
            let node_id = &node.node_id;

            if self.selected_items.contains(node_id) {
                // Unselect this node only
                self.selected_items.remove(node_id);
            } else {
                // Select this node only
                self.selected_items.insert(node_id.clone());
            }
        }
    }
}
