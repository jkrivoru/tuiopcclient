use log::{info, warn, error, debug};

fn main() {
    // Initialize tui-logger
    tui_logger::init_logger(log::LevelFilter::Debug).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Debug);
    
    // Test all log levels
    info!("This is an INFO message - should be white");
    warn!("This is a WARNING message - should be yellow");
    error!("This is an ERROR message - should be red");
    debug!("This is a DEBUG message - should be dark gray");
    
    // Keep some messages for display
    std::thread::sleep(std::time::Duration::from_secs(5));
}
