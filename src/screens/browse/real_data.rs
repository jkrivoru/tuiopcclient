use super::types::{TreeNode, NodeType, NodeAttribute};
use crate::client::{OpcUaClientManager, OpcUaNode};
use opcua::types::{NodeId, NodeClass};
use anyhow::Result;

impl super::BrowseScreen {
    pub async fn load_real_tree(&mut self) -> Result<()> {
        self.is_loading = true;
        
        // Clear existing nodes
        self.tree_nodes.clear();
        self.selected_node_index = 0;
        self.expanded_nodes.clear();
        
        // Get the root node (Objects folder)
        let client_guard = self.client.read().await;
        if !client_guard.is_connected() {
            self.is_loading = false;
            return Ok(());
        }
        
        let root_node_id = client_guard.get_root_node().await?;
        drop(client_guard);
        
        // Load the root level nodes
        let children = self.get_real_children(&root_node_id, 0, "").await?;
        self.tree_nodes = children;
        
        self.is_loading = false;
        Ok(())
    }
    
    pub async fn get_real_children(&self, parent_node_id: &NodeId, level: usize, parent_path: &str) -> Result<Vec<TreeNode>> {
        let client_guard = self.client.read().await;
        if !client_guard.is_connected() {
            return Ok(Vec::new());
        }
        
        let opcua_nodes = client_guard.browse_node(parent_node_id).await?;
        drop(client_guard);
        
        let mut tree_nodes = Vec::new();
        
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
        }
        
        Ok(tree_nodes)
    }
    
    pub async fn expand_real_node(&mut self, index: usize) -> Result<()> {
        if index >= self.tree_nodes.len() || !self.tree_nodes[index].has_children {
            return Ok(());
        }
        
        let node_path = {
            let node = &self.tree_nodes[index];
            if node.parent_path.is_empty() {
                node.name.clone()
            } else {
                format!("{}/{}", node.parent_path, node.name)
            }
        };
        
        if self.expanded_nodes.contains(&node_path) {
            return Ok(()); // Already expanded
        }
        
        self.expanded_nodes.insert(node_path.clone());
        
        // Get node info before modifying the vector
        let (opcua_node_id, level, parent_path) = {
            let node = &self.tree_nodes[index];
            (
                node.opcua_node_id.clone(),
                node.level,
                node_path.clone()
            )
        };
        
        self.tree_nodes[index].is_expanded = true;
        
        // Load child nodes from OPC UA server
        if let Some(opcua_node_id) = opcua_node_id {
            match self.get_real_children(&opcua_node_id, level + 1, &parent_path).await {
                Ok(mut child_nodes) => {
                    // Restore expanded state for child nodes that were previously expanded
                    for child in &mut child_nodes {
                        let child_path = if child.parent_path.is_empty() {
                            child.name.clone()
                        } else {
                            format!("{}/{}", child.parent_path, child.name)
                        };
                        
                        if self.expanded_nodes.contains(&child_path) {
                            child.is_expanded = true;
                        }
                    }
                    
                    // Insert child nodes after the parent
                    self.tree_nodes.splice(index + 1..index + 1, child_nodes);
                },
                Err(e) => {
                    log::error!("Failed to load children for node: {}", e);
                }
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
                    self.selected_attributes = opcua_attributes.into_iter().map(|attr| {
                        NodeAttribute {
                            name: attr.name,
                            value: attr.value,
                        }
                    }).collect();
                },
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
        
        let has_real_node_id = self.tree_nodes[self.selected_node_index].opcua_node_id.is_some();
        
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
