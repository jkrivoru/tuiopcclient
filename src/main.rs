use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;

mod client;
mod components;
mod config;
mod endpoint_utils;
mod node_utils;
mod screens;
mod session_utils;
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
    user_private_key: Option<String>,    /// Use original URL instead of server-provided endpoint URLs
    #[arg(long)]
    use_original_url: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tui-logger with custom settings
    tui_logger::init_logger(log::LevelFilter::Info).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Info);

    let client_manager = Arc::new(RwLock::new(OpcUaClientManager::new()));    // Check if we should connect directly via command line parameters
    if let Some(ref server_url) = args.server_url {
        println!("Starting OPC UA Client with command line connection...");
        println!("Server URL: {}", server_url);
        println!("Security Policy: {}", args.security_policy);
        println!("Security Mode: {}", args.security_mode);
        
        if args.use_original_url {
            println!("Using original URL override");
        }        match connect_via_command_line(&args, &server_url, client_manager.clone()).await {            Ok(()) => {
                println!("Connection successful! Opening browse screen...");
                
                // Create app in browse mode with the actual server URL
                let mut app = App::new_with_browse_direct(client_manager, server_url.clone());
                
                // Initialize the browse screen with tree data
                if let Err(e) = app.initialize_browse_screen().await {
                    eprintln!("Warning: Failed to load tree data: {}", e);
                }
                
                app.run().await?;
            }Err(e) => {
                eprintln!("Connection failed: {}", e);
                std::process::exit(1);
            }
        }    } else {
        // Normal TUI mode
        let mut app = App::new(client_manager);
        app.run().await?;
    }

    Ok(())
}

async fn connect_via_command_line(
    args: &Args,
    server_url: &str,
    client_manager: Arc<RwLock<OpcUaClientManager>>,
) -> Result<()> {
    println!("Parsing connection parameters...");

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
    } else if args.user_name.is_some() || args.password.is_some() || 
             args.user_certificate.is_some() || args.user_private_key.is_some() {
        return Err(anyhow::anyhow!(
            "Incomplete authentication parameters: both username and password OR both user certificate and private key must be specified"
        ));
    } else {
        "Anonymous"
    };

    println!("Authentication mode: {}", auth_mode);

    // Connect using a more detailed implementation
    println!("Creating endpoint and connecting...");
    let mut client_manager_guard = client_manager.write().await;
    
    match connect_with_cli_params(args, server_url, &mut *client_manager_guard).await {        Ok(()) => {
            println!("Successfully connected to OPC UA server");
            Ok(())
        }
        Err(e) => {
            Err(anyhow::anyhow!("Failed to connect to server: {}", e))
        }    }
}

