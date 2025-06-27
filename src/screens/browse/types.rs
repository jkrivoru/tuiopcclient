use crate::client::{ConnectionStatus, OpcUaClientManager};
use opcua::types::NodeId;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tui_input::Input;
use tui_logger::TuiWidgetState;

#[derive(Clone)]
pub struct TreeNode {
    pub name: String,
    pub node_id: String,
    pub opcua_node_id: Option<NodeId>, // Add the actual OPC UA NodeId
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
    pub is_value_good: bool, // True if this is a Value attribute with Good status
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

impl NodeType {
    /// Get sorting priority for nodes in the browse tree
    /// Lower numbers = higher priority (sorted first)
    pub fn get_sort_priority(&self) -> u8 {
        match self {
            // Functions (Methods) - highest priority
            NodeType::Method => 1,

            // Objects - second priority
            NodeType::Object => 2,

            // Variables - third priority
            NodeType::Variable => 3,

            // Views - fourth priority
            NodeType::View => 4,

            // Others - sorted by type hierarchy
            NodeType::ObjectType => 5,
            NodeType::VariableType => 6,
            NodeType::DataType => 7,
            NodeType::ReferenceType => 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SearchDialogFocus {
    Input,
    Checkbox,
    Button,
}

#[derive(Debug)]
pub enum SearchMessage {
    Progress {
        current: usize,
        total: usize,
        current_node: String,
    },
    Result {
        node_id: String,
    },
    Complete,
    Cancelled,
}

#[derive(Debug)]
pub enum SearchCommand {
    Start {
        query: String,
        include_values: bool,
    },
    Cancel,
}

pub struct BrowseScreen {
    // Tree navigation state
    pub current_path: Vec<String>,
    pub tree_nodes: Vec<TreeNode>,
    pub selected_node_index: usize,
    pub expanded_nodes: std::collections::HashSet<String>,
    pub scroll_offset: usize,
    pub current_visible_height: usize, // Store current visible height for scroll calculations

    // Attributes panel state
    pub selected_attributes: Vec<NodeAttribute>,
    pub attribute_scroll_offset: usize,

    // Connection info
    pub server_url: String,
    pub connection_status: ConnectionStatus,

    // Mouse state for double-click detection
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_position: Option<(u16, u16)>,

    // OPC UA client
    pub client: Arc<RwLock<OpcUaClientManager>>,

    // Loading state
    pub is_loading: bool,    // Search functionality
    pub search_dialog_open: bool,
    pub search_input: Input,
    pub search_include_values: bool,
    pub search_dialog_focus: SearchDialogFocus,
    pub last_search_query: String,
    pub search_results: Vec<String>, // Store node IDs instead of indices
    pub current_search_index: usize,
    
    // Progress dialog for search
    pub search_progress_open: bool,
    pub search_progress_current: usize,
    pub search_progress_total: usize,
    pub search_progress_message: String,
    pub search_cancelled: bool,
    
    // Background search channels
    pub search_command_tx: Option<mpsc::UnboundedSender<SearchCommand>>,
    pub search_message_rx: Option<mpsc::UnboundedReceiver<SearchMessage>>,
    
    // Log viewer
    pub log_viewer_open: bool,
    pub logger_widget_state: TuiWidgetState,
}

impl BrowseScreen {
    pub fn new(server_url: String, client: Arc<RwLock<OpcUaClientManager>>) -> Self {
        let browse_screen = Self {
            current_path: vec!["Root".to_string()],
            tree_nodes: Vec::new(),
            selected_node_index: 0,
            expanded_nodes: std::collections::HashSet::new(),
            scroll_offset: 0,
            current_visible_height: 20, // Default height, will be updated in render
            selected_attributes: Vec::new(),
            attribute_scroll_offset: 0,
            server_url,
            connection_status: ConnectionStatus::Connected,
            last_click_time: None,
            last_click_position: None,
            client,            is_loading: true, // Start in loading state
            search_dialog_open: false,
            search_input: Input::default(),
            search_include_values: false,
            search_dialog_focus: SearchDialogFocus::Input,            last_search_query: "".to_string(),
            search_results: Vec::new(),
            current_search_index: 0,
            search_progress_open: false,            search_progress_current: 0,
            search_progress_total: 0,
            search_progress_message: "Searching...".to_string(),
            search_cancelled: false,
            search_command_tx: None,
            search_message_rx: None,
            log_viewer_open: false,
            logger_widget_state: TuiWidgetState::new(),
        };// Real data will be loaded asynchronously via load_real_tree() from real_data.rs
        browse_screen
    }    pub fn update_selected_attributes(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let node = &self.tree_nodes[self.selected_node_index];
            self.selected_attributes = vec![
                NodeAttribute {
                    name: "DisplayName".to_string(),
                    value: node.name.clone(),
                    is_value_good: false,
                },
                NodeAttribute {
                    name: "NodeId".to_string(),
                    value: node.node_id.clone(),
                    is_value_good: false,
                },
                NodeAttribute {
                    name: "BrowseName".to_string(),
                    value: format!(
                        "{}:{}",
                        if node.node_id.starts_with("ns=") {
                            "2"
                        } else {
                            "0"
                        },
                        node.name
                    ),
                    is_value_good: false,
                },
                NodeAttribute {
                    name: "NodeClass".to_string(),
                    value: format!("{:?}", node.node_type),
                    is_value_good: false,
                },
            ];
        }
    }
}

impl TreeNode {
    /// Determines if this node should show an expand indicator based on its type
    /// following OPC UA best practices
    pub fn should_show_expand_indicator(&self) -> bool {
        match self.node_type {
            // Containers - always show expand
            NodeType::Object => true,
            NodeType::View => true,
            NodeType::ObjectType => true,
            NodeType::VariableType => true,
            NodeType::DataType => true,
            NodeType::ReferenceType => true,

            // Leaves - don't show expand
            NodeType::Method => false,
            NodeType::Variable => false, // Usually a leaf, may have properties but not interesting for browsing
        }
    }
}
