use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

mod client;
mod components;
mod config;
mod screens;
mod ui;

use client::OpcUaClientManager;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tui-logger with custom settings
    tui_logger::init_logger(log::LevelFilter::Info).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Info);

    let client_manager = Arc::new(Mutex::new(OpcUaClientManager::new()));
    let mut app = App::new(client_manager);

    app.run().await?;

    Ok(())
}