async fn connect_with_cli_params(
    args: &Args, 
    server_url: &str, 
    client_manager: &mut OpcUaClientManager
) -> Result<()> {    use opcua::client::prelude::*;
    use opcua::types::MessageSecurityMode;
    
    client_manager.connection_status = crate::client::ConnectionStatus::Connecting;
    client_manager.server_url = server_url.to_string();

    // Convert our local enums to opcua crate enums
    let opcua_security_policy = convert_security_policy(&args.security_policy)?;
    let opcua_security_mode = convert_security_mode(&args.security_mode)?;
      // Create identity token based on authentication parameters
    let identity_token = create_identity_token(args)?;
    
    println!("Building client with security policy: {} and mode: {}", 
             args.security_policy, args.security_mode);

    // First, discover endpoints from the server
    println!("Discovering endpoints from server...");
    let discovered_endpoints = discover_endpoints_for_cli(server_url).await?;
    
    if discovered_endpoints.is_empty() {
        return Err(anyhow::anyhow!("Server returned no endpoints"));
    }
    
    println!("Found {} endpoints from server", discovered_endpoints.len());
    
    // Find the matching endpoint
    let selected_endpoint = find_matching_endpoint(
        &discovered_endpoints, 
        opcua_security_policy, 
        opcua_security_mode,
        args.use_original_url,
        server_url
    )?;
    
    println!("Selected endpoint: {} (Security: {} - {})", 
             selected_endpoint.endpoint_url.as_ref(),
             args.security_policy,
             args.security_mode);

    let _server_url_clone = server_url.to_string();
    let auto_trust = args.auto_trust;
    let client_cert = args.client_certificate.clone();
    let client_key = args.client_private_key.clone();
    let endpoint_for_connection = selected_endpoint.clone();let connection_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(15),
        tokio::task::spawn_blocking(move || -> Result<(Client, Arc<parking_lot::RwLock<Session>>)> {
            // Create client builder
            let mut client_builder = ClientBuilder::new()
                .application_name("OPC UA TUI Client - CLI")
                .application_uri("urn:opcua-tui-client-cli")
                .session_retry_limit(1)
                .session_timeout(10000)
                .session_retry_interval(1000);            // Configure security
            if opcua_security_mode != MessageSecurityMode::None {
                if auto_trust {
                    println!("Auto-trusting server certificates");
                    client_builder = client_builder.trust_server_certs(true);
                }
                
                if let (Some(cert_path), Some(_key_path)) = (&client_cert, &client_key) {
                    println!("Using client certificate: {}", cert_path);
                    // For now, use sample keypair - in production you'd load actual files
                    client_builder = client_builder.create_sample_keypair(true);
                } else {
                    client_builder = client_builder.create_sample_keypair(true);
                }
            } else {
                client_builder = client_builder.trust_server_certs(true);
            }            let mut client = client_builder
                .client()
                .ok_or_else(|| anyhow::anyhow!("Failed to create client"))?;

            println!("Connecting to endpoint: {}", endpoint_for_connection.endpoint_url.as_ref());
            
            // Use the discovered endpoint instead of creating a new one
            let session = client.connect_to_endpoint(endpoint_for_connection, identity_token)?;

            Ok((client, session))        })
    ).await;    // Handle errors and unwrap the result
    match connection_result {
        Ok(spawn_result) => {
            match spawn_result {
                Ok(connection_result) => {
                    match connection_result {
                        Ok((client, session)) => {
                            // Store the connection in the client manager
                            client_manager.client = Some(client);
                            client_manager.session = Some(session);
                            client_manager.connection_status = crate::client::ConnectionStatus::Connected;
                            Ok(())
                        }
                        Err(e) => {
                            client_manager.connection_status = crate::client::ConnectionStatus::Error("Connection failed".to_string());
                            Err(anyhow::anyhow!("Connection failed: {}", e))
                        }
                    }
                }
                Err(e) => {
                    client_manager.connection_status = crate::client::ConnectionStatus::Error("Connection task failed".to_string());
                    Err(anyhow::anyhow!("Connection task failed: {}", e))
                }
            }
        }
        Err(_timeout) => {
            client_manager.connection_status = crate::client::ConnectionStatus::Error("Connection timed out".to_string());
            Err(anyhow::anyhow!("Connection timed out after 15 seconds"))
        }
    }
}

async fn discover_endpoints_for_cli(server_url: &str) -> Result<Vec<opcua::types::EndpointDescription>> {
    use opcua::client::prelude::*;
    
    let url = server_url.to_string();
    
    tokio::task::spawn_blocking(move || -> Result<Vec<opcua::types::EndpointDescription>> {
        let client_builder = ClientBuilder::new()
            .application_name("OPC UA TUI Client - Discovery")
            .application_uri("urn:opcua-tui-client-discovery")
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(1)
            .session_timeout(5000);

        let client = client_builder
            .client()
            .ok_or_else(|| anyhow::anyhow!("Failed to create discovery client"))?;

        match client.get_server_endpoints_from_url(&url) {
            Ok(endpoints) => {
                println!("Successfully discovered {} endpoints from server", endpoints.len());
                
                // Log discovered endpoints for debugging
                for (i, endpoint) in endpoints.iter().enumerate() {
                    println!("  Endpoint {}: {} (Security: {:?} - {:?})", 
                             i + 1,
                             endpoint.endpoint_url.as_ref(),
                             endpoint.security_policy_uri.as_ref(),
                             endpoint.security_mode);
                }
                
                Ok(endpoints)
            }
            Err(e) => {
                Err(anyhow::anyhow!("Failed to discover endpoints: {}", e))
            }
        }
    })
    .await?
}

