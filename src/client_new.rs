use anyhow::{anyhow, Result};
use opcua::client::prelude::*;
use opcua::types::*;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

pub struct OpcUaClientManager {
    connection_status: ConnectionStatus,
    client: Option<Client>,
    session: Option<Arc<RwLock<Session>>>,
    server_url: String,
}

#[derive(Clone, Debug)]
pub struct OpcUaNode {
    pub node_id: NodeId,
    pub browse_name: String,
    pub display_name: String,
    pub node_class: NodeClass,
    pub description: String,
    pub has_children: bool,
}

#[derive(Clone, Debug)]
pub struct OpcUaAttribute {
    pub name: String,
    pub value: String,
    pub data_type: String,
    pub status: String,
}

impl OpcUaClientManager {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Disconnected,
            client: None,
            session: None,
            server_url: String::new(),
        }
    }

    pub async fn connect(&mut self, endpoint_url: &str) -> Result<()> {
        self.connection_status = ConnectionStatus::Connecting;
        self.server_url = endpoint_url.to_string();

        // Create a simple client configuration
        let client_builder = ClientBuilder::new()
            .application_name("OPC UA TUI Client")
            .application_uri("urn:opcua-tui-client")
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(3);

        let mut client = client_builder
            .client()
            .ok_or_else(|| anyhow!("Failed to create client"))?; // Create an endpoint
        let endpoint = crate::endpoint_utils::EndpointUtils::create_default_endpoint(&endpoint_url);

        // Connect to the server
        let session = client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)?;

        self.client = Some(client);
        self.session = Some(session);
        self.connection_status = ConnectionStatus::Connected;

        Ok(())
    }
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(session) = self.session.take() {
            crate::session_utils::SessionUtils::disconnect_session(session).await?;
        }

        self.client = None;
        self.connection_status = ConnectionStatus::Disconnected;
        self.server_url.clear();

        Ok(())
    }

    pub fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.clone()
    }

    pub fn get_server_url(&self) -> &str {
        &self.server_url
    }

    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    pub async fn browse_node(&self, node_id: &NodeId) -> Result<Vec<OpcUaNode>> {
        if let Some(session) = &self.session {
            // Use the session to browse the node
            let session_guard = session.read();

            let browse_description = BrowseDescription {
                node_id: node_id.clone(),
                browse_direction: BrowseDirection::Forward,
                reference_type_id: ReferenceTypeId::HierarchicalReferences.into(),
                include_subtypes: true,
                node_class_mask: 0, // Include all node classes
                result_mask: 0x3F,  // All browse result attributes
            };

            match session_guard.browse(&[browse_description]) {
                Ok(results) => {
                    let mut nodes = Vec::new();
                    if let Some(results_vec) = results {
                        if let Some(result) = results_vec.first() {
                            if let Some(references) = &result.references {
                                for reference in references {
                                    if let Some(node_id) = &reference.node_id.node_id {
                                        let display_name = reference
                                            .display_name
                                            .text
                                            .as_ref()
                                            .unwrap_or(&"<No Name>".to_string())
                                            .clone();
                                        let browse_name = reference
                                            .browse_name
                                            .name
                                            .as_ref()
                                            .unwrap_or(&"<No Name>".to_string())
                                            .clone();

                                        // Determine if the node has children by checking if it's an object
                                        let has_children = matches!(
                                            reference.node_class,
                                            Some(NodeClass::Object)
                                                | Some(NodeClass::Variable)
                                                | Some(NodeClass::ObjectType)
                                        );

                                        nodes.push(OpcUaNode {
                                            node_id: node_id.clone(),
                                            browse_name,
                                            display_name,
                                            node_class: reference
                                                .node_class
                                                .unwrap_or(NodeClass::Unspecified),
                                            description: String::new(), // We'd need to read this separately
                                            has_children,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Ok(nodes)
                }
                Err(e) => {
                    // Fall back to demo data if browsing fails
                    log::warn!("Failed to browse node {}: {}. Using demo data.", node_id, e);
                    self.get_demo_nodes(node_id)
                }
            }
        } else {
            // Not connected, return demo data
            self.get_demo_nodes(node_id)
        }
    }

    fn get_demo_nodes(&self, node_id: &NodeId) -> Result<Vec<OpcUaNode>> {
        let demo_nodes = match node_id.to_string().as_str() {
            "i=85" => vec![
                // Objects folder
                OpcUaNode {
                    node_id: NodeId::new(0, 2253),
                    browse_name: "Server".to_string(),
                    display_name: "Server".to_string(),
                    node_class: NodeClass::Object,
                    description: "Server object".to_string(),
                    has_children: true,
                },
                OpcUaNode {
                    node_id: NodeId::new(2, "Devices"),
                    browse_name: "Devices".to_string(),
                    display_name: "Devices".to_string(),
                    node_class: NodeClass::Object,
                    description: "Device objects".to_string(),
                    has_children: true,
                },
            ],
            _ => Vec::new(),
        };

        Ok(demo_nodes)
    }

    pub async fn read_node_attributes(&self, node_id: &NodeId) -> Result<Vec<OpcUaAttribute>> {
        // For now, return demo attributes - real implementation will come later
        self.get_demo_attributes(node_id)
    }

    fn get_demo_attributes(&self, node_id: &NodeId) -> Result<Vec<OpcUaAttribute>> {
        let attributes = vec![
            OpcUaAttribute {
                name: "NodeId".to_string(),
                value: node_id.to_string(),
                data_type: "NodeId".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "DisplayName".to_string(),
                value: "Sample Node".to_string(),
                data_type: "LocalizedText".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "BrowseName".to_string(),
                value: "SampleNode".to_string(),
                data_type: "QualifiedName".to_string(),
                status: "Good".to_string(),
            },
        ];

        Ok(attributes)
    }

    pub async fn get_root_node(&self) -> Result<NodeId> {
        // Return the Objects folder as the root
        Ok(ObjectId::ObjectsFolder.into())
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }
}
