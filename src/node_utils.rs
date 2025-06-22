use crate::screens::browse::types::{NodeType, TreeNode};

/// Utility functions for tree node operations
pub struct NodeUtils;

impl NodeUtils {
    /// Generate a full path for a tree node
    pub fn generate_node_path(node: &TreeNode) -> String {
        if node.parent_path.is_empty() {
            node.name.clone()
        } else {
            format!("{}/{}", node.parent_path, node.name)
        }
    }

    /// Generate a full path from parent path and node name
    pub fn generate_path(parent_path: &str, node_name: &str) -> String {
        if parent_path.is_empty() {
            node_name.to_string()
        } else {
            format!("{}/{}", parent_path, node_name)
        }
    }

    /// Extract node information as a tuple for easier handling
    pub fn extract_node_info(node: &TreeNode) -> (String, String, String, bool) {
        (
            node.node_id.clone(),
            node.name.clone(),
            node.parent_path.clone(),
            node.has_children,
        )
    }
    /// Create a TreeNode with calculated level based on parent path
    pub fn create_tree_node(
        node_id: String,
        name: String,
        parent_path: String,
        has_children: bool,
    ) -> TreeNode {
        let level = if parent_path.is_empty() {
            0
        } else {
            parent_path.matches('/').count() + 1
        };

        TreeNode {
            node_id,
            name,
            parent_path,
            level,
            has_children,
            is_expanded: false,
            opcua_node_id: None,         // Default to None for demo nodes
            node_type: NodeType::Object, // Default to Object type
        }
    }
}
