use super::types::{BrowseScreen, SearchMessage, SearchCommand};
use crate::client::OpcUaClientManager;
use anyhow::Result;
use opcua::types::NodeId;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

pub struct RecursiveSearchOptions {
    pub query: String,
    pub include_values: bool,
    pub start_node_id: NodeId,
}

impl BrowseScreen {
    /// Start a background search task that communicates via channels
    pub fn start_background_search(&mut self, options: RecursiveSearchOptions) -> Result<()> {
        log::info!("Starting background search for '{}' from node {}", options.query, options.start_node_id);
        
        // Create channels for communication
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<SearchCommand>();
        let (message_tx, message_rx) = mpsc::unbounded_channel::<SearchMessage>();
        
        // Store the channels
        self.search_command_tx = Some(command_tx);
        self.search_message_rx = Some(message_rx);
        
        // Reset search state
        self.search_cancelled = false;
        self.search_progress_open = true;
        self.search_progress_current = 0;
        self.search_progress_total = 1;
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
            ).await {
                log::error!("Background search task failed: {}", e);
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
                    SearchMessage::Progress { current, total, current_node } => {
                        self.search_progress_current = current;
                        self.search_progress_total = total;
                        self.search_progress_message = format!("Searching: {}", current_node);
                    }
                    SearchMessage::Result { node_id } => {
                        let is_first_result = self.search_results.is_empty();
                        self.search_results.push(node_id.clone());
                        log::info!("Added search result: {} (total results: {})", 
                                  node_id, self.search_results.len());
                        
                        // Store the first result to navigate to it after processing messages
                        if is_first_result {
                            log::info!("First result found, will navigate after processing messages");
                            first_result_found = true;
                            first_result_node_id = Some(node_id);
                            
                            // Stop the search after finding the first result (Windows behavior)
                            if let Some(tx) = &self.search_command_tx {
                                let _ = tx.send(super::types::SearchCommand::Cancel);
                            }
                        }
                        
                        // Limit results to avoid overwhelming the user
                        if self.search_results.len() >= 50 {
                            // Stop after finding 50 results
                            if let Some(tx) = &self.search_command_tx {
                                let _ = tx.send(super::types::SearchCommand::Cancel);
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
            if let Err(e) = self.expand_to_find_node(&node_id).await {
                log::error!("Failed to navigate to first search result: {}", e);
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
                        log::info!("Search completed with no results found");
                    } else {
                        log::info!("Search completed with {} results", self.search_results.len());
                    }
                }
                SearchMessage::Cancelled => {
                    if first_result_found {
                        log::info!("Search stopped after finding first result");
                    } else {
                        log::info!("Search was cancelled");
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
            log::error!("Client is not connected, cannot perform recursive search");
            let _ = message_tx.send(SearchMessage::Complete);
            return Err(anyhow::anyhow!("OPC UA client is not connected"));
        }
        
        let query_lower = options.query.to_lowercase();
        let mut cancelled = false;
        
        log::info!("Starting search from children of selected node (skipping selected node itself)");
        
        // First, search the children of the selected node (skip the selected node itself)
        if let Err(_) = Self::search_children_iterative(
            &options.start_node_id,
            &query_lower,
            &options.query,
            options.include_values,
            &client,
            &message_tx,
            command_rx,
            &mut cancelled,
        ).await {
            if !cancelled {
                log::error!("Error during children search");
            }
        }
        
        if cancelled {
            let _ = message_tx.send(SearchMessage::Cancelled);
            return Ok(());
        }
        
        // Continue with siblings and remaining tree (skip the selected node itself)
        if selected_node_index < tree_nodes.len() {
            let start_node_level = tree_nodes[selected_node_index].level;
            
            // Collect sibling nodes to search (start from the node after selected)
            let mut nodes_to_search = Vec::new();
            for i in (selected_node_index + 1)..tree_nodes.len() {
                // Check for cancellation periodically
                if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                    let _ = message_tx.send(SearchMessage::Cancelled);
                    return Ok(());
                }
                
                let node = &tree_nodes[i];
                if node.level <= start_node_level {
                    if let Some(opcua_node_id) = &node.opcua_node_id {
                        nodes_to_search.push((opcua_node_id.clone(), node.name.clone()));
                    }
                }
            }
            
            // Search each collected node
            let total_nodes = nodes_to_search.len();
            for (i, (node_id, node_name)) in nodes_to_search.into_iter().enumerate() {
                // Check for cancellation
                if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                    let _ = message_tx.send(SearchMessage::Cancelled);
                    return Ok(());
                }
                
                // Send progress update
                let _ = message_tx.send(SearchMessage::Progress {
                    current: i + 1,
                    total: total_nodes,
                    current_node: node_name,
                });
                
                if let Err(_) = Self::search_subtree_iterative(
                    &node_id,
                    &query_lower,
                    &options.query,
                    options.include_values,
                    &client,
                    &message_tx,
                    command_rx,
                    &mut cancelled,
                ).await {
                    if cancelled {
                        let _ = message_tx.send(SearchMessage::Cancelled);
                        return Ok(());
                    }
                }
                
                if cancelled {
                    let _ = message_tx.send(SearchMessage::Cancelled);
                    return Ok(());
                }
            }
        }
        
        // Send completion message
        let _ = message_tx.send(SearchMessage::Complete);
        Ok(())
    }
    
    /// Search only the children of a node (not the node itself) - iterative version
    async fn search_children_iterative(
        parent_node_id: &NodeId,
        query_lower: &str,
        _query_original: &str,
        include_values: bool,
        client: &Arc<RwLock<OpcUaClientManager>>,
        message_tx: &mpsc::UnboundedSender<SearchMessage>,
        command_rx: &mut mpsc::UnboundedReceiver<SearchCommand>,
        cancelled: &mut bool,
    ) -> Result<()> {
        log::info!("Searching children of node: {} (skipping the node itself)", parent_node_id);
        
        // Check for cancellation before starting
        if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
            *cancelled = true;
            return Ok(());
        }
        
        // Get the children of this node
        let children = {
            let client_guard = client.read().await;
            match client_guard.browse_node(parent_node_id).await {
                Ok(browse_results) => browse_results,
                Err(e) => {
                    log::warn!("Failed to browse children of node {}: {}", parent_node_id, e);
                    return Ok(());
                }
            }
        };
        
        log::info!("Found {} children to search", children.len());
        
        // Use a queue for breadth-first search through all children and their descendants
        let mut search_queue = VecDeque::new();
        
        // Add all direct children to the queue
        for child in children {
            search_queue.push_back(child.node_id);
        }
        
        let mut processed_count = 0;
        let total_initial = search_queue.len();
        
        while let Some(current_node_id) = search_queue.pop_front() {
            // Check for cancellation
            if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                *cancelled = true;
                return Ok(());
            }
            
            processed_count += 1;
            
            // Send progress update
            let _ = message_tx.send(SearchMessage::Progress {
                current: processed_count,
                total: total_initial.max(processed_count),
                current_node: format!("Node {}", current_node_id),
            });
            
            // Check if this node matches
            if Self::node_matches_search_background(
                &current_node_id,
                query_lower,
                include_values,
                client,
            ).await {
                log::info!("Found match: {}", current_node_id);
                let _ = message_tx.send(SearchMessage::Result {
                    node_id: current_node_id.to_string(),
                });
                // Continue searching for more matches instead of returning
            }
            
            // Add children of this node to the queue for further searching
            let children = {
                let client_guard = client.read().await;
                match client_guard.browse_node(&current_node_id).await {
                    Ok(browse_results) => browse_results,
                    Err(e) => {
                        log::debug!("Failed to browse children of node {}: {}", current_node_id, e);
                        continue; // Skip this node and continue with the next one
                    }
                }
            };
            
            for child in children {
                search_queue.push_back(child.node_id);
            }
        }
        
