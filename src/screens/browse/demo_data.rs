use super::types::{NodeType, TreeNode};

// Helper function to create demo TreeNode instances
fn create_demo_node(
    name: &str,
    node_id: &str,
    node_type: NodeType,
    level: usize,
    has_children: bool,
    parent_path: &str,
) -> TreeNode {
    TreeNode {
        name: name.to_string(),
        node_id: node_id.to_string(),
        opcua_node_id: None, // Demo data doesn't have real NodeIds
        node_type,
        level,
        has_children,
        is_expanded: false,
        parent_path: parent_path.to_string(),
    }
}

impl super::BrowseScreen {
    pub fn get_demo_children(
        &self,
        parent_id: &str,
        level: usize,
        parent_path: &str,
    ) -> Vec<TreeNode> {
        match parent_id {
            "i=85" => vec![
                // Objects
                create_demo_node(
                    "Server",
                    "i=2253",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "DeviceSet",
                    "i=5001",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "Simulation",
                    "ns=2;s=Simulation",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "DataAccess",
                    "ns=2;s=DataAccess",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
            ],
            "i=86" => vec![
                // Types
                create_demo_node(
                    "ObjectTypes",
                    "i=58",
                    NodeType::ObjectType,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "VariableTypes",
                    "i=62",
                    NodeType::VariableType,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "DataTypes",
                    "i=22",
                    NodeType::DataType,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "ReferenceTypes",
                    "i=31",
                    NodeType::ReferenceType,
                    level,
                    true,
                    parent_path,
                ),
            ],
            "i=2253" => vec![
                // Server
                create_demo_node(
                    "ServerCapabilities",
                    "i=2268",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "ServerDiagnostics",
                    "i=2274",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "VendorServerInfo",
                    "i=2295",
                    NodeType::Object,
                    level,
                    false,
                    parent_path,
                ),
                create_demo_node(
                    "ServerRedundancy",
                    "i=2296",
                    NodeType::Object,
                    level,
                    false,
                    parent_path,
                ),
            ],
            "ns=2;s=Simulation" => vec![
                // Simulation
                create_demo_node(
                    "Random",
                    "ns=2;s=Simulation.Random",
                    NodeType::Variable,
                    level,
                    false,
                    parent_path,
                ),
                create_demo_node(
                    "Sinusoid",
                    "ns=2;s=Simulation.Sinusoid",
                    NodeType::Variable,
                    level,
                    false,
                    parent_path,
                ),
                create_demo_node(
                    "Ramp",
                    "ns=2;s=Simulation.Ramp",
                    NodeType::Variable,
                    level,
                    false,
                    parent_path,
                ),
            ],
            "ns=2;s=DataAccess" => vec![
                // DataAccess
                create_demo_node(
                    "AnalogType",
                    "ns=2;s=DataAccess.AnalogType",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
                create_demo_node(
                    "TwoStateDiscreteType",
                    "ns=2;s=DataAccess.TwoStateDiscreteType",
                    NodeType::Object,
                    level,
                    true,
                    parent_path,
                ),
            ],
            _ => vec![
                // Default children for any other node
                create_demo_node(
                    "Value",
                    &format!("{}.Value", parent_id),
                    NodeType::Variable,
                    level,
                    false,
                    parent_path,
                ),
                create_demo_node(
                    "Timestamp",
                    &format!("{}.Timestamp", parent_id),
                    NodeType::Variable,
                    level,
                    false,
                    parent_path,
                ),
            ],
        }
    }
}
