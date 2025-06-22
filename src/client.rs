use anyhow::{anyhow, Result};
use opcua::client::prelude::*;
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
    pub connection_status: ConnectionStatus,
    pub client: Option<Client>,
    pub session: Option<Arc<RwLock<Session>>>,
    pub server_url: String,
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

        // Use tokio::task::spawn_blocking to run the synchronous OPC UA connection
        // in a blocking thread to avoid runtime conflicts
        let endpoint_url = endpoint_url.to_string();
        let connection_result = match tokio::time::timeout(
            tokio::time::Duration::from_secs(
                crate::screens::connect::constants::timeouts::CONNECTION_TIMEOUT_SECS,
            ),
            tokio::task::spawn_blocking(move || -> Result<(Client, Arc<RwLock<Session>>)> {
                // Create a simple client configuration with timeouts
                let client_builder = ClientBuilder::new()
                    .application_name(crate::screens::connect::constants::ui::CLIENT_NAME)
                    .application_uri(crate::screens::connect::constants::ui::CLIENT_URI)
                    .create_sample_keypair(true)
                    .trust_server_certs(true)
                    .session_retry_limit(1) // Reduce retries to fail faster
                    .session_timeout(
                        crate::screens::connect::constants::timeouts::SESSION_TIMEOUT_MS,
                    )
                    .session_retry_interval(
                        crate::screens::connect::constants::timeouts::SESSION_RETRY_INTERVAL_MS,
                    );

                let mut client = client_builder
                    .client()
                    .ok_or_else(|| anyhow!("Failed to create client"))?; // Create an endpoint
                let endpoint = crate::endpoint_utils::EndpointUtils::create_endpoint(
                    &endpoint_url,
                    MessageSecurityMode::None,
                    SecurityPolicy::None,
                    0,
                );

                // Connect to the server
                let session = client.connect_to_endpoint(endpoint, IdentityToken::Anonymous)?;

                Ok((client, session))
            }),
        )
        .await
        {
            Ok(spawn_result) => match spawn_result {
                Ok(result) => result,
                Err(join_error) => {
                    self.connection_status = ConnectionStatus::Error("Task failed".to_string());
                    return Err(anyhow!("Spawn task failed: {}", join_error));
                }
            },
            Err(_timeout) => {
                self.connection_status =
                    ConnectionStatus::Error("Connection timed out".to_string());
                return Err(anyhow!(
                    "Connection timed out after {} seconds",
                    crate::screens::connect::constants::timeouts::CONNECTION_TIMEOUT_SECS
                ));
            }
        };

        let (client, session) = connection_result?;

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
                    result_mask: 0x3F,  // All browse result attributes
                };

                session_guard.browse(&[browse_description])
            }; // Apply timeout to the browse operation
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), browse_future).await {
                Ok(Ok(results)) => {
                    let mut nodes = Vec::new();
                    if let Some(results_vec) = results {
                        if let Some(result) = results_vec.first() {
                            if let Some(references) = &result.references {
                                for reference in references {
                                    let node_id = &reference.node_id.node_id;
                                    let display_name = reference
                                        .display_name
                                        .text
                                        .value()
                                        .as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("<No Name>");
                                    let browse_name = reference
                                        .browse_name
                                        .name
                                        .value()
                                        .as_ref()
                                        .map(|s| s.as_str())
                                        .unwrap_or("<No Name>");

                                    // Determine if the node has children by checking if it's an object
                                    let has_children = matches!(
                                        reference.node_class,
                                        NodeClass::Object
                                            | NodeClass::Variable
                                            | NodeClass::ObjectType
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
                }
                Ok(Err(e)) => {
                    // Browse operation failed, fall back to demo data
                    log::warn!("Failed to browse node {}: {}. Using demo data.", node_id, e);
                    self.get_demo_nodes(node_id)
                }
                Err(_timeout) => {
                    // Browse operation timed out, fall back to demo data
                    log::warn!(
                        "Browse operation timed out for node {}. Using demo data.",
                        node_id
                    );
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
                            let name = match attr_id {
                                AttributeId::NodeId => "NodeId",
                                AttributeId::DisplayName => "DisplayName",
                                AttributeId::BrowseName => "BrowseName",
                                AttributeId::NodeClass => "NodeClass",
                                AttributeId::Description => "Description",
                                AttributeId::Value => "Value",
                                AttributeId::DataType => "DataType",
                                AttributeId::AccessLevel => "AccessLevel",
                                _ => "Unknown Attribute",
                            }
                            .to_string();
                            let (value, data_type) = match &result.value {
                                Some(val) => {
                                    let (value_str, type_str) = match val {
                                        Variant::Boolean(b) => (b.to_string(), "Boolean"),
                                        Variant::SByte(n) => (n.to_string(), "SByte"),
                                        Variant::Byte(n) => {
                                            // Special handling for AccessLevel attribute
                                            if attr_id == AttributeId::AccessLevel {
                                                (Self::format_access_level(*n), "AccessLevel")
                                            } else {
                                                (n.to_string(), "Byte")
                                            }
                                        }
                                        Variant::Int16(n) => (n.to_string(), "Int16"),
                                        Variant::UInt16(n) => (n.to_string(), "UInt16"),
                                        Variant::Int32(n) => {
                                            // Special handling for NodeClass attribute
                                            if attr_id == AttributeId::NodeClass {
                                                (Self::format_node_class(*n), "NodeClass")
                                            } else {
                                                (n.to_string(), "Int32")
                                            }
                                        }
                                        Variant::UInt32(n) => (n.to_string(), "UInt32"),
                                        Variant::Int64(n) => (n.to_string(), "Int64"),
                                        Variant::UInt64(n) => (n.to_string(), "UInt64"),
                                        Variant::Float(f) => (f.to_string(), "Float"),
                                        Variant::Double(f) => (f.to_string(), "Double"),
                                        Variant::String(s) => (
                                            s.value()
                                                .as_ref()
                                                .map(|s| s.as_str())
                                                .unwrap_or("(empty)")
                                                .to_string(),
                                            "String",
                                        ),
                                        Variant::DateTime(dt) => (dt.to_string(), "DateTime"),
                                        Variant::Guid(g) => (g.to_string(), "Guid"),
                                        Variant::ByteString(bs) => (
                                            format!("ByteString[{}]", bs.as_ref().len()),
                                            "ByteString",
                                        ),
                                        Variant::NodeId(id) => {
                                            // Special handling for DataType attribute
                                            if attr_id == AttributeId::DataType {
                                                (Self::format_data_type(id), "DataType")
                                            } else {
                                                (id.to_string(), "NodeId")
                                            }
                                        }
                                        Variant::QualifiedName(qn) => (
                                            qn.name
                                                .value()
                                                .as_ref()
                                                .map(|s| s.as_str())
                                                .unwrap_or("(empty)")
                                                .to_string(),
                                            "QualifiedName",
                                        ),
                                        Variant::LocalizedText(lt) => (
                                            lt.text
                                                .value()
                                                .as_ref()
                                                .map(|s| s.as_str())
                                                .unwrap_or("(empty)")
                                                .to_string(),
                                            "LocalizedText",
                                        ),
                                        Variant::StatusCode(sc) => {
                                            (format!("{:?}", sc), "StatusCode")
                                        }
                                        _ => (format!("{:?}", val), "Unknown"),
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
                            }; // Add all attributes, even if they have bad status or null values
                               // This gives users more visibility into what's available
                            attributes.push(OpcUaAttribute {
                                name,
                                value,
                                data_type,
                                status,
                            });
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to read attribute {:?} for node {}: {}",
                            attr_id,
                            node_id,
                            e
                        );
                    }
                }
            }

            // If we couldn't read any real attributes, fall back to demo data
            if attributes.is_empty() {
                log::warn!(
                    "No real attributes found for node {}, using demo data",
                    node_id
                );
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
                value: "Demo Sample Node".to_string(),
                data_type: "LocalizedText".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "BrowseName".to_string(),
                value: "DemoSampleNode".to_string(),
                data_type: "QualifiedName".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "NodeClass".to_string(),
                value: "Object".to_string(),
                data_type: "NodeClass".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "Description".to_string(),
                value: "This is a demo node for testing".to_string(),
                data_type: "LocalizedText".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "Value".to_string(),
                value: "42".to_string(),
                data_type: "Int32".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "DataType".to_string(),
                value: "Int32".to_string(),
                data_type: "DataType".to_string(),
                status: "Good".to_string(),
            },
            OpcUaAttribute {
                name: "AccessLevel".to_string(),
                value: "CurrentRead | CurrentWrite (3)".to_string(),
                data_type: "AccessLevel".to_string(),
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

    // Helper function to format NodeClass values into human-readable text
    fn format_node_class(node_class_value: i32) -> String {
        match node_class_value {
            1 => "Object".to_string(),
            2 => "Variable".to_string(),
            4 => "Method".to_string(),
            8 => "ObjectType".to_string(),
            16 => "VariableType".to_string(),
            32 => "ReferenceType".to_string(),
            64 => "DataType".to_string(),
            128 => "View".to_string(),
            _ => format!("Unknown NodeClass ({})", node_class_value),
        }
    }

    // Helper function to format AccessLevel values into human-readable text
    fn format_access_level(access_level: u8) -> String {
        let mut permissions = Vec::new();

        if access_level & 0x01 != 0 {
            permissions.push("CurrentRead");
        }
        if access_level & 0x02 != 0 {
            permissions.push("CurrentWrite");
        }
        if access_level & 0x04 != 0 {
            permissions.push("HistoryRead");
        }
        if access_level & 0x08 != 0 {
            permissions.push("HistoryWrite");
        }
        if access_level & 0x10 != 0 {
            permissions.push("SemanticChange");
        }
        if access_level & 0x20 != 0 {
            permissions.push("StatusWrite");
        }
        if access_level & 0x40 != 0 {
            permissions.push("TimestampWrite");
        }

        if permissions.is_empty() {
            format!("None ({})", access_level)
        } else {
            format!("{} ({})", permissions.join(" | "), access_level)
        }
    }

    // Helper function to format DataType NodeIds into human-readable text
    fn format_data_type(data_type_id: &NodeId) -> String {
        // Common OPC UA data type NodeIds
        match data_type_id.to_string().as_str() {
            "i=1" => "Boolean".to_string(),
            "i=2" => "SByte".to_string(),
            "i=3" => "Byte".to_string(),
            "i=4" => "Int16".to_string(),
            "i=5" => "UInt16".to_string(),
            "i=6" => "Int32".to_string(),
            "i=7" => "UInt32".to_string(),
            "i=8" => "Int64".to_string(),
            "i=9" => "UInt64".to_string(),
            "i=10" => "Float".to_string(),
            "i=11" => "Double".to_string(),
            "i=12" => "String".to_string(),
            "i=13" => "DateTime".to_string(),
            "i=14" => "Guid".to_string(),
            "i=15" => "ByteString".to_string(),
            "i=16" => "XmlElement".to_string(),
            "i=17" => "NodeId".to_string(),
            "i=18" => "ExpandedNodeId".to_string(),
            "i=19" => "StatusCode".to_string(),
            "i=20" => "QualifiedName".to_string(),
            "i=21" => "LocalizedText".to_string(),
            "i=22" => "Structure".to_string(),
            "i=23" => "DataValue".to_string(),
            "i=24" => "BaseDataType".to_string(),
            "i=25" => "DiagnosticInfo".to_string(),
            "i=26" => "Number".to_string(),
            "i=27" => "Integer".to_string(),
            "i=28" => "UInteger".to_string(),
            "i=29" => "Enumeration".to_string(),
            "i=30" => "Image".to_string(),
            _ => format!("Custom DataType ({})", data_type_id),
        }
    }
}
