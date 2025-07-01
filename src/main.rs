use anyhow::Result;
use clap::Parser;
use opcua::types::MessageSecurityMode;
// Add this import
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

mod client;
mod components;
mod config;
mod connection_manager;
mod logging;
mod node_utils;
mod screens;
mod ui;
mod ui_utils;

use client::OpcUaClientManager;
use ui::App;

#[derive(Parser, Debug)]
#[command(name = "opcua-client")]
#[command(about = "OPC UA TUI Client with command line support")]
#[command(version)]
pub struct Args {
    /// OPC UA server URL (e.g., opc.tcp://localhost:4840)
    #[arg(long)]
    server_url: Option<String>,

    /// Security policy (None, Basic128Rsa15, Basic256, Basic256Sha256, Aes128Sha256RsaOaep, Aes256Sha256RsaPss)
    #[arg(long, default_value = "None")]
    security_policy: String,

    /// Security mode (None, Sign, SignAndEncrypt)
    #[arg(long, default_value = "None")]
    security_mode: String,

    /// Path to client certificate file
    #[arg(long)]
    client_certificate: Option<String>,

    /// Path to client private key file
    #[arg(long)]
    client_private_key: Option<String>,

    /// Auto-trust server certificate
    #[arg(long)]
    auto_trust: bool,

    /// Path to trusted certificate store (required if auto_trust is false)
    #[arg(long)]
    trusted_store: Option<String>,

    /// Username for authentication
    #[arg(long)]
    user_name: Option<String>,

    /// Password for authentication
    #[arg(long)]
    password: Option<String>,

    /// Path to user certificate file for X.509 authentication
    #[arg(long)]
    user_certificate: Option<String>,

    /// Path to user private key file for X.509 authentication
    #[arg(long)]
    user_private_key: Option<String>,

    /// Use original URL instead of server-provided endpoint URLs
    #[arg(long)]
    use_original_url: bool,

    /// Log level (Error, Warn, Info, Debug, Trace)
    #[arg(long, default_value = "Info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Parse log level from command line argument
    let log_level = match args.log_level.to_lowercase().as_str() {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        "off" => log::LevelFilter::Off,
        _ => {
            eprintln!("Invalid log level: {}. Using Info level.", args.log_level);
            log::LevelFilter::Info
        }
    };

    // Initialize our custom dual logger with the specified level
    logging::init_logger(log_level);

    let client_manager = Arc::new(RwLock::new(OpcUaClientManager::new())); // Check if we should connect directly via command line parameters
    if let Some(ref server_url) = args.server_url {
        // Use log macros for CLI connection (will be buffered)
        log::info!("Starting OPC UA Client with command line connection...");
        log::info!("Server URL: {server_url}");
        log::info!("Security Policy: {}", args.security_policy);
        log::info!("Security Mode: {}", args.security_mode);

        if args.use_original_url {
            log::info!("Using original URL override");
        }
        match connect_via_command_line(&args, server_url, client_manager.clone()).await {
            Ok(()) => {
                log::info!("Connection successful! Opening browse screen...");

                // Switch to TUI logging before creating the app
                logging::switch_to_tui_logging();

                // Create app in browse mode with the actual server URL
                let mut app = App::new_with_browse_direct(client_manager, server_url.clone());

                // Initialize the browse screen with tree data
                if let Err(e) = app.initialize_browse_screen().await {
                    log::warn!("Failed to load tree data: {e}");
                }

                app.run().await?;
            }
            Err(e) => {
                log::error!("Connection failed: {e}");
                // Flush console logs before exiting on connection failure
                logging::flush_console_logs();
                std::process::exit(1);
            }
        }
    } else {
        // Normal TUI mode - switch to TUI logging immediately
        logging::switch_to_tui_logging();

        let mut app = App::new(client_manager);
        app.run().await?;
    }

    // Flush console logs before normal application exit
    logging::flush_console_logs();
    Ok(())
}

