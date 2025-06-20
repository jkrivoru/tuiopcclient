use crate::client::ConnectionStatus;

#[derive(Clone)]
pub struct TreeNode {
    pub name: String,
    pub node_id: String,
    pub node_type: NodeType,
    pub level: usize,
    pub has_children: bool,
    pub is_expanded: bool,
    pub parent_path: String,
}

#[derive(Clone)]
pub struct NodeAttribute {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub enum NodeType {
    Object,
    Variable,
    Method,
    View,
    ObjectType,
    VariableType,
    DataType,
    ReferenceType,
}

pub struct BrowseScreen {
    // Tree navigation state
    pub current_path: Vec<String>,
    pub tree_nodes: Vec<TreeNode>,
    pub selected_node_index: usize,
    pub expanded_nodes: std::collections::HashSet<String>,
    pub scroll_offset: usize,
    
    // Attributes panel state
    pub selected_attributes: Vec<NodeAttribute>,
    pub attribute_scroll_offset: usize,

    // Connection info
    pub server_url: String,
    pub connection_status: ConnectionStatus,
    
    // Selection state for subscription
    pub selected_items: std::collections::HashSet<String>, // Store node IDs of selected items
    
    // Mouse state for double-click detection
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_position: Option<(u16, u16)>,
}

impl BrowseScreen {    pub fn new(server_url: String) -> Self {
        let mut browse_screen = Self {
            current_path: vec!["Root".to_string()],
            tree_nodes: Vec::new(),
            selected_node_index: 0,
            expanded_nodes: std::collections::HashSet::new(),
            scroll_offset: 0,
            selected_attributes: Vec::new(),
            attribute_scroll_offset: 0,
            server_url,
            connection_status: ConnectionStatus::Connected,
            selected_items: std::collections::HashSet::new(),
            last_click_time: None,
            last_click_position: None,
        };

        // Initialize with demo OPC UA tree structure
        browse_screen.load_demo_tree();
        browse_screen.update_selected_attributes();
        browse_screen
    }

    fn load_demo_tree(&mut self) {
        // Create a hierarchical OPC UA server structure
        self.tree_nodes = vec![
            TreeNode {
                name: "Objects".to_string(),
                node_id: "i=85".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
            TreeNode {
                name: "Types".to_string(),
                node_id: "i=86".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
            TreeNode {
                name: "Views".to_string(),
                node_id: "i=87".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
            TreeNode {
                name: "Server".to_string(),
                node_id: "i=2253".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
        ];
    }

    pub fn update_selected_attributes(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let node = &self.tree_nodes[self.selected_node_index];
            self.selected_attributes = vec![
                NodeAttribute {
                    name: "DisplayName".to_string(),
                    value: node.name.clone(),
                },
                NodeAttribute {
                    name: "NodeId".to_string(),
                    value: node.node_id.clone(),
                },
                NodeAttribute {
                    name: "BrowseName".to_string(),
                    value: format!("{}:{}", 
                        if node.node_id.starts_with("ns=") { "2" } else { "0" },
                        node.name
                    ),
                },
                NodeAttribute {
                    name: "NodeClass".to_string(),
                    value: format!("{:?}", node.node_type),
                },
            ];
        }
    }

    pub fn get_selected_items(&self) -> Vec<String> {
        self.selected_items.iter().cloned().collect()
    }

    pub fn clear_selections(&mut self) {
        self.selected_items.clear();
    }
}
