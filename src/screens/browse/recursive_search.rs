use super::types::{BrowseScreen, SearchCommand, SearchMessage};
use crate::client::OpcUaClientManager;
use anyhow::Result;
use opcua::types::NodeId;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub struct RecursiveSearchOptions {
    pub query: String,
    pub include_values: bool,
    pub start_node_id: NodeId,
}

impl BrowseScreen {
    /// Start a background search task that communicates via channels
    pub fn start_background_search(&mut self, options: RecursiveSearchOptions) -> Result<()> {
        log::info!(
            "search: starting background search for '{}' from node '{}'",
            options.query,
            options.start_node_id
        );

        // Create channels for communication
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<SearchCommand>();
        let (message_tx, message_rx) = mpsc::unbounded_channel::<SearchMessage>();

        // Store the channels
        self.search_command_tx = Some(command_tx);
        self.search_message_rx = Some(message_rx);

        // Reset search state
        self.search_cancelled = false;
        self.search_progress_open = true;
        self.search_progress_message = format!("Searching for '{}'...", options.query);
        self.search_results.clear();
        self.current_search_index = 0;

        // Clone data needed for the background task
        let client = self.client.clone();
        let tree_nodes = self.tree_nodes.clone();
        let selected_node_index = self.selected_node_index;

        // Spawn the background search task
        let message_tx_clone = message_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::background_search_task(
                options,
                client,
                tree_nodes,
                selected_node_index,
                message_tx_clone,
                &mut command_rx,
            )
            .await
            {
                log::error!("search: background search task failed: {}", e);
                // Try to send a complete message even on error (using the original sender from the closure)
            }
        });

        Ok(())
    }

    /// Process messages from the background search task
    pub async fn process_search_messages(&mut self) {
        let mut should_close_search = false;
        let mut close_reason = SearchMessage::Complete;
        let mut first_result_found = false;
        let mut first_result_node_id: Option<String> = None;

        if let Some(rx) = &mut self.search_message_rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    SearchMessage::Progress {
                        current: _,
                        total: _,
                        current_node,
                    } => {
                        self.search_progress_message = format!("{}", current_node);
                    }
                    SearchMessage::Result { node_id } => {
                        let is_first_result = self.search_results.is_empty();
                        self.search_results.push(node_id.clone());
                        log::info!(
                            "search: result #{} found: {} (navigating to first result)",
                            self.search_results.len(),
                            node_id
                        );

                        // Store the first result to navigate to it after processing messages
                        if is_first_result {
                            log::info!(
                                "search: first result found {}, will navigate immediately",
                                node_id
                            );
                            first_result_found = true;
                            first_result_node_id = Some(node_id);

                            // Stop the search after finding the first result (Windows behavior)
                            if let Some(tx) = &self.search_command_tx {
                                let _ = tx.send(super::types::SearchCommand::Cancel);
                                log::info!("search: sent cancel command to stop search after first result");
                            }
                        }

                        // Limit results to avoid overwhelming the user
                        if self.search_results.len() >= 50 {
                            // Stop after finding 50 results
                            if let Some(tx) = &self.search_command_tx {
                                let _ = tx.send(super::types::SearchCommand::Cancel);
                                log::info!("search: stopping after finding first result due to result limit (50)");
                            }
                        }
                    }
                    SearchMessage::Complete => {
                        should_close_search = true;
                        close_reason = SearchMessage::Complete;
                        break;
                    }
                    SearchMessage::Cancelled => {
                        should_close_search = true;
                        close_reason = SearchMessage::Cancelled;
                        self.search_cancelled = true;
                        break;
                    }
                }
            }
        }

        // Navigate to the first result if we found one
        if let Some(node_id) = first_result_node_id {
            log::info!("search: navigating to first search result '{}'", node_id);
            if let Err(e) = self.expand_to_find_node(&node_id).await {
                log::error!("search: failed to navigate to first search result: {}", e);
            } else {
                log::info!("search: successfully navigated to search result");
            }
        }

        // Close search after we're done with the receiver
        if should_close_search {
            self.search_progress_open = false;
            self.search_command_tx = None;
            self.search_message_rx = None;
            match close_reason {
                SearchMessage::Complete => {
                    if !first_result_found && self.search_results.is_empty() {
                        log::info!("search: completed, no results found");
                    } else {
                        log::info!(
                            "search: completed with {} results found",
                            self.search_results.len()
                        );
                    }
                }
                SearchMessage::Cancelled => {
                    if first_result_found {
                        log::info!("search: stopped after finding first result");
                    } else {
                        log::info!("search: cancelled by user");
                    }
                }
                _ => {}
            }
        }
    }

    /// Background task that performs the actual search
    async fn background_search_task(
        options: RecursiveSearchOptions,
        client: Arc<RwLock<OpcUaClientManager>>,
        tree_nodes: Vec<super::types::TreeNode>,
        selected_node_index: usize,
        message_tx: mpsc::UnboundedSender<SearchMessage>,
        command_rx: &mut mpsc::UnboundedReceiver<SearchCommand>,
    ) -> Result<()> {
        // Check if client is connected
        let is_connected = {
            let client_guard = client.read().await;
            client_guard.is_connected()
        };

        if !is_connected {
            log::error!("search: client is not connected, cannot perform recursive search");
            let _ = message_tx.send(SearchMessage::Complete);
            return Err(anyhow::anyhow!("OPC UA client is not connected"));
        }

        let query_lower = options.query.to_lowercase();
        let mut cancelled = false;

        log::info!("search: starting background search");
        log::info!(
            "Query: '{}' (include values: {})",
            options.query,
            options.include_values
        );
        log::info!("search: starting from node '{}'", options.start_node_id);
        log::info!(
            "Selected node index: {} / {}",
            selected_node_index,
            tree_nodes.len()
        );

        // Start the depth-first search following the exact algorithm
        if let Some(found_node_id) = Self::depth_first_search_algorithm(
            &options.start_node_id,
            &query_lower,
            options.include_values,
            &client,
            &tree_nodes,
            &message_tx,
            command_rx,
            &mut cancelled,
        )
        .await?
        {
            log::info!("search: found result '{}'", found_node_id);
            let _ = message_tx.send(SearchMessage::Result {
                node_id: found_node_id,
            });
        } else {
            log::info!("search: no match found");
        }

        // Send completion message
        if cancelled {
            let _ = message_tx.send(SearchMessage::Cancelled);
        } else {
            let _ = message_tx.send(SearchMessage::Complete);
        }
        Ok(())
    }

    /// Implements the exact depth-first search algorithm from the pseudocode
    async fn depth_first_search_algorithm(
        selected_node_id: &NodeId,
        query: &str,
        search_by_value: bool,
        client: &Arc<RwLock<OpcUaClientManager>>,
        tree_nodes: &[super::types::TreeNode],
        message_tx: &mpsc::UnboundedSender<SearchMessage>,
        command_rx: &mut mpsc::UnboundedReceiver<SearchCommand>,
        cancelled: &mut bool,
    ) -> Result<Option<String>> {
        log::debug!("search: starting depth-first algorithm");
        log::debug!("search: starting from selected node '{}'", selected_node_id);

        // 1️⃣ Search *descendants* of the selected node
        log::debug!("search: step 1 - searching descendants of selected node");
        let children = Self::get_visible_children_sorted(selected_node_id, client).await?;
        log::debug!("search: found {} visible children to search", children.len());

        for child in children.iter() {
            // Check for cancellation
            if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                *cancelled = true;
                return Ok(None);
            }

            if let Some(found) = Self::search_in_node_recursive(
                &child.opcua_node_id,
                query,
                search_by_value,
                client,
                message_tx,
                command_rx,
                cancelled,
            )
            .await?
            {
                log::info!("search: found match in descendants '{}'", found);
                return Ok(Some(found));
            }

            if *cancelled {
                return Ok(None);
            }
        }

        log::debug!("search: step 1 complete - no match found in descendants");

        // 2️⃣ No luck below: walk *upwards* and scan remaining siblings
        log::debug!("search: step 2 - walking upwards and scanning remaining siblings");
        let mut current_node_id = selected_node_id.clone();

        loop {
            // Check for cancellation
            if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                *cancelled = true;
                return Ok(None);
            }

            // Find parent of current node
            let parent_node_id =
                match Self::find_parent_node_id_in_tree(&current_node_id, client, tree_nodes)
                    .await?
                {
                    Some(parent) => parent,
                    None => {
                        // We're at root level - search remaining siblings at root level
                        log::debug!("search: at root level, searching remaining root-level siblings");

                        // Find all root level nodes (level 0) and search the ones after current
                        let root_siblings: Vec<_> = tree_nodes
                            .iter()
                            .filter(|node| node.level == 0 && node.opcua_node_id.is_some())
                            .collect();

                        // Find current position among root siblings
                        let current_position = root_siblings
                            .iter()
                            .position(|node| {
                                if let Some(opcua_id) = &node.opcua_node_id {
                                    *opcua_id == current_node_id
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(0);

                        // Search remaining root siblings
                        let remaining_root_siblings = &root_siblings[(current_position + 1)..];
                        log::info!(
                            "Searching {} remaining root-level siblings",
                            remaining_root_siblings.len()
                        );

                        for sibling in remaining_root_siblings.iter() {
                            // Check for cancellation
                            if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                                *cancelled = true;
                                return Ok(None);
                            }

                            if let Some(sibling_opcua_id) = &sibling.opcua_node_id {
                                if let Some(found) = Self::search_in_node_recursive(
                                    sibling_opcua_id,
                                    query,
                                    search_by_value,
                                    client,
                                    message_tx,
                                    command_rx,
                                    cancelled,
                                )
                                .await?
                                {
                                    log::info!("search: found match in root sibling subtree '{}'", found);
                                    return Ok(Some(found));
                                }

                                if *cancelled {
                                    return Ok(None);
                                }
                            }
                        }

                        // Finished searching all root siblings, we're done
                        log::debug!("search: finished searching all root-level siblings");
                        break;
                    }
                };

            // Don't go ABOVE the Objects folder ("i=85") - but we can search its siblings
            let objects_node_id = opcua::types::NodeId::new(0, 85u32);
            if parent_node_id == objects_node_id {
                log::debug!("search: reached Objects folder boundary, stopping upward traversal");
                break;
            }

            log::debug!("search: searching siblings under parent '{}'", parent_node_id);

            // Get all visible siblings (children of parent), already sorted
            let siblings = Self::get_visible_children_sorted(&parent_node_id, client).await?;

            // Find the position of current node in siblings
            let current_position = siblings
                .iter()
                .position(|n| n.opcua_node_id == current_node_id)
                .unwrap_or(0);

            log::info!(
                "Current node position in siblings: {} / {}",
                current_position,
                siblings.len()
            );

            // Start with the sibling *after* the one we just finished
            let remaining_siblings = &siblings[(current_position + 1)..];
            log::debug!("search: searching {} remaining siblings", remaining_siblings.len());

            for sibling in remaining_siblings.iter() {
                // Check for cancellation
                if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                    *cancelled = true;
                    return Ok(None);
                }

                if let Some(found) = Self::search_in_node_recursive(
                    &sibling.opcua_node_id,
                    query,
                    search_by_value,
                    client,
                    message_tx,
                    command_rx,
                    cancelled,
                )
                .await?
                {
                    log::info!("search: found match in sibling subtree '{}'", found);
                    return Ok(Some(found));
                }

                if *cancelled {
                    return Ok(None);
                }
            }

            // Nothing on this level; climb one level up and loop
            current_node_id = parent_node_id;
            log::debug!("search: moving up to next level '{}'", current_node_id);
        }

        log::info!("search: completed, no matches found");
        Ok(None)
    }

    /// Returns the first matching descendant of `node`, or `None` (iterative depth-first using stack)
    async fn search_in_node_recursive(
        start_node_id: &NodeId,
        query: &str,
        search_by_value: bool,
        client: &Arc<RwLock<OpcUaClientManager>>,
        message_tx: &mpsc::UnboundedSender<SearchMessage>,
        command_rx: &mut mpsc::UnboundedReceiver<SearchCommand>,
        cancelled: &mut bool,
    ) -> Result<Option<String>> {
        // Use iterative approach with a stack to avoid async recursion issues
        let mut stack = vec![start_node_id.clone()];

        while let Some(current_node_id) = stack.pop() {
            // Check for cancellation
            if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                *cancelled = true;
                return Ok(None);
            }

            log::debug!("search: searching in node '{}'", current_node_id);

            // Check if this node matches
            if Self::is_match(&current_node_id, query, search_by_value, client, message_tx).await {
                log::info!(
                    "🎯 MATCH FOUND: {} matches query '{}'",
                    current_node_id,
                    query
                );
                return Ok(Some(current_node_id.to_string()));
            } else {
                log::debug!("search: no match in node '{}'", current_node_id);
            }

            // Check if this node is a Method - if so, skip its children (Input/Output arguments)
            let client_guard = client.read().await;
            let should_skip_children = if let Ok((_, _, _, node_class)) = client_guard
                .read_node_search_attributes(&current_node_id, false)
                .await
            {
                matches!(node_class, opcua::types::NodeClass::Method)
            } else {
                false // If we can't read the node class, assume it's not a Method
            };
            drop(client_guard);

            if should_skip_children {
                log::debug!("search: skipping children of Method node '{}'", current_node_id);
                continue; // Skip to next node in stack without adding children
            }

            // Add children to stack (in reverse order for depth-first left-to-right traversal)
            let children = Self::get_visible_children_sorted(&current_node_id, client).await?;
            log::info!(
                "Node {} has {} visible children",
                current_node_id,
                children.len()
            );

            for child in children.into_iter().rev() {
                stack.push(child.opcua_node_id);
            }
        }

        log::info!(
            "🚫 No match found in subtree starting from {}",
            start_node_id
        );
        Ok(None)
    }

    /// Get visible children of a node, sorted in tree display order
    async fn get_visible_children_sorted(
        node_id: &NodeId,
        client: &Arc<RwLock<OpcUaClientManager>>,
    ) -> Result<Vec<ChildNodeInfo>> {
        let client_guard = client.read().await;

        // Browse the node to get its children
        let browse_results = match client_guard.browse_node(node_id).await {
            Ok(results) => results,
            Err(e) => {
                log::debug!("search: failed to browse node '{}': {}", node_id, e);
                return Ok(Vec::new());
            }
        };

        let mut children = Vec::new();

        for result in browse_results {
            // Convert to our node type for consistent sorting
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

            children.push(ChildNodeInfo {
                opcua_node_id: result.node_id.clone(),
                node_id: result.node_id.to_string(),
                browse_name: result.browse_name.clone(),
                display_name: result.display_name.clone(),
                node_class: format!("{:?}", result.node_class),
                node_type,
                has_children: result.has_children,
            });
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

        log::debug!("search: sorted {} children for node '{}'", children.len(), node_id);
        Ok(children)
    }

    /// Find the parent node ID by looking it up in the loaded tree structure
    async fn find_parent_node_id_in_tree(
        target_node_id: &NodeId,
        _client: &Arc<RwLock<OpcUaClientManager>>,
        tree_nodes: &[super::types::TreeNode],
    ) -> Result<Option<NodeId>> {
        // Since we're searching within the tree view, the parent should be visible in the tree
        // Find the target node in the tree and get its parent

        for (i, node) in tree_nodes.iter().enumerate() {
            if let Some(node_opcua_id) = &node.opcua_node_id {
                if *node_opcua_id == *target_node_id {
                    // Found the target node, now find its parent by looking at indentation levels
                    let target_level = node.level;

                    if target_level == 0 {
                        // This is a root level node, no parent to search beyond
                        return Ok(None);
                    }

                    // Look backwards in the tree to find the parent (one level up)
                    for j in (0..i).rev() {
                        if tree_nodes[j].level == target_level - 1 {
                            if let Some(parent_opcua_id) = &tree_nodes[j].opcua_node_id {
                                log::debug!(
                                    "Found parent of {} (level {}) -> {} (level {})",
                                    target_node_id,
                                    target_level,
                                    parent_opcua_id,
                                    tree_nodes[j].level
                                );
                                return Ok(Some(parent_opcua_id.clone()));
                            }
                        }
                    }

                    // If we didn't find a parent in the tree, it might be a root node
                    log::debug!(
                        "No parent found in tree for node {} at level {}",
                        target_node_id,
                        target_level
                    );
                    return Ok(None);
                }
            }
        }

        // Node not found in tree - this shouldn't happen during tree search
        // Fall back to the simplified heuristic
        log::debug!(
            "Node {} not found in loaded tree, using fallback logic",
            target_node_id
        );

        match target_node_id {
            // Objects folder has no parent we search beyond
            node_id if *node_id == opcua::types::NodeId::new(0, 85u32) => Ok(None),
            // For other nodes, assume their parent is the Objects folder for simplicity
            _ => Ok(Some(opcua::types::NodeId::new(0, 85u32))), // Objects folder
        }
    }

    /// Text comparison helper (case-insensitive)
    async fn is_match(
        node_id: &NodeId,
        query: &str,
        search_by_value: bool,
        client: &Arc<RwLock<OpcUaClientManager>>,
        message_tx: &mpsc::UnboundedSender<SearchMessage>,
    ) -> bool {
        let client_guard = client.read().await;
        let query_lower = query.to_ascii_lowercase();

        // Use the lightweight method to read BrowseName, DisplayName, NodeClass, and optionally Value
        if let Ok((browse_name, display_name, value_attr, _node_class)) = client_guard
            .read_node_search_attributes(node_id, search_by_value)
            .await
        {
            // Send progress message with the current node being searched (DisplayName + NodeId)
            let progress_text = format!("{} [{}]", display_name, node_id);
            let _ = message_tx.send(SearchMessage::Progress {
                current: 0, // Not used anymore
                total: 0,   // Not used anymore
                current_node: progress_text,
            });

            // Check NodeId
            let node_id_str = node_id.to_string().to_ascii_lowercase();
            if node_id_str.contains(&query_lower) {
                log::info!("search: NodeId match '{}' contains '{}'", node_id_str, query);
                return true;
            }

            // Check BrowseName
            let browse_name_lower = browse_name.to_ascii_lowercase();
            if browse_name_lower.contains(&query_lower) {
                log::info!(
                    "search: BrowseName match '{}' contains '{}'",
                    browse_name_lower,
                    query
                );
                return true;
            }

            // Check DisplayName
            let display_name_lower = display_name.to_ascii_lowercase();
            if display_name_lower.contains(&query_lower) {
                log::info!(
                    "search: DisplayName match '{}' contains '{}'",
                    display_name_lower,
                    query
                );
                return true;
            }

            // Check Value attribute if it was requested and available
            if search_by_value {
                if let Some(value) = value_attr {
                    let value_lower = value.to_ascii_lowercase();
                    if value_lower.contains(&query_lower) {
                        log::info!(
                            "✓ Value attribute match: '{}' contains '{}'",
                            value_lower,
                            query
                        );
                        return true;
                    }
                }
            }
        }

        false
    }
}

// Helper struct for child node information
#[derive(Clone)]
pub struct ChildNodeInfo {
    pub opcua_node_id: NodeId,
    pub node_id: String,
    pub browse_name: String,
    pub display_name: String,
    pub node_class: String,
    pub node_type: super::types::NodeType,
    pub has_children: bool,
}
