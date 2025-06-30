use super::recursive_search::ChildNodeInfo;
use crate::client::ConnectionStatus;
use anyhow::Result;
use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::{HashMap, HashSet, VecDeque};
use std::str::FromStr;
use std::time::{Duration, Instant};
use tui_input::backend::crossterm::EventHandler;
use tui_logger::TuiWidgetEvent;

impl super::BrowseScreen {
    pub async fn handle_input(
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
                // F3: Continue search from current position or open search dialog
                if !self.last_search_query.is_empty() {
                    log::info!("search: continuing search for '{}'", self.last_search_query);
                    self.continue_search().await?;
                } else {
                    log::info!("search: opening dialog (no previous search)");
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
                // Handle different dialog states
                if self.log_viewer_open {
                    // Close log viewer
                    self.log_viewer_open = false;
                    Ok(None)
                } else if self.search_progress_open {
                    // Cancel search progress
                    self.cancel_search();
                    Ok(None)
                } else if self.search_dialog_open {
                    // Close search dialog first
                    self.close_search_dialog();
                    Ok(None)
                } else {
                    // Disconnect and return to connect screen
                    Ok(Some(ConnectionStatus::Disconnected))
                }
            }
            // Disable navigation keys when any dialog is open (except F3, Ctrl+F, Esc, q)
            _ if self.search_dialog_open || self.search_progress_open || self.log_viewer_open => {
                // Allow some keys in log viewer for navigation
                if self.log_viewer_open {
                    match key {
                        KeyCode::F(12) => {
                            // F12: Close log viewer
                            self.log_viewer_open = false;
                            Ok(None)
                        }
                        KeyCode::Up => {
                            self.logger_widget_state.transition(TuiWidgetEvent::UpKey);
                            Ok(None)
                        }
                        KeyCode::Down => {
                            self.logger_widget_state.transition(TuiWidgetEvent::DownKey);
                            Ok(None)
                        }
                        KeyCode::PageUp => {
                            self.logger_widget_state
                                .transition(TuiWidgetEvent::PrevPageKey);
                            Ok(None)
                        }
                        KeyCode::PageDown => {
                            self.logger_widget_state
                                .transition(TuiWidgetEvent::NextPageKey);
                            Ok(None)
                        }
                        KeyCode::Home => {
                            // Go to the beginning - scroll up multiple pages
                            for _ in 0..10 {
                                self.logger_widget_state
                                    .transition(TuiWidgetEvent::PrevPageKey);
                            }
                            Ok(None)
                        }
                        KeyCode::End => {
                            // Go to the end (latest messages) - exit page mode
                            self.logger_widget_state
                                .transition(TuiWidgetEvent::EscapeKey);
                            Ok(None)
                        }
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            KeyCode::Up => {
                if self.selected_node_index > 0 {
                    self.selected_node_index -= 1;
                    self.update_scroll();
                    if let Err(e) = self.update_selected_attributes_async().await {
                        log::error!("browse: failed to update attributes: {}", e);
                    }
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.selected_node_index < self.tree_nodes.len().saturating_sub(1) {
                    self.selected_node_index += 1;
                    self.update_scroll();
                    if let Err(e) = self.update_selected_attributes_async().await {
                        log::error!("browse: failed to update attributes: {}", e);
                    }
                }
                Ok(None)
            }
            KeyCode::Right | KeyCode::Enter => {
                // Expand node if it supports expansion (based on node type) and has children
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.should_show_expand_indicator() && node.has_children && !node.is_expanded
                    {
                        if let Err(e) = self.expand_node_async(self.selected_node_index).await {
                            log::error!("browse: failed to expand node: {}", e);
                        }
                        if let Err(e) = self.update_selected_attributes_async().await {
                            log::error!("browse: failed to update attributes: {}", e);
                        }
                    }
                }
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
                            log::error!("browse: failed to update attributes: {}", e);
                        }
                    } else if node.level > 0 {
                        // Move to immediate parent node
                        self.move_to_parent();
                        if let Err(e) = self.update_selected_attributes_async().await {
                            log::error!("browse: failed to update attributes: {}", e);
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
                    log::error!("browse: failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::PageDown => {
                let page_size = 10;
                self.selected_node_index = (self.selected_node_index + page_size)
                    .min(self.tree_nodes.len().saturating_sub(1));
                self.update_scroll();
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("browse: failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::Home => {
                self.selected_node_index = 0;
                self.scroll_offset = 0;
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("browse: failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::End => {
                self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
                self.update_scroll();
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("browse: failed to update attributes: {}", e);
                }
                Ok(None)
            }
            KeyCode::F(12) => {
                // F12: Open log viewer (closing is handled in the log viewer navigation block)
                if !self.log_viewer_open {
                    self.log_viewer_open = true;
                    // Reset logger state when opening
                    self.logger_widget_state = tui_logger::TuiWidgetState::new();
                }
                Ok(None)
            }
            KeyCode::Char('r') => {
                // Refresh/reload real OPC UA data
                if let Err(e) = self.load_real_tree().await {
                    log::error!("browse: failed to load real OPC UA data: {}", e);
                }
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("browse: failed to update attributes: {}", e);
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
        dialog_area: Option<Rect>,
        progress_area: Option<Rect>,
    ) -> Result<Option<ConnectionStatus>> {
        // Disable mouse input when log viewer is open
        if self.log_viewer_open {
            return Ok(None);
        }

        // Handle search progress dialog mouse input first
        if self.search_progress_open {
            return self.handle_progress_mouse_input(mouse, progress_area).await;
        }

        // Handle search dialog mouse input
        if self.search_dialog_open {
            return self.handle_search_mouse_input(mouse, dialog_area).await;
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_left_click(mouse.column, mouse.row, tree_area)
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

    async fn handle_single_click(&mut self, index: usize) -> Result<Option<ConnectionStatus>> {
        // Navigate to the clicked node
        self.selected_node_index = index;
        self.update_scroll();
        if let Err(e) = self.update_selected_attributes_async().await {
            log::error!("browse: failed to update attributes: {}", e);
        }
        Ok(None)
    }
    async fn handle_double_click(&mut self, index: usize) -> Result<Option<ConnectionStatus>> {
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
                        log::error!("browse: failed to expand node: {}", e);
                    }
                }
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("browse: failed to update attributes: {}", e);
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

    fn cancel_search(&mut self) {
        log::info!("search: cancelling operation");
        self.search_cancelled = true;
        self.search_progress_open = false;

        // Send cancel command to background search task
        if let Some(tx) = &self.search_command_tx {
            if let Err(e) = tx.send(super::types::SearchCommand::Cancel) {
                log::warn!("search: failed to send cancel command to background search: {}", e);
            }
        }

        // Clear channels
        self.search_command_tx = None;
        self.search_message_rx = None;
    }

    // Search functionality methods
    fn open_search_dialog(&mut self) {
        self.search_dialog_open = true;
        self.search_input = tui_input::Input::default();
        self.search_dialog_focus = super::types::SearchDialogFocus::Input;
    }
    fn close_search_dialog(&mut self) {
        self.search_dialog_open = false;
        // Keep search_input intact so highlighting persists
        self.search_dialog_focus = super::types::SearchDialogFocus::Input;
    }
    async fn handle_search_input(
        &mut self,
        key: KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Esc => {
                self.close_search_dialog();
                Ok(None)
            }
            KeyCode::Tab => {
                // Cycle through Input -> Checkbox -> Input (button removed)
                use super::types::SearchDialogFocus;
                self.search_dialog_focus = match self.search_dialog_focus {
                    SearchDialogFocus::Input => SearchDialogFocus::Checkbox,
                    SearchDialogFocus::Checkbox => SearchDialogFocus::Input,
                };
                Ok(None)
            }
            KeyCode::Enter => {
                use super::types::SearchDialogFocus;
                match self.search_dialog_focus {
                    SearchDialogFocus::Input | SearchDialogFocus::Checkbox => {
                        // Enter pressed - perform search if not empty
                        if !self.search_input.value().trim().is_empty() {
                            self.perform_search().await?;
                            // Dialog is closed inside perform_search()
                        } else {
                            self.close_search_dialog();
                        }
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
                if mouse.column >= dialog_area.x
                    && mouse.column < dialog_area.x + dialog_area.width
                    && mouse.row >= dialog_area.y
                    && mouse.row < dialog_area.y + dialog_area.height
                {
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
                                    // Perform search on button click (dialog closed inside perform_search)
                                    self.perform_search().await?;
                                } else {
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
    }
    async fn perform_search(&mut self) -> Result<()> {
        let query = self.search_input.value().trim().to_lowercase();
        log::info!("search: initialized with query '{}' (include values: {})", query,
            self.search_include_values);

        if query.is_empty() {
            log::info!("search: empty query, nothing to search");
            return Ok(());
        }

        self.last_search_query = query.clone();
        self.search_results.clear();
        self.current_search_index = 0;

        // Close search dialog immediately so user can see progress
        self.search_dialog_open = false;

        // Check if we have a connection and tree data
        let has_connection = {
            let client_guard = self.client.read().await;
            client_guard.is_connected()
        };

        log::info!(
            "search: connection status {}, tree nodes count {}",
            has_connection,
            self.tree_nodes.len()
        );

        if has_connection && !self.tree_nodes.is_empty() {
            // Use background recursive search
            let start_node_id =
                if let Some(current_node) = self.tree_nodes.get(self.selected_node_index) {
                    // If the current node has an OPC UA node ID, use it
                    if let Some(ref opcua_node_id) = current_node.opcua_node_id {
                        log::info!(
                            "search: starting from selected node '{}' ({})",
                            current_node.name,
                            opcua_node_id
                        );
                        opcua_node_id.clone()
                    } else {
                        log::warn!("search: selected node has no OPC UA node ID, using ObjectsFolder");
                        opcua::types::ObjectId::ObjectsFolder.into()
                    }
                } else {
                    log::info!("search: no selected node, starting from ObjectsFolder");
                    opcua::types::ObjectId::ObjectsFolder.into()
                };

            // Start background search
            let options = super::recursive_search::RecursiveSearchOptions {
                query,
                include_values: self.search_include_values,
                start_node_id,
            };

            self.start_background_search(options)?;
        } else {
            // Fallback to local search if no connection
            self.search_progress_open = true;
            self.search_cancelled = false;
            self.search_progress_current = 0;
            self.search_progress_total = 1;
            self.search_progress_message = format!("Searching locally for '{}'...", query);

            self.perform_local_search().await?;
        }

        Ok(())
    }
    async fn perform_local_search(&mut self) -> Result<()> {
        let query = self.search_input.value().trim().to_lowercase();
        log::info!("search: performing local search for query '{}'", query);

        // Search through currently visible nodes for the first match
        for (_index, node) in self.tree_nodes.iter().enumerate() {
            if self.node_matches_query(node, &query).await? {
                log::debug!("search: local search found match {}", node.node_id);
                self.search_results.push(node.node_id.clone());

                // Navigate to first result immediately (like Windows search)
                self.navigate_to_search_result(0).await?;
                break; // Stop at first match
            }
        }

        if self.search_results.is_empty() {
            log::info!("search: local search found no matches");
        } else {
            log::info!("search: local search completed with 1 match");
        }

        // Hide progress popup
        self.search_progress_open = false;
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
    }

    /// Navigate to the search result (used when a match is found)
    async fn navigate_to_search_result(&mut self, result_index: usize) -> Result<()> {
        log::info!(
            "search: navigating to result index {} of {}",
            result_index,
            self.search_results.len()
        );

        if result_index < self.search_results.len() {
            let target_node_id = self.search_results[result_index].clone(); // Clone to avoid borrowing issues
            log::info!("search: looking for node with ID {}", target_node_id);

            // Find the current index of the node with this ID
            let node_index = self.find_node_index_by_id(&target_node_id);
            log::info!("search: found node at index {:?}", node_index);

            if let Some(node_index) = node_index {
                // Expand parent nodes if necessary
                self.ensure_node_visible(node_index).await?;

                // Re-find the index after potential tree changes
                if let Some(updated_node_index) = self.find_node_index_by_id(&target_node_id) {
                    log::info!("search: navigating to node at index {}", updated_node_index);

                    // Select the node
                    self.selected_node_index = updated_node_index;
                    self.update_scroll();

                    log::info!(
                        "search: navigation complete, selected index is now {}",
                        self.selected_node_index
                    );

                    // Update attributes and set up highlighting
                    if let Err(e) = self.update_selected_attributes_async().await {
                        log::error!("browse: failed to update attributes: {}", e);
                    }
                } else {
                    log::error!(
                        "search: node with ID {} disappeared after ensuring visibility",
                        target_node_id
                    );
                }
            } else {
                log::warn!("search: node with ID {} not found in current tree", target_node_id);
                // Node not currently visible, need to search and expand to find it
                self.expand_to_find_node(&target_node_id).await?;
            }
        } else {
            log::error!(
                "search: invalid result index {} (total results {})",
                result_index,
                self.search_results.len()
            );
        }
        Ok(())
    }

    fn find_node_index_by_id(&self, target_node_id: &str) -> Option<usize> {
        self.tree_nodes
            .iter()
            .position(|node| node.node_id == target_node_id)
    }
    pub async fn expand_to_find_node(&mut self, target_node_id: &str) -> Result<()> {
        log::info!("search: navigating to search result {}", target_node_id);

        // Parse the target node ID
        let target_opcua_node_id = match opcua::types::NodeId::from_str(target_node_id) {
            Ok(node_id) => node_id,
            Err(e) => {
                log::error!("search: failed to parse target node ID '{}': {}", target_node_id, e);
                return Err(anyhow::anyhow!(
                    "Invalid node ID format: {}",
                    target_node_id
                ));
            }
        };

        // Check if the target node is already visible in the tree
        if let Some(target_index) = self.find_node_index_by_id(target_node_id) {
            log::info!("search: target node already visible at index {}", target_index);
            self.selected_node_index = target_index;
            self.update_scroll();
            log::info!("search: navigation completed - node selected and scrolled into view");

            // Update attributes
            if let Err(e) = self.update_selected_attributes_async().await {
                log::error!("browse: failed to update attributes: {}", e);
            }
            return Ok(());
        }

        // Find the path from the root to the target node
        if let Some(path_to_target) = self.find_path_to_node(&target_opcua_node_id).await? {
            log::info!("search: found path to target node: {:?}", path_to_target);

            // Filter out the Objects node (i=85) since it's not displayed in the tree
            let objects_node_id = opcua::types::NodeId::new(0, 85u32);
            let filtered_path: Vec<_> = path_to_target
                .into_iter()
                .filter(|node_id| *node_id != objects_node_id)
                .collect();

            log::info!(
                "search: filtered path (excluding Objects node): {:?}",
                filtered_path
            );

            // Expand nodes along the filtered path to make the target visible
            for ancestor_node_id in filtered_path {
                if let Err(e) = self.expand_node_by_opcua_id(&ancestor_node_id).await {
                    log::error!("search: failed to expand ancestor node {}: {}", ancestor_node_id, e);
                    // Continue trying to expand other ancestors
                }
            }

            // Now try to find the target node in the expanded tree
            if let Some(target_index) = self.find_node_index_by_id(target_node_id) {
                log::info!("search: target node now visible at index {}", target_index);
                self.selected_node_index = target_index;
                self.update_scroll();

                // Update attributes
                if let Err(e) = self.update_selected_attributes_async().await {
                    log::error!("browse: failed to update attributes: {}", e);
                }
            } else {
                log::error!("search: target node still not visible after expanding path");
            }
        } else {
            log::error!("search: could not find path to target node {}", target_node_id);
        }

        Ok(())
    }

    /// Find the path from root to a target node by traversing the OPC UA server structure
    async fn find_path_to_node(
        &self,
        target_node_id: &opcua::types::NodeId,
    ) -> Result<Option<Vec<opcua::types::NodeId>>> {
        log::info!("search: finding path to node {}", target_node_id);

        let client_guard = self.client.read().await;
        if !client_guard.is_connected() {
            return Err(anyhow::anyhow!("OPC UA client is not connected"));
        }

        // Start from the Objects folder (standard starting point)
        let objects_node_id = opcua::types::NodeId::new(0, 85u32); // Objects folder

        // Use breadth-first search to find the path
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent_map: HashMap<opcua::types::NodeId, opcua::types::NodeId> = HashMap::new();

        queue.push_back(objects_node_id.clone());
        visited.insert(objects_node_id.clone());

        while let Some(current_node_id) = queue.pop_front() {
            // Check if we found the target
            if current_node_id == *target_node_id {
                log::info!("search: found target node, reconstructing path");

                // Reconstruct the path from root to target
                let mut path = Vec::new();
                let mut current = current_node_id;

                while let Some(parent) = parent_map.get(&current) {
                    path.push(parent.clone());
                    current = parent.clone();
                }

                path.reverse();
                return Ok(Some(path));
            }

            // Get children of current node
            if let Ok(browse_results) = client_guard.browse_node(&current_node_id).await {
                for result in browse_results {
                    let child_node_id = result.node_id;

                    if !visited.contains(&child_node_id) {
                        visited.insert(child_node_id.clone());
                        parent_map.insert(child_node_id.clone(), current_node_id.clone());
                        queue.push_back(child_node_id);

                        // Limit search depth to prevent infinite loops
                        if queue.len() > 10000 {
                            log::warn!("search: queue too large, stopping path search");
                            return Ok(None);
                        }
                    }
                }
            }
        }

        log::warn!("search: could not find path to target node");
        Ok(None)
    }

    /// Expand a node in the tree by its OPC UA node ID
    async fn expand_node_by_opcua_id(
        &mut self,
        opcua_node_id: &opcua::types::NodeId,
    ) -> Result<()> {
        log::info!("search: expanding node by OPC UA ID {}", opcua_node_id);

        // Find the tree node with this OPC UA node ID
        let node_index = self.tree_nodes.iter().position(|node| {
            if let Some(ref node_opcua_id) = node.opcua_node_id {
                node_opcua_id == opcua_node_id
            } else {
                false
            }
        });

        if let Some(index) = node_index {
            log::info!("search: found tree node at index {}, expanding", index);

            if self.can_expand(index) {
                self.expand_node_async(index).await?;
                log::info!("search: successfully expanded node at index {}", index);
            } else {
                log::info!(
                    "search: node at index {} cannot be expanded (already expanded or no children)",
                    index
                );
            }
        } else {
            log::warn!("search: could not find tree node with OPC UA ID {}", opcua_node_id);
        }

        Ok(())
    }
    async fn handle_progress_mouse_input(
        &mut self,
        mouse: MouseEvent,
        progress_area: Option<Rect>,
    ) -> Result<Option<ConnectionStatus>> {
        if let Some(area) = progress_area {
            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                // Check if click is OUTSIDE the progress dialog area (click outside to cancel)
                let is_outside_dialog = mouse.column < area.x
                    || mouse.column >= area.x + area.width
                    || mouse.row < area.y
                    || mouse.row >= area.y + area.height;

                if is_outside_dialog {
                    // Cancel the search only when clicking outside the dialog
                    self.cancel_search();
                }
                // If clicking inside the dialog, do nothing (don't cancel)
            }
        }
        Ok(None)
    }

    /// Ensure a node at the given index is visible by expanding parent nodes if necessary
    async fn ensure_node_visible(&mut self, node_index: usize) -> Result<()> {
        if node_index >= self.tree_nodes.len() {
            return Ok(());
        }

        let target_level = self.tree_nodes[node_index].level;

        // Find all parent nodes that need to be expanded
        let mut nodes_to_expand = Vec::new();

        // Look backwards from the target node to find parents
        for i in (0..=node_index).rev() {
            let node = &self.tree_nodes[i];
            if node.level < target_level && node.has_children && !node.is_expanded {
                // This is a parent that needs to be expanded
                nodes_to_expand.push(i);
            }
        }

        // Expand parents from top-level down
        nodes_to_expand.reverse();
        for &index in &nodes_to_expand {
            if self.can_expand(index) {
                log::info!(
                    "search: expanding parent node at index {} to make target visible",
                    index
                );
                if let Err(e) = self.expand_node_async(index).await {
                    log::error!("search: failed to expand parent node at index {}: {}", index, e);
                }
            }
        }

        Ok(())
    }

    /// Depth-first search that follows the exact tree view sort order
    /// Returns the first matching node ID, or None if no match found
    #[allow(dead_code)]
    async fn depth_first_search(
        &mut self,
        start_node_id: &opcua::types::NodeId,
        query: &str,
    ) -> Result<Option<String>> {
        log::info!("search: starting depth-first search from node {}", start_node_id);

        // Stack for depth-first traversal: (node_id, depth)
        let mut stack = Vec::new();
        let mut visited_count = 0;

        // Start with children of the selected node (don't search the selected node itself)
        let initial_children = self.get_sorted_children(start_node_id).await?;

        // Push children in reverse order so we process them left-to-right
        for child in initial_children.into_iter().rev() {
            stack.push((child.opcua_node_id, 0));
        }

        while let Some((current_node_id, depth)) = stack.pop() {
            // Check for cancellation periodically
            if self.search_cancelled {
                log::info!("search: cancelled by user");
                return Ok(None);
            }

            visited_count += 1;

            // Update progress every 10 nodes and yield to allow UI updates
            if visited_count % 10 == 0 {
                self.search_progress_current = visited_count;
                self.search_progress_total = visited_count + stack.len();
                self.search_progress_message =
                    format!("Searching node {} (depth {})...", current_node_id, depth);

                // Yield to allow UI updates (non-blocking)
                tokio::task::yield_now().await;
            }

            log::info!(
                "search: processing node {} at depth {} (visit #{})",
                current_node_id,
                depth,
                visited_count
            );

            // Check if this node matches the search criteria
            if self
                .node_matches_opcua_node(&current_node_id, query)
                .await?
            {
                log::info!("search: found match at node {}", current_node_id);
                return Ok(Some(current_node_id.to_string()));
            }

            // Yield more frequently for better UI responsiveness
            if visited_count % 5 == 0 {
                tokio::task::yield_now().await;
            }

            // Only get children if this node type would be expandable in the tree view
            // This prevents us from searching in method parameters and other non-displayed nodes
            if self.should_expand_node_in_search(&current_node_id).await? {
                let children = self.get_sorted_children(&current_node_id).await?;
                log::debug!(
                    "search: node {} has {} children",
                    current_node_id,
                    children.len()
                );

                // Add children to stack in reverse order for left-to-right processing
                for (i, child) in children.into_iter().enumerate().rev() {
                    log::debug!(
                        "search: adding child {} to stack at position {}",
                        child.opcua_node_id,
                        i
                    );
                    stack.push((child.opcua_node_id, depth + 1));
                }
            } else {
                log::debug!(
                    "search: skipping children of node {} (not expandable in tree view)",
                    current_node_id
                );
            }

            // Prevent infinite loops by limiting depth
            if depth > 20 {
                log::warn!("search: reached maximum depth, continuing with other branches");
                continue;
            }
        }

        log::info!(
            "search: depth-first search completed, no matches found (visited {} nodes)",
            visited_count
        );
        Ok(None)
    }

    /// Check if a node should be expanded during search (based on tree view logic)
    async fn should_expand_node_in_search(&self, node_id: &opcua::types::NodeId) -> Result<bool> {
        let client_guard = self.client.read().await;

        // Get the node class to determine if it should be expandable
        if let Ok(attributes) = client_guard.read_node_attributes(node_id).await {
            for attr in &attributes {
                if attr.name.to_lowercase() == "nodeclass" {
                    // Parse the node class from the attribute value
                    return match attr.value.to_lowercase().as_str() {
                        "object" => Ok(true),
                        "view" => Ok(true),
                        "objecttype" => Ok(true),
                        "variabletype" => Ok(true),
                        "datatype" => Ok(true),
                        "referencetype" => Ok(true),
                        "method" => Ok(false), // Methods don't expand in tree view
                        "variable" => Ok(false), // Variables don't expand in tree view
                        _ => Ok(true),         // Default to expandable for unknown types
                    }
                }
            }
        }

        // Fallback: assume expandable if we can't determine the node class
        Ok(true)
    }

    /// Get children of a node in the same sort order as the tree view
    async fn get_sorted_children(
        &self,
        node_id: &opcua::types::NodeId,
    ) -> Result<Vec<ChildNodeInfo>> {
        let client_guard = self.client.read().await;

        // Browse the node to get its children
        let browse_results = match client_guard.browse_node(node_id).await {
            Ok(results) => results,
            Err(e) => {
                log::debug!("search: failed to browse node {}: {}", node_id, e);
                return Ok(Vec::new());
            }
        };

        let mut children = Vec::new();

        for result in browse_results {
            // Skip nodes that shouldn't be displayed in the tree
            let node_type = match result.node_class {
                opcua::types::NodeClass::Object => super::types::NodeType::Object,
                opcua::types::NodeClass::Variable => super::types::NodeType::Variable,
                opcua::types::NodeClass::Method => super::types::NodeType::Method,
                opcua::types::NodeClass::View => super::types::NodeType::View,
                opcua::types::NodeClass::ObjectType => super::types::NodeType::ObjectType,
                opcua::types::NodeClass::VariableType => super::types::NodeType::VariableType,
                opcua::types::NodeClass::DataType => super::types::NodeType::DataType,
                opcua::types::NodeClass::ReferenceType => super::types::NodeType::ReferenceType,
                _ => continue, // Skip unknown node types
            };

            // Create a temporary tree node to check if it would be displayed
            let temp_tree_node = super::types::TreeNode {
                name: result.display_name.clone(),
                node_id: result.node_id.to_string(),
                opcua_node_id: Some(result.node_id.clone()),
                node_type: node_type.clone(),
                level: 0, // Not relevant for this check
                has_children: result.has_children,
                is_expanded: false,
                parent_path: String::new(),
            };

            // Only include nodes that would actually be displayed in the tree
            // This excludes method parameters and other nodes that aren't shown in the UI
            if self.should_include_node_in_search(&temp_tree_node) {
                log::debug!(
                    "search: including child {} DisplayName:'{}' BrowseName:'{}' Type:{:?}",
                    result.node_id,
                    result.display_name,
                    result.browse_name,
                    node_type
                );

                children.push(ChildNodeInfo {
                    opcua_node_id: result.node_id.clone(),
                    display_name: result.display_name.clone(),
                    node_type,
                });
            } else {
                log::debug!("search: skipping child {} DisplayName:'{}' BrowseName:'{}' Type:{:?} (not displayed in tree)", 
                           result.node_id, result.display_name, result.browse_name, node_type);
            }
        }

        // Sort children using the exact same logic as the tree view
        children.sort_by(|a, b| {
            let type_order_a = a.node_type.get_sort_priority();
            let type_order_b = b.node_type.get_sort_priority();

            match type_order_a.cmp(&type_order_b) {
                std::cmp::Ordering::Equal => {
                    // If same type, sort by display name (case-insensitive)
                    a.display_name
                        .to_lowercase()
                        .cmp(&b.display_name.to_lowercase())
                }
                other => other,
            }
        });

        log::debug!("search: sorted {} children for node {}", children.len(), node_id);
        for (i, child) in children.iter().enumerate() {
            log::debug!(
                "search: child {}: {} - '{}' (priority: {})",
                i,
                child.opcua_node_id,
                child.display_name,
                child.node_type.get_sort_priority()
            );
        }
        Ok(children)
    }

    /// Check if a node should be included in search (same logic as tree display)
    fn should_include_node_in_search(&self, node: &super::types::TreeNode) -> bool {
        // For now, include all node types that are displayed in the tree
        // In the future, we might want to exclude certain types based on their parent
        // For example, method input/output parameters are typically Variable nodes under Methods
        // but they're not displayed in the tree because Methods don't expand
        match node.node_type {
            super::types::NodeType::Object => true,
            super::types::NodeType::Variable => true, // Include variables, but they might be filtered by parent context
            super::types::NodeType::Method => true,
            super::types::NodeType::View => true,
            super::types::NodeType::ObjectType => true,
            super::types::NodeType::VariableType => true,
            super::types::NodeType::DataType => true,
            super::types::NodeType::ReferenceType => true,
        }
    }

    /// Check if an OPC UA node matches the search query
    async fn node_matches_opcua_node(
        &self,
        node_id: &opcua::types::NodeId,
        query: &str,
    ) -> Result<bool> {
        let client_guard = self.client.read().await;

        // Check NodeId string representation
        let node_id_str = node_id.to_string().to_lowercase();
        log::debug!("search: checking node {} NodeId '{}'", node_id, node_id_str);
        if node_id_str.contains(query) {
            log::info!(
                "search: node {} matches on NodeId '{}' contains '{}'",
                node_id,
                node_id_str,
                query
            );
            return Ok(true);
        }

        // Get the node's own attributes to check browse name and display name
        // We need to read the node's attributes, not browse its children
        if let Ok(attributes) = client_guard.read_node_attributes(node_id).await {
            for attr in &attributes {
                // Check BrowseName attribute
                if attr.name.to_lowercase() == "browsename" {
                    let browse_name = attr.value.to_lowercase();
                    log::debug!("search: node {} BrowseName '{}'", node_id, browse_name);
                    if browse_name.contains(query) {
                        log::info!(
                            "search: node {} matches on BrowseName '{}' contains '{}'",
                            node_id,
                            browse_name,
                            query
                        );
                        return Ok(true);
                    }
                }

                // Check DisplayName attribute
                if attr.name.to_lowercase() == "displayname" {
                    let display_name = attr.value.to_lowercase();
                    log::debug!("search: node {} DisplayName '{}'", node_id, display_name);
                    if display_name.contains(query) {
                        log::info!(
                            "search: node {} matches on DisplayName '{}' contains '{}'",
                            node_id,
                            display_name,
                            query
                        );
                        return Ok(true);
                    }
                }

                // If "Also look at values" is checked, search in other attribute values
                if self.search_include_values && attr.name.to_lowercase() == "value" {
                    let value_str = attr.value.to_lowercase();
                    log::debug!("search: node {} Value attribute '{}'", node_id, value_str);
                    if value_str.contains(query) {
                        log::info!(
                            "search: node {} matches on Value attribute '{}' contains '{}'",
                            node_id,
                            value_str,
                            query
                        );
                        return Ok(true);
                    }
                }
            }
        } else {
            // Fallback: try browsing to get basic info if read_node_attributes fails
            log::debug!(
                "search: failed to read attributes for node {}, trying browse fallback",
                node_id
            );

            // Get the parent of this node and browse it to find this node's info
            if let Some(parent_id) = self.find_parent_node_id(node_id).await? {
                if let Ok(browse_results) = client_guard.browse_node(&parent_id).await {
                    for result in browse_results {
                        if result.node_id == *node_id {
                            // Check BrowseName
                            let browse_name = result.browse_name.to_lowercase();
                            log::debug!(
                                "search: node {} BrowseName (from browse) '{}'",
                                node_id,
                                browse_name
                            );
                            if browse_name.contains(query) {
                                log::info!("search: node {} matches on BrowseName (from browse) '{}' contains '{}'", node_id, browse_name, query);
                                return Ok(true);
                            }

                            // Check DisplayName
                            let display_name = result.display_name.to_lowercase();
                            log::debug!(
                                "search: node {} DisplayName (from browse) '{}'",
                                node_id,
                                display_name
                            );
                            if display_name.contains(query) {
                                log::info!("search: node {} matches on DisplayName (from browse) '{}' contains '{}'", node_id, display_name, query);
                                return Ok(true);
                            }
                            break;
                        }
                    }
                }
            }
        }

        log::debug!("search: node {} does not match query '{}'", node_id, query);
        Ok(false)
    }

    /// Find the parent node ID of a given node (helper for node matching)
    async fn find_parent_node_id(
        &self,
        _target_node_id: &opcua::types::NodeId,
    ) -> Result<Option<opcua::types::NodeId>> {
        // This is a simplified implementation - in a full implementation, we might
        // traverse up the reference hierarchy to find the parent
        // For now, we'll return None to skip this fallback
        Ok(None)
    }

    /// Continue searching from the currently selected node (like Windows F3)
    async fn continue_search(&mut self) -> Result<()> {
        let query = self.last_search_query.clone();
        log::info!("search: continuing search for query '{}'", query);

        if query.is_empty() {
            log::info!("search: no previous search query");
            return Ok(());
        }

        // Clear previous results
        self.search_results.clear();
        self.current_search_index = 0;

        // Check if we have a connection and tree data
        let has_connection = {
            let client_guard = self.client.read().await;
            client_guard.is_connected()
        };

        if has_connection && !self.tree_nodes.is_empty() {
            // Get the start node for continuing the search
            let start_node_id =
                if let Some(current_node) = self.tree_nodes.get(self.selected_node_index) {
                    if let Some(ref opcua_node_id) = current_node.opcua_node_id {
                        log::info!(
                            "search: continuing from selected node '{}' ({})",
                            current_node.name,
                            opcua_node_id
                        );
                        opcua_node_id.clone()
                    } else {
                        log::warn!("search: selected node has no OPC UA node ID, using ObjectsFolder");
                        opcua::types::ObjectId::ObjectsFolder.into()
                    }
                } else {
                    log::info!("search: no selected node, starting from ObjectsFolder");
                    opcua::types::ObjectId::ObjectsFolder.into()
                };

            // Start background search from the current position
            let options = super::recursive_search::RecursiveSearchOptions {
                query,
                include_values: self.search_include_values,
                start_node_id,
            };

            self.start_background_search(options)?;
        } else {
            log::warn!("search: cannot continue search, no connection or tree data");
        }

        Ok(())
    }

    /// Continue search from the current position following tree order
    #[allow(dead_code)]
    async fn continue_tree_search(&mut self, query: &str) -> Result<Option<String>> {
        log::info!(
            "search: continuing tree search from selected index {}",
            self.selected_node_index
        );

        let start_node_id =
            if let Some(current_node) = self.tree_nodes.get(self.selected_node_index) {
                if let Some(ref opcua_node_id) = current_node.opcua_node_id {
                    log::info!(
                        "search: starting from selected node '{}' ({})",
                        current_node.name,
                        opcua_node_id
                    );
                    opcua_node_id.clone()
                } else {
                    log::warn!("search: selected node has no OPC UA node ID, using ObjectsFolder");
                    opcua::types::ObjectId::ObjectsFolder.into()
                }
            } else {
                log::info!("search: no selected node, starting from ObjectsFolder");
                opcua::types::ObjectId::ObjectsFolder.into()
            };

        // First, search in the children of the current node (if it has expandable children)
        if let Some(current_node) = self.tree_nodes.get(self.selected_node_index) {
            if current_node.should_show_expand_indicator() && current_node.has_children {
                log::info!("search: searching in children of current node {}", start_node_id);
                if let Some(found_id) = self.depth_first_search(&start_node_id, query).await? {
                    return Ok(Some(found_id));
                }
            }
        }

        // If no match in children, search the remaining tree starting from the root
        // and skip everything up to and including the current node
        log::info!("search: no match in children, searching remaining tree");
        self.search_remaining_tree(&start_node_id, query).await
    }

    /// Search the remaining tree after the current node
    #[allow(dead_code)]
    async fn search_remaining_tree(
        &mut self,
        current_node_id: &opcua::types::NodeId,
        query: &str,
    ) -> Result<Option<String>> {
        log::info!("search: searching remaining tree after node {}", current_node_id);

        // Start from the root and perform a full DFS, but skip nodes until we're past the current node
        let root_node_id = opcua::types::ObjectId::ObjectsFolder.into();

        // Get all top-level children of the root
        let root_children = self.get_sorted_children(&root_node_id).await?;

        // Find which top-level node contains our current node
        let current_tree_node = self.tree_nodes.get(self.selected_node_index);
        let current_level_0_parent = if let Some(current_node) = current_tree_node {
            if current_node.level == 0 {
                // Current node is at level 0, so we look for siblings after it
                current_node.opcua_node_id.clone()
            } else {
                // Find the level-0 parent
                self.find_level_0_parent(self.selected_node_index)
            }
        } else {
            None
        };

        if let Some(level_0_parent) = current_level_0_parent {
            // Find the index of the current level-0 parent in the root children
            let parent_index = root_children.iter().position(|child| {
                Some(child.opcua_node_id.clone()) == Some(level_0_parent.clone())
            });

            if let Some(parent_idx) = parent_index {
                // Search in all the siblings that come after the current parent
                for sibling in root_children.iter().skip(parent_idx + 1) {
                    log::info!(
                        "search: searching in next top-level sibling {}",
                        sibling.opcua_node_id
                    );

                    // Check if this sibling matches
                    if self
                        .node_matches_opcua_node(&sibling.opcua_node_id, query)
                        .await?
                    {
                        log::info!(
                            "search: found match in top-level sibling {}",
                            sibling.opcua_node_id
                        );
                        return Ok(Some(sibling.opcua_node_id.to_string()));
                    }

                    // Search in the sibling's subtree
                    if let Some(found_id) = self
                        .depth_first_search(&sibling.opcua_node_id, query)
                        .await?
                    {
                        return Ok(Some(found_id));
                    }
                }
            }
        }

        // No more matches found
        Ok(None)
    }

    /// Find the level-0 parent of a node at the given index
    #[allow(dead_code)]
    fn find_level_0_parent(&self, node_index: usize) -> Option<opcua::types::NodeId> {
        if let Some(node) = self.tree_nodes.get(node_index) {
            if node.level == 0 {
                return node.opcua_node_id.clone();
            } else {
                // Walk backwards to find the level-0 parent
                for i in (0..node_index).rev() {
                    if let Some(parent_candidate) = self.tree_nodes.get(i) {
                        if parent_candidate.level == 0 {
                            return parent_candidate.opcua_node_id.clone();
                        }
                    }
                }
            }
        }
        None
    }

    /// Wrap search to the beginning when no more matches found
    #[allow(dead_code)]
    async fn wrap_search_to_beginning(&mut self, query: &str) -> Result<()> {
        log::info!("search: wrapping search to beginning of tree");

        // Start from the root of the tree
        let root_node_id = opcua::types::ObjectId::ObjectsFolder.into();

        // Update progress message to show wrapping
        self.search_progress_message = format!("Wrapping search for '{}'...", query);

        if let Some(found_node_id) = self.depth_first_search(&root_node_id, query).await? {
            log::info!("search: found match after wrapping {}", found_node_id);
            self.search_results.push(found_node_id.clone());

            // Navigate to the found node
            self.expand_to_find_node(&found_node_id).await?;
        } else {
            log::info!("search: no matches found in entire tree");
            // Update progress message to show no results
            self.search_progress_message = format!("No matches found for '{}'", query);

            // Keep the progress dialog open for a moment so user can see the "no matches" message
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }

        Ok(())
    }
}
