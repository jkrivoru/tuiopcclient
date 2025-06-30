use crate::screens::browse::types::TreeNode;

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
}
