use crate::client::ConnectionStatus;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::time::{Duration, Instant};
use tui_input::backend::crossterm::EventHandler;

impl super::BrowseScreen {    pub async fn handle_input(
        &mut self,
        key: KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        // Handle search dialog input first
        if self.search_dialog_open {
            return self.handle_search_input(key, modifiers).await;
        }

        match key {
            KeyCode::F(3) => {
                // F3: Open search dialog or go to next result
                if !self.last_search_query.is_empty() && !self.search_results.is_empty() {
                    self.next_search_result().await?;
                } else {
                    self.open_search_dialog();
                }
                Ok(None)
            }
            KeyCode::Char('f') if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                // Ctrl+F: Open search dialog
                self.open_search_dialog();
                Ok(None)
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Disconnect and return to connect screen (or close search dialog)
                if self.search_dialog_open {
                    // Close search dialog first
                    self.close_search_dialog();
                    Ok(None)
                } else {
                    // Disconnect and return to connect screen
                    Ok(Some(ConnectionStatus::Disconnected))
                }
            }
            // Disable navigation keys when search dialog is open (except F3, Ctrl+F, Esc, q)
            _ if self.search_dialog_open => {
                Ok(None)
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
    }    pub async fn handle_mouse_input(
        &mut self,
        mouse: MouseEvent,
        tree_area: Rect,
        dialog_area: Option<Rect>,
    ) -> Result<Option<ConnectionStatus>> {
        // Handle search dialog mouse input first
        if self.search_dialog_open {
            return self.handle_search_mouse_input(mouse, dialog_area).await;
        }

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
        }        // Update last click info for next time
        self.last_click_time = Some(now);
        self.last_click_position = Some((x, y));
        false
    }

    // Search functionality methods
    fn open_search_dialog(&mut self) {
        self.search_dialog_open = true;
        self.search_input = tui_input::Input::default();
        self.search_dialog_focus = super::types::SearchDialogFocus::Input;
    }    fn close_search_dialog(&mut self) {
        self.search_dialog_open = false;
        // Keep search_input intact so highlighting persists
        self.search_dialog_focus = super::types::SearchDialogFocus::Input;
    }async fn handle_search_input(
        &mut self,
        key: KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {            KeyCode::Esc => {
                self.close_search_dialog();
                Ok(None)
            }
            KeyCode::Tab => {
                // Cycle through Input -> Checkbox -> Input (button removed from navigation)
                use super::types::SearchDialogFocus;
                self.search_dialog_focus = match self.search_dialog_focus {
                    SearchDialogFocus::Input => SearchDialogFocus::Checkbox,
                    SearchDialogFocus::Checkbox => SearchDialogFocus::Input,
                    SearchDialogFocus::Button => SearchDialogFocus::Input, // If somehow on button, go to input
                };
                Ok(None)
            }
            KeyCode::Enter => {
                use super::types::SearchDialogFocus;
                match self.search_dialog_focus {
                    SearchDialogFocus::Button => {
                        // Find Next button selected
                        if !self.search_input.value().trim().is_empty() {
                            self.perform_search().await?;
                        }
                        self.close_search_dialog();
                    }
                    SearchDialogFocus::Input => {
                        // Enter pressed in input field - perform search if not empty
                        if !self.search_input.value().trim().is_empty() {
                            self.perform_search().await?;
                            self.close_search_dialog();
                        }
                    }
                    SearchDialogFocus::Checkbox => {
                        // Enter on checkbox toggles it
                        self.search_include_values = !self.search_include_values;
                    }
                }
                Ok(None)
            }
            KeyCode::Char(' ') => {
                use super::types::SearchDialogFocus;
                match self.search_dialog_focus {
                    SearchDialogFocus::Checkbox => {
                        // Space on checkbox toggles it
                        self.search_include_values = !self.search_include_values;
                    }
                    SearchDialogFocus::Input => {
                        // Space in input field - let tui-input handle it
                        self.search_input
                            .handle_event(&crossterm::event::Event::Key(
                                crossterm::event::KeyEvent::new(key, modifiers),
                            ));
                    }
                    _ => {
                        // Space on buttons - ignore
                    }
                }
                Ok(None)
            }
            // Let tui-input handle all other keys when input is focused
            _ => {
                use super::types::SearchDialogFocus;
                if self.search_dialog_focus == SearchDialogFocus::Input {
                    self.search_input
                        .handle_event(&crossterm::event::Event::Key(
                            crossterm::event::KeyEvent::new(key, modifiers),
                        ));
                }
                Ok(None)
            }
        }
    }