        Ok(())
    }
    
    /// Search a single subtree iteratively 
    async fn search_subtree_iterative(
        root_node_id: &NodeId,
        query_lower: &str,
        _query_original: &str,
        include_values: bool,
        client: &Arc<RwLock<OpcUaClientManager>>,
        message_tx: &mpsc::UnboundedSender<SearchMessage>,
        command_rx: &mut mpsc::UnboundedReceiver<SearchCommand>,
        cancelled: &mut bool,
    ) -> Result<()> {
        log::info!("Search subtree called for node: {}", root_node_id);
        
        // Check for cancellation before starting
        if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
            *cancelled = true;
            return Ok(());
        }
        
        // Use a queue for breadth-first search
        let mut search_queue = VecDeque::new();
        search_queue.push_back(root_node_id.clone());
        
        let mut processed_count = 0;
        
        while let Some(current_node_id) = search_queue.pop_front() {
            // Check for cancellation
            if let Ok(SearchCommand::Cancel) = command_rx.try_recv() {
                *cancelled = true;
                return Ok(());
            }
            
            processed_count += 1;
            
            // Check if this node matches
            if Self::node_matches_search_background(
                &current_node_id,
                query_lower,
                include_values,
                client,
            ).await {
                log::info!("Found match: {}", current_node_id);
                let _ = message_tx.send(SearchMessage::Result {
                    node_id: current_node_id.to_string(),
                });
                // Continue searching for more matches instead of returning
            }
            
            // Add children of this node to the queue
            let children = {
                let client_guard = client.read().await;
                match client_guard.browse_node(&current_node_id).await {
                    Ok(browse_results) => browse_results,
                    Err(e) => {
                        log::debug!("Failed to browse children of node {}: {}", current_node_id, e);
                        continue; // Skip this node and continue with the next one
                    }
                }
            };
            
            for child in children {
                search_queue.push_back(child.node_id);
            }
        }
        
        Ok(())
    }
    
    /// Check if a node matches the search criteria (background version)
    async fn node_matches_search_background(
        node_id: &NodeId,
        query_lower: &str,
        include_values: bool,
        client: &Arc<RwLock<OpcUaClientManager>>,
    ) -> bool {
        let client_guard = client.read().await;
        
        // Check NodeId
        let node_id_str = node_id.to_string().to_lowercase();
        if node_id_str.contains(query_lower) {
            return true;
        }
        
        // Get browse results to check browse name and display name
        if let Ok(browse_results) = client_guard.browse_node(node_id).await {
            if let Some(browse_result) = browse_results.first() {
                // Check BrowseName
                if browse_result.browse_name.to_lowercase().contains(query_lower) {
                    return true;
                }
                
                // Check DisplayName
                if browse_result.display_name.to_lowercase().contains(query_lower) {
                    return true;
                }
            }
        }
        
        // Optionally check attribute values
        if include_values {
            if let Ok(attributes) = client_guard.read_node_attributes(node_id).await {
                for attr in attributes {
                    let value_str = attr.value.to_lowercase();
                    if value_str.contains(query_lower) {
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
