use anyhow::Result;
use opcua::client::prelude::Session;
use parking_lot::RwLock;
use std::sync::Arc;

/// Utility functions for OPC UA session management
pub struct SessionUtils;

impl SessionUtils {
    /// Safely disconnect an OPC UA session using spawn_blocking
    pub async fn disconnect_session(session: Arc<RwLock<Session>>) -> Result<()> {
        let disconnect_result = tokio::task::spawn_blocking(move || {
            session.write().disconnect();
        })
        .await;

        if let Err(e) = disconnect_result {
            log::warn!("Error during session disconnect: {}", e);
        }

        Ok(())
    }
}