    async fn handle_search_mouse_input(
        &mut self,
        mouse: MouseEvent,
        dialog_area: Option<Rect>,
    ) -> Result<Option<ConnectionStatus>> {
        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
            if let Some(dialog_area) = dialog_area {
                let inner_area = Rect::new(
                    dialog_area.x + 1,
                    dialog_area.y + 1,
                    dialog_area.width - 2,
                    dialog_area.height - 2,
                );

                // Check if click is within dialog
                if mouse.column >= dialog_area.x && mouse.column < dialog_area.x + dialog_area.width &&
                   mouse.row >= dialog_area.y && mouse.row < dialog_area.y + dialog_area.height {
                    
                    // Calculate relative position within inner area
                    let relative_y = mouse.row.saturating_sub(inner_area.y);
                    let relative_x = mouse.column.saturating_sub(inner_area.x);

                    use super::types::SearchDialogFocus;
                    match relative_y {
                        0..=2 => {
                            // Input field area and button area (same row)
                            let input_width = (inner_area.width * 70) / 100; // 70% for input

                            if relative_x < input_width {
                                // Clicked in input area - focus the input if not already focused
                                if self.search_dialog_focus != SearchDialogFocus::Input {
                                    self.search_dialog_focus = SearchDialogFocus::Input;
                                }
                            } else if relative_x >= input_width + 1 && relative_y == 1 {
                                // Clicked in button area (after spacing) and on the middle line
                                // Always perform search if input is not empty, regardless of current focus
                                if !self.search_input.value().trim().is_empty() {
                                    // Perform search on button click
                                    self.perform_search().await?;
                                    self.close_search_dialog();
                                }
                            }
                        }
                        3 => {
                            // Checkbox area - toggle checkbox on click
                            self.search_include_values = !self.search_include_values;
                            // Also set focus to checkbox
                            self.search_dialog_focus = SearchDialogFocus::Checkbox;
                        }
                        _ => {
                            // Clicked elsewhere in dialog - keep current focus
                        }
                    }
                } else {
                    // Clicked outside dialog - close it
                    self.close_search_dialog();
                }
            }
        }
        Ok(None)
    }    async fn perform_search(&mut self) -> Result<()> {
        let query = self.search_input.value().trim().to_lowercase();        self.last_search_query = query.clone();
        self.search_results.clear();
        self.current_search_index = 0;

        // First, search through currently visible nodes
        for (_index, node) in self.tree_nodes.iter().enumerate() {
            if self.node_matches_query(node, &query).await? {
                self.search_results.push(node.node_id.clone());
            }
        }

        // Then, search through unexpanded areas by temporarily expanding them
        // We'll collect all possible search matches by doing a deep search
        let additional_matches = self.deep_search_unexpanded_nodes(&query).await?;
        
        // Add any new matches to search results
        for node_id in additional_matches {
            if !self.search_results.contains(&node_id) {
                self.search_results.push(node_id);
            }
        }

        // No need to sort since we're using node IDs now

        if !self.search_results.is_empty() {
            // Navigate to first result
            self.navigate_to_search_result(0).await?;
        }

        Ok(())
    }

    async fn node_matches_query(&self, node: &super::types::TreeNode, query: &str) -> Result<bool> {
        let node_id_lower = node.node_id.to_lowercase();
        let name_lower = node.name.to_lowercase();
        
        // Check if query matches NodeId, BrowseName (name), or DisplayName
        if node_id_lower.contains(query) || name_lower.contains(query) {
            return Ok(true);
        }
        
        // If "Also look at values" is checked, search in node attributes
        if self.search_include_values {
            if let Some(opcua_node_id) = &node.opcua_node_id {
                // Try to read attributes for this node and search in values
                let client_guard = self.client.read().await;
                if let Ok(attributes) = client_guard.read_node_attributes(opcua_node_id).await {
                    for attr in &attributes {
                        if attr.value.to_lowercase().contains(query) {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }    async fn deep_search_unexpanded_nodes(&mut self, query: &str) -> Result<Vec<String>> {
        let mut additional_matches = Vec::new();
        let mut processed_nodes = std::collections::HashSet::new();
        
        // Use iterative approach instead of recursion to avoid boxing issues
        loop {
            let mut nodes_to_expand = Vec::new();
            
            // Find nodes that have children but aren't expanded and haven't been processed
            for (index, node) in self.tree_nodes.iter().enumerate() {
                if node.has_children && !node.is_expanded && node.should_show_expand_indicator()
                    && !processed_nodes.contains(&node.node_id) {
                    nodes_to_expand.push((index, node.node_id.clone()));
                }
            }
            
            if nodes_to_expand.is_empty() {
                break; // No more nodes to expand
            }
            
            // Process each unexpanded node
            for (node_index, node_id) in nodes_to_expand {
                if processed_nodes.contains(&node_id) {
                    continue; // Skip if already processed
                }
                
                let original_tree_len = self.tree_nodes.len();
                
                // Temporarily expand the node
                if let Err(e) = self.expand_node_async(node_index).await {
                    log::warn!("Failed to expand node during search: {}", e);
                    processed_nodes.insert(node_id);
                    continue;
                }
                
                // Search through the newly added nodes
                for new_index in original_tree_len..self.tree_nodes.len() {
                    if let Some(node) = self.tree_nodes.get(new_index) {
                        if self.node_matches_query(node, query).await? {
                            additional_matches.push(node.node_id.clone());
                        }
                    }
                }
                
                // Mark as processed and collapse back to restore original state
                processed_nodes.insert(node_id);
                self.collapse_node(node_index);
            }
        }

        Ok(additional_matches)
    }

    async fn next_search_result(&mut self) -> Result<()> {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
            self.navigate_to_search_result(self.current_search_index).await?;
        }
        Ok(())
    }    async fn navigate_to_search_result(&mut self, result_index: usize) -> Result<()> {
        if result_index < self.search_results.len() {
            let target_node_id = self.search_results[result_index].clone(); // Clone to avoid borrowing issues
            
            // Find the current index of the node with this ID
            let node_index = self.find_node_index_by_id(&target_node_id);
            
            if let Some(node_index) = node_index {
                // Expand parent nodes if necessary
                self.ensure_node_visible(node_index).await?;
                
                // Re-find the index after potential tree changes
                if let Some(updated_node_index) = self.find_node_index_by_id(&target_node_id) {
                    // Select the node
                    self.selected_node_index = updated_node_index;
                    self.update_scroll();
                    
                    // Update attributes and set up highlighting
                    if let Err(e) = self.update_selected_attributes_async().await {
                        log::error!("Failed to update attributes: {}", e);                    }
                }
            } else {
                // Node not currently visible, need to search and expand to find it
                self.expand_to_find_node(&target_node_id).await?;
            }
        }
        Ok(())
    }

    fn find_node_index_by_id(&self, target_node_id: &str) -> Option<usize> {
        self.tree_nodes.iter().position(|node| node.node_id == target_node_id)
    }

    async fn expand_to_find_node(&mut self, target_node_id: &str) -> Result<()> {
        // This is a simplified version - in a real implementation, you might need
        // to traverse the OPC UA server structure to find the node's path
        log::warn!("Node with ID {} not found in current tree - may need deeper search", target_node_id);
        Ok(())
    }

    async fn ensure_node_visible(&mut self, target_index: usize) -> Result<()> {
        if target_index >= self.tree_nodes.len() {
            return Ok(());
        }

        let target_node = &self.tree_nodes[target_index];
        let target_level = target_node.level;

        // Find all parent nodes that need to be expanded
        let mut parents_to_expand = Vec::new();
        
        for (index, node) in self.tree_nodes.iter().enumerate() {
            if index < target_index && node.level < target_level {
                // Check if this node is a parent of our target
                let target_path_prefix = &target_node.parent_path;
                if target_path_prefix.starts_with(&node.parent_path) && 
                   target_path_prefix.len() > node.parent_path.len() {
                    if !node.is_expanded && node.has_children {
                        parents_to_expand.push(index);
                    }
                }
            }
        }

        // Expand parent nodes
        for parent_index in parents_to_expand {
            if let Err(e) = self.expand_node_async(parent_index).await {
                log::error!("Failed to expand parent node during search navigation: {}", e);
            }
        }        Ok(())
    }
}
