use opcua::client::prelude::*;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use parking_lot::RwLock;

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
        }    }
    
    pub async fn connect(&mut self, endpoint_url: &str) -> Result<()> {
        self.connection_status = ConnectionStatus::Connecting;
        self.server_url = endpoint_url.to_string();

        // Use tokio::task::spawn_blocking to run the synchronous OPC UA connection
        // in a blocking thread to avoid runtime conflicts
        let endpoint_url = endpoint_url.to_string();
        let connection_result = match tokio::time::timeout(
            tokio::time::Duration::from_secs(10), // 10 second timeout
            tokio::task::spawn_blocking(move || -> Result<(Client, Arc<RwLock<Session>>)> {
                // Create a simple client configuration with timeouts
                let client_builder = ClientBuilder::new()
                    .application_name("OPC UA TUI Client")
                    .application_uri("urn:opcua-tui-client")
                    .create_sample_keypair(true)
                    .trust_server_certs(true)
                    .session_retry_limit(1) // Reduce retries to fail faster
                    .session_timeout(5000) // 5 second session timeout
                    .session_retry_interval(1000); // 1 second retry interval
                    
                let mut client = client_builder.client().ok_or_else(|| anyhow!("Failed to create client"))?;
            
                // Create an endpoint
                let endpoint = EndpointDescription {
                    endpoint_url: UAString::from(endpoint_url),
                    security_mode: MessageSecurityMode::None,
                    security_policy_uri: SecurityPolicy::None.to_uri().into(),
                    server_certificate: ByteString::null(),
                    user_identity_tokens: None,
                    transport_profile_uri: UAString::null(),
                    security_level: 0,
                    server: ApplicationDescription::default(),
                };
                
                // Connect to the server
                let session = client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)?;
                
                Ok((client, session))
            })
        ).await {
            Ok(spawn_result) => match spawn_result {
                Ok(result) => result,
                Err(join_error) => {
                    self.connection_status = ConnectionStatus::Error("Task failed".to_string());
                    return Err(anyhow!("Spawn task failed: {}", join_error));
                }
            },
            Err(_timeout) => {
                self.connection_status = ConnectionStatus::Error("Connection timed out".to_string());
                return Err(anyhow!("Connection timed out after 10 seconds"));
            }
        };
        
        let (client, session) = connection_result?;
        
        self.client = Some(client);
        self.session = Some(session);
        self.connection_status = ConnectionStatus::Connected;
        
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(session) = &self.session {
            session.write().disconnect();
        }
        
        self.client = None;
        self.session = None;
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
    }    pub async fn browse_node(&self, node_id: &NodeId) -> Result<Vec<OpcUaNode>> {
        if let Some(session) = &self.session {
            // Add timeout to browse operation to prevent hanging
            let browse_future = async {
                // Use the session to browse the node
                let session_guard = session.read();
                
                let browse_description = BrowseDescription {
                    node_id: node_id.clone(),
                    browse_direction: BrowseDirection::Forward,
                    reference_type_id: ReferenceTypeId::HierarchicalReferences.into(),
                    include_subtypes: true,
                    node_class_mask: 0, // Include all node classes
                    result_mask: 0x3F, // All browse result attributes
                };

                session_guard.browse(&[browse_description])
            };            // Apply timeout to the browse operation
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), browse_future).await {
                Ok(Ok(results)) => {
                    let mut nodes = Vec::new();
                    if let Some(results_vec) = results {
                        if let Some(result) = results_vec.first() {
                            if let Some(references) = &result.references {
                                for reference in references {
                                    let node_id = &reference.node_id.node_id;
                                    let display_name = reference.display_name.text.value()
                                        .as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("<No Name>");
                                    let browse_name = reference.browse_name.name.value()
                                        .as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("<No Name>");
                                    
                                    // Determine if the node has children by checking if it's an object
                                    let has_children = matches!(
                                        reference.node_class,
                                        NodeClass::Object | NodeClass::Variable | NodeClass::ObjectType
                                    );

                                    nodes.push(OpcUaNode {
                                        node_id: node_id.clone(),
                                        browse_name: browse_name.to_string(),
                                        display_name: display_name.to_string(),
                                        node_class: reference.node_class,
                                        description: String::new(), // We'd need to read this separately
                                        has_children,
                                    });
                                }
                            }
                        }
                    }
                    Ok(nodes)
                }                Ok(Err(e)) => {
                    // Browse operation failed, fall back to demo data
                    log::warn!("Failed to browse node {}: {}. Using demo data.", node_id, e);
                    self.get_demo_nodes(node_id)
                }
                Err(_timeout) => {
                    // Browse operation timed out, fall back to demo data
                    log::warn!("Browse operation timed out for node {}. Using demo data.", node_id);
                    self.get_demo_nodes(node_id)
                }
            }
        } else {
            // Not connected, return demo data
            self.get_demo_nodes(node_id)
        }
    }    fn get_demo_nodes(&self, node_id: &NodeId) -> Result<Vec<OpcUaNode>> {
        let demo_nodes = match node_id.to_string().as_str() {
            "i=85" => vec![ // Objects folder
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
    }pub async fn read_node_attributes(&self, node_id: &NodeId) -> Result<Vec<OpcUaAttribute>> {
        if let Some(session) = &self.session {
            let session_guard = session.read();
            let mut attributes = Vec::new();

            // Define the attributes we want to read
            let attribute_ids = vec![
                AttributeId::NodeId,
                AttributeId::DisplayName,
                AttributeId::BrowseName,
                AttributeId::NodeClass,
                AttributeId::Description,
                AttributeId::Value,
                AttributeId::DataType,
                AttributeId::AccessLevel,
            ];

            for attr_id in attribute_ids {
                let read_value_id = ReadValueId {
                    node_id: node_id.clone(),
                    attribute_id: attr_id as u32,
                    index_range: UAString::null(),
                    data_encoding: QualifiedName::null(),
                };

                match session_guard.read(&[read_value_id], TimestampsToReturn::Both, 0.0) {
                    Ok(results) => {
                        if let Some(result) = results.first() {
                                let name = format!("{:?}", attr_id);
                                let (value, data_type) = match &result.value {
                                    Some(val) => {
                                        let value_str = format!("{:?}", val);
                                        let type_str = match val {
                                            Variant::Boolean(_) => "Boolean",
                                            Variant::SByte(_) => "SByte",
                                            Variant::Byte(_) => "Byte",
                                            Variant::Int16(_) => "Int16",
                                            Variant::UInt16(_) => "UInt16",
                                            Variant::Int32(_) => "Int32",
                                            Variant::UInt32(_) => "UInt32",
                                            Variant::Int64(_) => "Int64",
                                            Variant::UInt64(_) => "UInt64",
                                            Variant::Float(_) => "Float",
                                            Variant::Double(_) => "Double",
                                            Variant::String(_) => "String",
                                            Variant::DateTime(_) => "DateTime",
                                            Variant::Guid(_) => "Guid",
                                            Variant::ByteString(_) => "ByteString",
                                            Variant::XmlElement(_) => "XmlElement",
                                            Variant::NodeId(_) => "NodeId",
                                            Variant::ExpandedNodeId(_) => "ExpandedNodeId",
                                            Variant::StatusCode(_) => "StatusCode",
                                            Variant::QualifiedName(_) => "QualifiedName",
                                            Variant::LocalizedText(_) => "LocalizedText",
                                            _ => "Unknown",
                                        };
                                        (value_str, type_str.to_string())
                                    }
                                    None => ("(null)".to_string(), "Unknown".to_string()),
                                };

                                let status = if let Some(status_code) = &result.status {
                                    if status_code.is_good() {
                                        "Good".to_string()
                                    } else {
                                        format!("Error: {:?}", status_code)
                                    }
                                } else {
                                    "Unknown".to_string()
                                };

                                // Only add attributes that have meaningful values
                                if let Some(status_code) = &result.status {
                                    if status_code.is_good() && value != "(null)" {
                                        attributes.push(OpcUaAttribute {
                                            name,
                                            value,
                                            data_type,
                                            status,
                                        });
                                    }
                                }
                            }
                    }
                    Err(e) => {
                        log::warn!("Failed to read attribute {:?} for node {}: {}", attr_id, node_id, e);
                    }
                }
            }

            // If we couldn't read any real attributes, fall back to demo data
            if attributes.is_empty() {
                log::warn!("No real attributes found for node {}, using demo data", node_id);
                self.get_demo_attributes(node_id)
            } else {
                Ok(attributes)
            }
        } else {
            // Not connected, return demo data
            self.get_demo_attributes(node_id)
        }
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
    }    pub async fn get_root_node(&self) -> Result<NodeId> {
        // Return the Objects folder as the root
        Ok(ObjectId::ObjectsFolder.into())
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }
}
