use super::types::{NodeAttribute, NodeType, TreeNode};
use anyhow::Result;
use opcua::types::{NodeClass, NodeId};

impl super::BrowseScreen {
    pub async fn load_real_tree(&mut self) -> Result<()> {
        self.is_loading = true;

        // Clear existing nodes
        self.tree_nodes.clear();
        self.selected_node_index = 0;
        self.expanded_nodes.clear();

        // Add timeout to the entire loading process
        let load_future = async {
            // Get the root node (Objects folder)
            let client_guard = self.client.read().await;
            if !client_guard.is_connected() {
                return Ok(Vec::new());
            }

            let root_node_id = client_guard.get_root_node().await?;
            drop(client_guard);

            // Load the root level nodes
            self.get_real_children(&root_node_id, 0, "").await
        };
        match tokio::time::timeout(tokio::time::Duration::from_secs(10), load_future).await {
            Ok(Ok(children)) => {
                self.tree_nodes = children;
            }
            Ok(Err(e)) => {
                log::warn!(
                    "Failed to load real tree data: {}. Will use demo data on expand.",
                    e
                );
                // Don't fail completely, just leave tree_nodes empty
            }
            Err(_timeout) => {
                log::warn!("Tree loading timed out. Will use demo data on expand.");
                // Don't fail completely, just leave tree_nodes empty
            }
        }

        self.is_loading = false;
        Ok(())
    }
    pub async fn get_real_children(
        &self,
        parent_node_id: &NodeId,
        level: usize,
        parent_path: &str,
    ) -> Result<Vec<TreeNode>> {
        let client_guard = self.client.read().await;
        if !client_guard.is_connected() {
            return Ok(Vec::new());
        }

        let opcua_nodes = client_guard.browse_node(parent_node_id).await?;
        drop(client_guard);        let mut tree_nodes = Vec::new();
        for opcua_node in opcua_nodes {
            let node_type = match opcua_node.node_class {
                NodeClass::Object => NodeType::Object,
                NodeClass::Variable => NodeType::Variable,
                NodeClass::Method => NodeType::Method,
                NodeClass::View => NodeType::View,
                NodeClass::ObjectType => NodeType::ObjectType,
                NodeClass::VariableType => NodeType::VariableType,
                NodeClass::DataType => NodeType::DataType,
                NodeClass::ReferenceType => NodeType::ReferenceType,
                _ => NodeType::Object, // Default fallback
            };

            let display_name = if opcua_node.display_name.is_empty() {
                opcua_node.browse_name.clone()
            } else {
                opcua_node.display_name.clone()
            };

            tree_nodes.push(TreeNode {
                name: display_name,
                node_id: opcua_node.node_id.to_string(),
                opcua_node_id: Some(opcua_node.node_id),
                node_type,
                level,
                has_children: opcua_node.has_children,
                is_expanded: false,
                parent_path: parent_path.to_string(),
            });
        }        // Sort nodes by type priority, then by name
        tree_nodes.sort_by(|a, b| {
            let type_order_a = a.node_type.get_sort_priority();
            let type_order_b = b.node_type.get_sort_priority();
            
            match type_order_a.cmp(&type_order_b) {
                std::cmp::Ordering::Equal => {
                    // If same type, sort by name (case-insensitive)
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
                other => other,
            }
        });        Ok(tree_nodes)
    }

    // Improved expand method for real OPC UA data
    pub async fn expand_real_node(&mut self, index: usize) -> Result<()> {
        if !self.can_expand(index) {
            return Ok(());
        }

        let node_info = {
            let node = &self.tree_nodes[index];
            (
                node.opcua_node_id.clone(),
                node.level,
                self.get_node_path(node),
            )
        };

        let opcua_node_id = node_info.0.ok_or_else(|| {
            anyhow::anyhow!("No OPC UA node ID for expansion")
        })?;

        // Update expansion state
        self.update_expansion_state(index, true);

        // Load child nodes from OPC UA server
        match self
            .get_real_children(&opcua_node_id, node_info.1 + 1, &node_info.2)
            .await
        {
            Ok(mut child_nodes) => {
                // Restore expansion state for children
                self.restore_child_expansion_states(&mut child_nodes);                // Insert children after the current node
                self.tree_nodes.splice(index + 1..index + 1, child_nodes);

                // TODO: Implement recursive expansion restoration in a separate non-recursive method
            }
            Err(e) => {
                log::error!("Failed to load children for node: {}", e);
                // Revert expansion state on error
                self.update_expansion_state(index, false);
            }
        }

        Ok(())
    }

    pub async fn update_real_attributes(&mut self) -> Result<()> {
        if self.selected_node_index >= self.tree_nodes.len() {
            self.selected_attributes.clear();
            return Ok(());
        }

        let opcua_node_id = {
            let node = &self.tree_nodes[self.selected_node_index];
            node.opcua_node_id.clone()
        };

        if let Some(opcua_node_id) = opcua_node_id {
            let client_guard = self.client.read().await;
            if !client_guard.is_connected() {
                self.selected_attributes.clear();
                return Ok(());
            }

            match client_guard.read_node_attributes(&opcua_node_id).await {
                Ok(opcua_attributes) => {
                    self.selected_attributes = opcua_attributes
                        .into_iter()
                        .map(|attr| NodeAttribute {
                            name: attr.name,
                            value: attr.value,
                        })
                        .collect();
                }
                Err(e) => {
                    log::error!("Failed to read node attributes: {}", e);
                    self.selected_attributes.clear();
                }
            }
        } else {
            self.selected_attributes.clear();
        }

        Ok(())
    }

    // Async wrapper that chooses between real and demo attribute updates
    pub async fn update_selected_attributes_async(&mut self) -> Result<()> {
        if self.selected_node_index >= self.tree_nodes.len() {
            self.selected_attributes.clear();
            return Ok(());
        }

        let has_real_node_id = self.tree_nodes[self.selected_node_index]
            .opcua_node_id
            .is_some();

        if has_real_node_id {
            // Use real OPC UA data
            self.update_real_attributes().await?;
        } else {
            // Use demo data (existing sync method)
            self.update_selected_attributes();
        }

        Ok(())
    }
}