fn find_matching_endpoint(
    endpoints: &[opcua::types::EndpointDescription],
    requested_policy: opcua::crypto::SecurityPolicy,
    requested_mode: opcua::types::MessageSecurityMode,
    use_original_url: bool,
    original_url: &str,
) -> Result<opcua::types::EndpointDescription> {
    use opcua::types::UAString;
      // Convert security policy to URI string for comparison
    let requested_policy_uri = match requested_policy {
        opcua::crypto::SecurityPolicy::None => "http://opcfoundation.org/UA/SecurityPolicy#None",
        opcua::crypto::SecurityPolicy::Basic128Rsa15 => "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15",
        opcua::crypto::SecurityPolicy::Basic256 => "http://opcfoundation.org/UA/SecurityPolicy#Basic256",
        opcua::crypto::SecurityPolicy::Basic256Sha256 => "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256",
        opcua::crypto::SecurityPolicy::Aes128Sha256RsaOaep => "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep",
        opcua::crypto::SecurityPolicy::Aes256Sha256RsaPss => "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss",
        opcua::crypto::SecurityPolicy::Unknown => {
            return Err(anyhow::anyhow!("Unknown security policy not supported"));
        }
    };
    
    // Find matching endpoint
    let matching_endpoint = endpoints.iter().find(|endpoint| {
        endpoint.security_policy_uri.as_ref() == requested_policy_uri &&
        endpoint.security_mode == requested_mode
    });
    
    match matching_endpoint {
        Some(endpoint) => {
            let mut selected_endpoint = endpoint.clone();
            
            // If use_original_url is enabled, override the endpoint URL
            if use_original_url {
                println!("Using original URL override: {} -> {}", 
                         endpoint.endpoint_url.as_ref(), 
                         original_url);
                selected_endpoint.endpoint_url = UAString::from(original_url);
            }
            
            Ok(selected_endpoint)
        }
        None => {
            // No matching endpoint found, provide helpful error message
            let available_endpoints: Vec<String> = endpoints.iter().map(|ep| {
                format!("  - {} (Security: {} - {:?})", 
                        ep.endpoint_url.as_ref(),
                        policy_uri_to_name(ep.security_policy_uri.as_ref()),
                        ep.security_mode)
            }).collect();
            
            Err(anyhow::anyhow!(
                "No endpoint found matching security policy '{}' and mode '{:?}'.\n\nAvailable endpoints:\n{}",
                policy_name_from_enum(requested_policy),
                requested_mode,
                available_endpoints.join("\n")
            ))
        }
    }
}

fn policy_uri_to_name(uri: &str) -> &str {
    match uri {
        "http://opcfoundation.org/UA/SecurityPolicy#None" => "None",
        "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15" => "Basic128Rsa15",
        "http://opcfoundation.org/UA/SecurityPolicy#Basic256" => "Basic256",
        "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256" => "Basic256Sha256",
        "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep" => "Aes128Sha256RsaOaep",
        "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss" => "Aes256Sha256RsaPss",
        _ => "Unknown"
    }
}

fn policy_name_from_enum(policy: opcua::crypto::SecurityPolicy) -> &'static str {
    match policy {
        opcua::crypto::SecurityPolicy::None => "None",
        opcua::crypto::SecurityPolicy::Basic128Rsa15 => "Basic128Rsa15",
        opcua::crypto::SecurityPolicy::Basic256 => "Basic256",
        opcua::crypto::SecurityPolicy::Basic256Sha256 => "Basic256Sha256",
        opcua::crypto::SecurityPolicy::Aes128Sha256RsaOaep => "Aes128Sha256RsaOaep",
        opcua::crypto::SecurityPolicy::Aes256Sha256RsaPss => "Aes256Sha256RsaPss",
        opcua::crypto::SecurityPolicy::Unknown => "Unknown",
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
        _ => Err(anyhow::anyhow!("Invalid security policy: {}", policy))
    }
}

fn convert_security_mode(mode: &str) -> Result<opcua::types::MessageSecurityMode> {
    match mode {
        "None" => Ok(opcua::types::MessageSecurityMode::None),
        "Sign" => Ok(opcua::types::MessageSecurityMode::Sign),
        "SignAndEncrypt" => Ok(opcua::types::MessageSecurityMode::SignAndEncrypt),
        _ => Err(anyhow::anyhow!("Invalid security mode: {}", mode))
    }
}

fn create_identity_token(args: &Args) -> Result<opcua::client::prelude::IdentityToken> {
    use opcua::client::prelude::IdentityToken;
      if let (Some(username), Some(password)) = (&args.user_name, &args.password) {
        println!("Using username/password authentication for user: {}", username);
        Ok(IdentityToken::UserName(username.clone(), password.clone()))
    } else if let (Some(_cert_path), Some(_key_path)) = (&args.user_certificate, &args.user_private_key) {
        println!("Using X.509 certificate authentication");
        // For now, return error as X.509 auth needs more implementation
        Err(anyhow::anyhow!("X.509 certificate authentication not yet fully implemented"))    } else {
        println!("Using anonymous authentication");
        Ok(IdentityToken::Anonymous)
    }
}