async fn connect_via_command_line(
    args: &Args,
    server_url: &str,
    client_manager: Arc<RwLock<OpcUaClientManager>>,
) -> Result<()> {
    log::info!("Parsing connection parameters...");

    // Validate security configuration
    if args.security_policy != "None" || args.security_mode != "None" {
        if args.client_certificate.is_none() || args.client_private_key.is_none() {
            return Err(anyhow::anyhow!(
                "Client certificate and private key are required for non-None security"
            ));
        }

        if !args.auto_trust && args.trusted_store.is_none() {
            return Err(anyhow::anyhow!(
                "Trusted certificate store is required when auto_trust is false"
            ));
        }
    }

    // Validate authentication configuration
    let auth_mode = if args.user_name.is_some() && args.password.is_some() {
        if args.user_certificate.is_some() || args.user_private_key.is_some() {
            return Err(anyhow::anyhow!(
                "Cannot specify both username/password and user certificate authentication"
            ));
        }
        "Username/Password"
    } else if args.user_certificate.is_some() && args.user_private_key.is_some() {
        "X.509 Certificate"
    } else if args.user_name.is_some()
        || args.password.is_some()
        || args.user_certificate.is_some()
        || args.user_private_key.is_some()
    {
        return Err(anyhow::anyhow!(
            "Incomplete authentication parameters: both username and password OR both user certificate and private key must be specified"
        ));
    } else {
        "Anonymous"
    };

    log::info!("Authentication mode: {auth_mode}");

    // Connect using a more detailed implementation
    log::info!("Creating endpoint and connecting...");
    let mut client_manager_guard = client_manager.write().await;

    match connect_with_cli_params(args, server_url, &mut client_manager_guard).await {
        Ok(()) => {
            log::info!("Successfully connected to OPC UA server");
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Failed to connect to server: {}", e)),
    }
}

async fn connect_with_cli_params(
    args: &Args,
    server_url: &str,
    client_manager: &mut OpcUaClientManager,
) -> Result<()> {
    use crate::connection_manager::{ConnectionConfig, ConnectionManager};

    client_manager.connection_status = crate::client::ConnectionStatus::Connecting;
    client_manager.server_url = server_url.to_string();

    // Convert our local enums to opcua crate enums
    let opcua_security_policy = convert_security_policy(&args.security_policy)?;
    let opcua_security_mode = convert_security_mode(&args.security_mode)?;

    // Create identity token based on authentication parameters
    let identity_token = create_identity_token(args)?;

    log::info!(
        "Building client with security policy: {} and mode: {}",
        args.security_policy,
        args.security_mode
    );

    // Create unified connection configuration
    let mut config = ConnectionConfig::cli_connection()
        .with_security(
            opcua_security_policy,
            opcua_security_mode,
            args.auto_trust,
            args.client_certificate.clone(),
            args.client_private_key.clone(),
        )
        .with_authentication(identity_token.clone())
        .with_url_override(args.use_original_url);

    // If using secure connection with certificates, use the secure config
    if opcua_security_mode != MessageSecurityMode::None
        && (args.client_certificate.is_some() || args.client_private_key.is_some())
    {
        let secure_config = ConnectionConfig::secure_connection()
            .with_security_auto_uri(
                opcua_security_policy,
                opcua_security_mode,
                args.auto_trust,
                args.client_certificate.clone(),
                args.client_private_key.clone(),
            )
            .with_authentication(identity_token)
            .with_url_override(args.use_original_url);

        config = secure_config;
        log::info!("Using secure connection configuration with certificates");
    }

    // Use unified connection manager
    let connection_result = ConnectionManager::connect_to_server(server_url, &config).await;

    match connection_result {
        Ok((client, session)) => {
            // Store the connection in the client manager
            client_manager.client = Some(client);
            client_manager.session = Some(session);
            client_manager.connection_status = crate::client::ConnectionStatus::Connected;
            Ok(())
        }
        Err(e) => {
            client_manager.connection_status =
                crate::client::ConnectionStatus::Error("Connection failed".to_string());
            Err(anyhow::anyhow!("Connection failed: {}", e))
        }
    }
}

fn convert_security_policy(policy: &str) -> Result<opcua::crypto::SecurityPolicy> {
    match policy {
        "None" => Ok(opcua::crypto::SecurityPolicy::None),
        "Basic128Rsa15" => Ok(opcua::crypto::SecurityPolicy::Basic128Rsa15),
        "Basic256" => Ok(opcua::crypto::SecurityPolicy::Basic256),
        "Basic256Sha256" => Ok(opcua::crypto::SecurityPolicy::Basic256Sha256),
        "Aes128Sha256RsaOaep" => Ok(opcua::crypto::SecurityPolicy::Aes128Sha256RsaOaep),
        "Aes256Sha256RsaPss" => Ok(opcua::crypto::SecurityPolicy::Aes256Sha256RsaPss),
        _ => Err(anyhow::anyhow!("Invalid security policy: {}", policy)),
    }
}

fn convert_security_mode(mode: &str) -> Result<opcua::types::MessageSecurityMode> {
    match mode {
        "None" => Ok(opcua::types::MessageSecurityMode::None),
        "Sign" => Ok(opcua::types::MessageSecurityMode::Sign),
        "SignAndEncrypt" => Ok(opcua::types::MessageSecurityMode::SignAndEncrypt),
        _ => Err(anyhow::anyhow!("Invalid security mode: {}", mode)),
    }
}

fn create_identity_token(args: &Args) -> Result<opcua::client::prelude::IdentityToken> {
    use opcua::client::prelude::IdentityToken;
    if let (Some(username), Some(password)) = (&args.user_name, &args.password) {
        log::info!(
            "Using username/password authentication for user: {username}"
        );
        Ok(IdentityToken::UserName(username.clone(), password.clone()))
    } else if let (Some(cert_path), Some(key_path)) =
        (&args.user_certificate, &args.user_private_key)
    {
        log::info!("Using X.509 certificate authentication");

        // Validate certificate file exists
        let cert_path = std::path::Path::new(cert_path);
        if !cert_path.exists() {
            return Err(anyhow::anyhow!(
                "Certificate file does not exist: {}",
                cert_path.display()
            ));
        }

        // Validate private key file exists
        let key_path = std::path::Path::new(key_path);
        if !key_path.exists() {
            return Err(anyhow::anyhow!(
                "Private key file does not exist: {}",
                key_path.display()
            ));
        }

        log::info!("Certificate: {}", cert_path.display());
        log::info!("Private key: {}", key_path.display());

        Ok(IdentityToken::X509(
            PathBuf::from(cert_path),
            PathBuf::from(key_path),
        ))
    } else {
        log::info!("Using anonymous authentication");
        Ok(IdentityToken::Anonymous)
    }
}
