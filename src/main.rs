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
    
    // Check for test mode argument
    let args: Vec<String> = std::env::args().collect();
    let test_browse_screen = args.contains(&"--test-browse".to_string());
    
    let mut app = if test_browse_screen {
        App::new_with_browse_test(client_manager)
    } else {
        App::new(client_manager)
    };

    app.run().await?;

    Ok(())
}
