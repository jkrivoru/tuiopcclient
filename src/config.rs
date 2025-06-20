use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcUaConfig {
    pub server_url: String,
    pub security_policy: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub application_name: String,
    pub application_uri: String,
    pub session_timeout: u32,
    pub keep_alive_interval: u32,
}

impl Default for OpcUaConfig {
    fn default() -> Self {
        Self {
            server_url: "opc.tcp://localhost:4840".to_string(),
            security_policy: "None".to_string(),
            username: None,
            password: None,
            application_name: "OPC UA Rust Client".to_string(),
            application_uri: "urn:OPC-UA-Rust-Client".to_string(),
            session_timeout: 60000,
            keep_alive_interval: 1000,
        }
    }
}
