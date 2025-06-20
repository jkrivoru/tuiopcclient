#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

pub struct OpcUaClientManager {
    connection_status: ConnectionStatus,
}

impl OpcUaClientManager {
    pub fn new() -> Self {
        Self {
            connection_status: ConnectionStatus::Disconnected,
        }
    }

    pub fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.clone()
    }
    
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }
}
