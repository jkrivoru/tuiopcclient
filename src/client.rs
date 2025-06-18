use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::screens::connect::SecurityPolicy;

#[derive(Debug, Clone)]
pub struct BrowseItem {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub is_folder: bool,
    pub value: Option<String>,
    pub data_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

#[derive(Debug, Clone)]
pub struct SubscriptionItem {
    pub node_id: String,
    pub display_name: String,
    pub value: Option<String>,
    pub timestamp: Option<String>,
    pub quality: Option<String>,
}

pub struct OpcUaClientManager {
    connection_status: ConnectionStatus,
    subscription_items: Vec<SubscriptionItem>,
    server_url: String,
    // Mock data for demo purposes
    mock_nodes: HashMap<String, Vec<BrowseItem>>,
}

impl OpcUaClientManager {
    pub fn new() -> Self {
        let mut mock_nodes = HashMap::new();
        
        // Create some mock OPC UA nodes for demonstration
        mock_nodes.insert("ns=0;i=85".to_string(), vec![
            BrowseItem {
                node_id: "ns=0;i=2253".to_string(),
                display_name: "Server".to_string(),
                node_class: "Object".to_string(),
                is_folder: true,
                value: None,
                data_type: None,
            },
            BrowseItem {
                node_id: "ns=2;s=Demo".to_string(),
                display_name: "Demo".to_string(),
                node_class: "Object".to_string(),
                is_folder: true,
                value: None,
                data_type: None,
            },
            BrowseItem {
                node_id: "ns=2;s=Simulation".to_string(),
                display_name: "Simulation".to_string(),
                node_class: "Object".to_string(),
                is_folder: true,
                value: None,
                data_type: None,
            },
        ]);

        mock_nodes.insert("ns=0;i=2253".to_string(), vec![
            BrowseItem {
                node_id: "ns=0;i=2256".to_string(),
                display_name: "ServerStatus".to_string(),
                node_class: "Variable".to_string(),
                is_folder: false,
                value: Some("Running".to_string()),
                data_type: Some("ServerStatusDataType".to_string()),
            },
            BrowseItem {
                node_id: "ns=0;i=2254".to_string(),
                display_name: "ServerArray".to_string(),
                node_class: "Variable".to_string(),
                is_folder: false,
                value: Some("['urn:localhost:OPCUA:SimulationServer']".to_string()),
                data_type: Some("String[]".to_string()),
            },
        ]);

        mock_nodes.insert("ns=2;s=Demo".to_string(), vec![
            BrowseItem {
                node_id: "ns=2;s=Demo.Dynamic.Scalar.Boolean".to_string(),
                display_name: "Boolean".to_string(),
                node_class: "Variable".to_string(),
                is_folder: false,
                value: Some("true".to_string()),
                data_type: Some("Boolean".to_string()),
            },
            BrowseItem {
                node_id: "ns=2;s=Demo.Dynamic.Scalar.Int32".to_string(),
                display_name: "Int32".to_string(),
                node_class: "Variable".to_string(),
                is_folder: false,
                value: Some("42".to_string()),
                data_type: Some("Int32".to_string()),
            },
            BrowseItem {
                node_id: "ns=2;s=Demo.Dynamic.Scalar.Double".to_string(),
                display_name: "Double".to_string(),
                node_class: "Variable".to_string(),
                is_folder: false,
                value: Some("3.14159".to_string()),
                data_type: Some("Double".to_string()),
            },
            BrowseItem {
                node_id: "ns=2;s=Demo.Dynamic.Scalar.String".to_string(),
                display_name: "String".to_string(),
                node_class: "Variable".to_string(),
                is_folder: false,
                value: Some("Hello OPC UA!".to_string()),
                data_type: Some("String".to_string()),
            },
        ]);

        Self {
            connection_status: ConnectionStatus::Disconnected,
            subscription_items: Vec::new(),
            server_url: String::new(),
            mock_nodes,
        }
    }

    pub async fn connect(&mut self, endpoint_url: &str, _security_policy: &SecurityPolicy) -> Result<()> {
        self.connection_status = ConnectionStatus::Connecting;
        self.server_url = endpoint_url.to_string();

        // Simulate connection delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // For demo purposes, always succeed
        self.connection_status = ConnectionStatus::Connected;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.connection_status = ConnectionStatus::Disconnected;
        self.subscription_items.clear();
        self.server_url.clear();
        Ok(())
    }

    pub fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.clone()
    }

    pub async fn browse_node(&mut self, node_id: &str) -> Result<Vec<BrowseItem>> {
        if self.connection_status != ConnectionStatus::Connected {
            return Err(anyhow!("Not connected to server"));
        }

        // Return mock data based on node_id
        if let Some(children) = self.mock_nodes.get(node_id) {
            Ok(children.clone())
        } else {
            // Return empty list for unknown nodes
            Ok(Vec::new())
        }
    }

    pub async fn add_to_subscription(&mut self, node_id: &str, display_name: &str) -> Result<()> {
        if self.connection_status != ConnectionStatus::Connected {
            return Err(anyhow!("Not connected to server"));
        }

        // Check if already subscribed
        if self.subscription_items.iter().any(|item| item.node_id == node_id) {
            return Err(anyhow!("Node already subscribed"));
        }

        let subscription_item = SubscriptionItem {
            node_id: node_id.to_string(),
            display_name: display_name.to_string(),
            value: Some("Live Value".to_string()),
            timestamp: Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()),
            quality: Some("Good".to_string()),
        };

        self.subscription_items.push(subscription_item);
        Ok(())
    }

    pub async fn remove_from_subscription(&mut self, node_id: &str) -> Result<()> {
        let initial_len = self.subscription_items.len();
        self.subscription_items.retain(|item| item.node_id != node_id);
        
        if self.subscription_items.len() == initial_len {
            Err(anyhow!("Item not found in subscription"))
        } else {
            Ok(())
        }
    }

    pub async fn get_subscription_items(&self) -> Result<Vec<SubscriptionItem>> {
        Ok(self.subscription_items.clone())
    }

    pub async fn write_node_value(&mut self, node_id: &str, value: &str) -> Result<()> {
        if self.connection_status != ConnectionStatus::Connected {
            return Err(anyhow!("Not connected to server"));
        }        // In a real implementation, this would write to the OPC UA server
        // For demo purposes, we'll just simulate success
        Ok(())
    }
}
