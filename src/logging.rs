use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

static TUI_MODE: AtomicBool = AtomicBool::new(false);

pub fn init_logger(log_level: log::LevelFilter) {
    // Set environment variable to enable logging from opcua crate
    let level_str = match log_level {
        log::LevelFilter::Error => "error",
        log::LevelFilter::Warn => "warn",
        log::LevelFilter::Info => "info",
        log::LevelFilter::Debug => "debug",
        log::LevelFilter::Trace => "trace",
        log::LevelFilter::Off => "off",
    };
    std::env::set_var("RUST_LOG", format!("{level_str},opcua={level_str}"));

    if TUI_MODE.load(Ordering::Relaxed) {
        // In TUI mode, use tui-logger directly
        tui_logger::init_logger(log_level).ok();
        tui_logger::set_default_level(log_level);
    } else {
        // In console mode, use env_logger with a Drain to forward logs to tui-logger
        let drain = tui_logger::Drain::new();
        env_logger::Builder::default()
            .filter_level(log_level)
            .format(move |buf, record| {
                // Always forward to tui-logger for potential TUI use later
                drain.log(record);

                // Only output to console if not in TUI mode
                if !TUI_MODE.load(Ordering::Relaxed) {
                    // Format for console output with module path
                    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
                    let level = record.level().to_string();
                    let message = record.args().to_string();
                    let target = record.target();

                    // Show target (module path) for better context
                    let formatted_message =
                        format!("[{timestamp}] {level} [{target}]: {message}");

                    // Return formatted output for console
                    writeln!(buf, "{formatted_message}")
                } else {
                    // In TUI mode, don't output to console (just return Ok)
                    Ok(())
                }
            })
            .init();
    }

    // Log a test message to confirm our logger is working
    log::debug!("Logger initialized with {level_str} level");
}

pub fn switch_to_tui_logging() {
    // Mark that we're in TUI mode
    TUI_MODE.store(true, Ordering::Relaxed);
    log::info!("Switched to TUI logging mode - this should appear in the TUI");
}

pub fn flush_console_logs() {
    // Flush is handled by the underlying logger
}
