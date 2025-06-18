use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

mod ui;
mod client;
mod config;
mod menu;
mod statusbar;

use ui::App;
use client::OpcUaClientManager;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let client_manager = Arc::new(Mutex::new(OpcUaClientManager::new()));
    let mut app = App::new(client_manager);
    
    app.run().await?;
    
    Ok(())
}
