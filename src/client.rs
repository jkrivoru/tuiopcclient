use anyhow::Result;
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
    pub is_value_good: bool, // True if this is a Value attribute with Good status
}

impl OpcUaClientManager {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Disconnected,
            client: None,
            session: None,
            server_url: String::new(),
        }
    }    pub async fn connect(&mut self, endpoint_url: &str) -> Result<()> {
        use crate::connection_manager::{ConnectionManager, ConnectionConfig};
        
        self.connection_status = ConnectionStatus::Connecting;
        self.server_url = endpoint_url.to_string();        // Create unified connection configuration
        let config = ConnectionConfig::ui_connection();

        match ConnectionManager::connect_to_server(endpoint_url, &config).await {
            Ok((client, session)) => {
                self.client = Some(client);
                self.session = Some(session);
                self.connection_status = ConnectionStatus::Connected;
                Ok(())
            }
            Err(e) => {
                self.connection_status = ConnectionStatus::Error(format!("Connection failed: {}", e));
                Err(e)
            }
        }
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
                }                Ok(Err(e)) => {
                    // Browse operation failed
                    log::warn!("Failed to browse node {}: {}", node_id, e);
                    Err(anyhow::anyhow!("Browse operation failed: {}", e))
                }
                Err(_timeout) => {
                    // Browse operation timed out
                    log::warn!("Browse operation timed out for node {}", node_id);
                    Err(anyhow::anyhow!("Browse operation timed out"))
                }
            }
        } else {
            // Not connected
            Err(anyhow::anyhow!("Not connected to OPC UA server"))        }
    }    pub async fn read_node_attributes(&self, node_id: &NodeId) -> Result<Vec<OpcUaAttribute>> {
        if let Some(session) = &self.session {
            let session_guard = session.read();
            let mut attributes = Vec::new();            // Define all the standard OPC UA attributes we want to read (excluding Value for special handling)
            let attribute_ids = vec![
                AttributeId::NodeId,
                AttributeId::NodeClass,
                AttributeId::BrowseName,
                AttributeId::DisplayName,
                AttributeId::Description,
                AttributeId::WriteMask,
                AttributeId::UserWriteMask,
                AttributeId::IsAbstract,
                AttributeId::Symmetric,
                AttributeId::InverseName,
                AttributeId::ContainsNoLoops,
                AttributeId::EventNotifier,
                AttributeId::DataType,
                AttributeId::ValueRank,
                AttributeId::ArrayDimensions,
                AttributeId::AccessLevel,
                AttributeId::UserAccessLevel,
                AttributeId::MinimumSamplingInterval,
                AttributeId::Historizing,
                AttributeId::Executable,
                AttributeId::UserExecutable,
                AttributeId::DataTypeDefinition,
                AttributeId::RolePermissions,
                AttributeId::UserRolePermissions,
                AttributeId::AccessRestrictions,
                AttributeId::AccessLevelEx,
            ];

            // Read standard attributes
            for attr_id in attribute_ids {
                let read_value_id = ReadValueId {
                    node_id: node_id.clone(),
                    attribute_id: attr_id as u32,
                    index_range: UAString::null(),
                    data_encoding: QualifiedName::null(),
                };

                match session_guard.read(&[read_value_id], TimestampsToReturn::Both, 0.0) {
                    Ok(results) => {
                        if let Some(result) = results.first() {                            let name = match attr_id {
                                AttributeId::NodeId => "NodeId",
                                AttributeId::NodeClass => "NodeClass",
                                AttributeId::BrowseName => "BrowseName",
                                AttributeId::DisplayName => "DisplayName",
                                AttributeId::Description => "Description",
                                AttributeId::WriteMask => "WriteMask",
                                AttributeId::UserWriteMask => "UserWriteMask",
                                AttributeId::IsAbstract => "IsAbstract",
                                AttributeId::Symmetric => "Symmetric",
                                AttributeId::InverseName => "InverseName",
                                AttributeId::ContainsNoLoops => "ContainsNoLoops",
                                AttributeId::EventNotifier => "EventNotifier",
                                AttributeId::DataType => "DataType",
                                AttributeId::ValueRank => "ValueRank",
                                AttributeId::ArrayDimensions => "ArrayDimensions",
                                AttributeId::AccessLevel => "AccessLevel",
                                AttributeId::UserAccessLevel => "UserAccessLevel",
                                AttributeId::MinimumSamplingInterval => "MinimumSamplingInterval",
                                AttributeId::Historizing => "Historizing",
                                AttributeId::Executable => "Executable",
                                AttributeId::UserExecutable => "UserExecutable",
                                AttributeId::DataTypeDefinition => "DataTypeDefinition",
                                AttributeId::RolePermissions => "RolePermissions",
                                AttributeId::UserRolePermissions => "UserRolePermissions",
                                AttributeId::AccessRestrictions => "AccessRestrictions",
                                AttributeId::AccessLevelEx => "AccessLevelEx",
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
                            };                            let status = if let Some(status_code) = &result.status {
                                if status_code.is_good() {
                                    "Good".to_string()
                                } else {
                                    format!("Error: {:?}", status_code)
                                }
                            } else {
                                "Unknown".to_string()
                            };

                            // Filter out null/empty attributes (except for Value which is handled separately)
                            let should_include = match &result.value {
                                Some(val) => match val {
                                    Variant::String(s) => {
                                        s.value().as_ref().map(|s| !s.is_empty()).unwrap_or(false)
                                    },
                                    Variant::LocalizedText(lt) => {
                                        lt.text.value().as_ref().map(|s| !s.is_empty()).unwrap_or(false)
                                    },
                                    Variant::QualifiedName(qn) => {
                                        qn.name.value().as_ref().map(|s| !s.is_empty()).unwrap_or(false)
                                    },
                                    Variant::ByteString(bs) => !bs.as_ref().is_empty(),
                                    _ => true, // Include all other non-null variants
                                },
                                None => false, // Exclude null values
                            };

                            // Only add non-null/non-empty attributes
                            if should_include {
                                attributes.push(OpcUaAttribute {
                                    name,
                                    value,
                                    data_type,
                                    status,
                                    is_value_good: false,
                                });
                            }
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
            }            // Check if this node can have a Value attribute by examining its NodeClass
            let node_class_from_attributes = attributes.iter()
                .find(|attr| attr.name == "NodeClass")
                .map(|attr| attr.value.as_str());

            let can_have_value = match node_class_from_attributes {
                Some("Variable") | Some("VariableType") => true,
                _ => false,
            };            // Only read Value attribute for Variable and VariableType nodes
            if can_have_value {
                let read_value_id = ReadValueId {
                    node_id: node_id.clone(),
                    attribute_id: AttributeId::Value as u32,
                    index_range: UAString::null(),
                    data_encoding: QualifiedName::null(),
                };

                match session_guard.read(&[read_value_id], TimestampsToReturn::Both, 0.0) {
                    Ok(results) => {
                        if let Some(data_value) = results.first() {
                            let (value, data_type) = match &data_value.value {
                                Some(val) => {
                                    let (value_str, type_str) = match val {
                                        Variant::Boolean(b) => (b.to_string(), "Boolean"),
                                        Variant::SByte(n) => (n.to_string(), "SByte"),
                                        Variant::Byte(n) => (n.to_string(), "Byte"),
                                        Variant::Int16(n) => (n.to_string(), "Int16"),
                                        Variant::UInt16(n) => (n.to_string(), "UInt16"),
                                        Variant::Int32(n) => (n.to_string(), "Int32"),
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
                                        Variant::NodeId(id) => (id.to_string(), "NodeId"),
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

                            let status = if let Some(status_code) = &data_value.status {
                                if status_code.is_good() {
                                    "Good".to_string()
                                } else {
                                    format!("Error: {:?}", status_code)
                                }
                            } else {
                                "Unknown".to_string()
                            };                            // Use DataValue.is_valid() to determine if value should be colored green
                            let is_value_good = data_value.is_valid();

                            attributes.push(OpcUaAttribute {
                                name: "Value".to_string(),
                                value,
                                data_type,
                                status,
                                is_value_good,
                            });                            // Add custom debug attributes with indentation
                            let value_status_text = if let Some(status_code) = &data_value.status {
                                format!("{:?}", status_code)
                            } else {
                                "Good".to_string()
                            };

                            attributes.push(OpcUaAttribute {
                                name: "   Status".to_string(),
                                value: value_status_text,
                                data_type: "Debug".to_string(),
                                status: "Good".to_string(),
                                is_value_good: false,
                            });

                            // Add SourceTimestamp attribute
                            let source_timestamp_text = if let Some(timestamp) = &data_value.source_timestamp {
                                timestamp.to_string()
                            } else {
                                "None".to_string()
                            };

                            attributes.push(OpcUaAttribute {
                                name: "   SourceTimestamp".to_string(),
                                value: source_timestamp_text,
                                data_type: "Debug".to_string(),
                                status: "Good".to_string(),
                                is_value_good: false,
                            });

                            // Add ServerTimestamp attribute
                            let server_timestamp_text = if let Some(timestamp) = &data_value.server_timestamp {
                                timestamp.to_string()
                            } else {
                                "None".to_string()
                            };

                            attributes.push(OpcUaAttribute {
                                name: "   ServerTimestamp".to_string(),
                                value: server_timestamp_text,
                                data_type: "Debug".to_string(),
                                status: "Good".to_string(),
                                is_value_good: false,
                            });
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to read Value attribute for node {}: {}",
                            node_id,
                            e
                        );
                        // Add a placeholder Value attribute with error status
                        attributes.push(OpcUaAttribute {
                            name: "Value".to_string(),
                            value: format!("Read Error: {}", e),
                            data_type: "Error".to_string(),
                            status: "Error".to_string(),
                            is_value_good: false,
                        });
                    }
                }
            }
            // Note: For nodes that cannot have values (Objects, Methods, etc.), 
            // we simply don't add a Value attribute at all

            // If we couldn't read any real attributes, return error
            if attributes.is_empty() {
                log::warn!("No attributes found for node {}", node_id);
                Err(anyhow::anyhow!("No attributes found for node"))
            } else {
                Ok(attributes)
            }
        } else {
            // Not connected
            Err(anyhow::anyhow!("Not connected to OPC UA server"))
        }
    }

    /// Read only the attributes needed for search (BrowseName and DisplayName)
    /// This is much more efficient than reading all node attributes
    pub async fn read_node_search_attributes(&self, node_id: &NodeId) -> Result<(String, String)> {
        if let Some(session) = &self.session {
            let session_guard = session.read();
            
            // Read only BrowseName and DisplayName attributes
            let read_values = vec![
                ReadValueId {
                    node_id: node_id.clone(),
                    attribute_id: AttributeId::BrowseName as u32,
                    index_range: UAString::null(),
                    data_encoding: QualifiedName::null(),
                },
                ReadValueId {
                    node_id: node_id.clone(),
                    attribute_id: AttributeId::DisplayName as u32,
                    index_range: UAString::null(),
                    data_encoding: QualifiedName::null(),
                },
            ];
            
            match session_guard.read(&read_values, TimestampsToReturn::Neither, 0.0) {
                Ok(results) => {
                    let browse_name = if let Some(result) = results.get(0) {
                        if let Some(Variant::QualifiedName(qname)) = &result.value {
                            qname.name
                                .value()
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or("(empty)")
                                .to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    
                    let display_name = if let Some(result) = results.get(1) {
                        if let Some(Variant::LocalizedText(ltext)) = &result.value {
                            ltext.text
                                .value()
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or("(empty)")
                                .to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    
                    Ok((browse_name, display_name))
                }
                Err(e) => {
                    log::debug!("Failed to read search attributes for node {}: {}", node_id, e);
                    // Return empty strings if we can't read the attributes
                    Ok((String::new(), String::new()))
                }
            }
        } else {
            Err(anyhow::anyhow!("Not connected to OPC UA server"))
        }
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
