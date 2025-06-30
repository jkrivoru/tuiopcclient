use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;

static TUI_MODE: AtomicBool = AtomicBool::new(false);

pub fn init_logger() {
    // Set environment variable to enable more verbose logging from opcua crate
    std::env::set_var("RUST_LOG", "debug,opcua=debug");
    
    if TUI_MODE.load(Ordering::Relaxed) {
        // In TUI mode, use tui-logger directly
        tui_logger::init_logger(log::LevelFilter::Trace).ok();
        tui_logger::set_default_level(log::LevelFilter::Trace);
    } else {
        // In console mode, use env_logger with a Drain to forward logs to tui-logger
        let drain = tui_logger::Drain::new();
        env_logger::Builder::default()
            .filter_level(log::LevelFilter::Trace)
            .format(move |buf, record| {
                // Always forward to tui-logger for potential TUI use later
                drain.log(record);
                
                // Only output to console if not in TUI mode
                if !TUI_MODE.load(Ordering::Relaxed) {
                    // Format for console output
                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
                    let level = record.level().to_string();
                    let message = record.args().to_string();
                    let target = record.target();
                    
                    // Include the target (module/crate name) to see where logs are coming from
                    let formatted_message = if target != record.module_path().unwrap_or("unknown") {
                        format!("[{}] {} [{}]: {}", timestamp, level, target, message)
                    } else {
                        format!("[{}] {}: {}", timestamp, level, message)
                    };
                    
                    // Return formatted output for console
                    writeln!(buf, "{}", formatted_message)
                } else {
                    // In TUI mode, don't output to console (just return Ok)
                    Ok(())
                }
            })
            .init();
    }
    
    // Log a test message to confirm our logger is working
    log::debug!("Logger initialized with Trace level");
}

pub fn switch_to_tui_logging() {
    // Mark that we're in TUI mode
    TUI_MODE.store(true, Ordering::Relaxed);
    log::info!("Switched to TUI logging mode - this should appear in the TUI");
}

pub fn flush_console_logs() {
    // Flush is handled by the underlying logger
}

pub fn is_tui_mode() -> bool {
    TUI_MODE.load(Ordering::Relaxed)
}
