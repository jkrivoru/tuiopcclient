use anyhow::Result;
use opcua::client::prelude::*;
use opcua::crypto::SecurityPolicy;
use opcua::types::MessageSecurityMode;
use std::path::Path;

/// Minimal test program to isolate certificate loading issues
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    println!("=== OPC UA Certificate Test ===");
    
    // Test parameters
    let server_url = "opc.tcp://localhost:50001";
    let cert_path = "./pki/own/OpcPlc.der";
    let key_path = "./pki/private/private.pem";  // Use the correct private key file
    let app_uri = "urn:OpcPlc:f6527cc2559e";
    
    println!("Server URL: {}", server_url);
    println!("Certificate: {}", cert_path);
    println!("Private Key: {}", key_path);
    println!("Application URI: {}", app_uri);
    
    // Check if files exist
    if !Path::new(cert_path).exists() {
        eprintln!("ERROR: Certificate file not found: {}", cert_path);
        return Ok(());
    }
    
    if !Path::new(key_path).exists() {
        eprintln!("ERROR: Private key file not found: {}", key_path);
        return Ok(());
    }
    
    println!("✓ Certificate files exist");
    
    // Test 1: Create client with minimal configuration
    println!("\n=== Test 1: Basic client creation ===");
    match test_basic_client_creation().await {
        Ok(()) => println!("✓ Basic client creation successful"),
        Err(e) => println!("✗ Basic client creation failed: {}", e),
    }
    
    // Test 2: Create client with PKI directory structure
    println!("\n=== Test 2: Client with PKI directory ===");
    match test_client_with_pki_dir(cert_path, key_path, app_uri).await {
        Ok(()) => println!("✓ Client with PKI directory successful"),
        Err(e) => println!("✗ Client with PKI directory failed: {}", e),
    }
    
    // Test 3: Create client with absolute paths
    println!("\n=== Test 3: Client with absolute paths ===");
    match test_client_with_absolute_paths(cert_path, key_path, app_uri).await {
        Ok(()) => println!("✓ Client with absolute paths successful"),
        Err(e) => println!("✗ Client with absolute paths failed: {}", e),
    }
    
    // Test 4: Test endpoint discovery only (no certificates)
    println!("\n=== Test 4: Endpoint discovery (no certificates) ===");
    match test_endpoint_discovery_simple(server_url).await {
        Ok(()) => println!("✓ Endpoint discovery successful"),
        Err(e) => println!("✗ Endpoint discovery failed: {}", e),
    }
    
    // Test 5: Test secure connection
    println!("\n=== Test 5: Secure connection attempt ===");
    match test_secure_connection(server_url, cert_path, key_path, app_uri).await {
        Ok(()) => println!("✓ Secure connection successful"),
        Err(e) => println!("✗ Secure connection failed: {}", e),
    }
    
    Ok(())
}

async fn test_basic_client_creation() -> Result<()> {
    let client_builder = ClientBuilder::new()
        .application_name("Test OPC UA Client")
        .application_uri("urn:test-opcua-client")
        .create_sample_keypair(true)
        .trust_server_certs(true);
    
    let _client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create basic client"))?;
    
    Ok(())
}

async fn test_client_with_pki_dir(cert_path: &str, key_path: &str, app_uri: &str) -> Result<()> {
    // Create PKI directory structure
    std::fs::create_dir_all("./pki/own")?;
    std::fs::create_dir_all("./pki/private")?;
    std::fs::create_dir_all("./pki/trusted")?;
    std::fs::create_dir_all("./pki/rejected")?;
    
    let cert_filename = Path::new(cert_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("cert.der");
        
    let key_filename = Path::new(key_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("private.pem");
    
    println!("Using certificate filename: {}", cert_filename);
    println!("Using private key filename: {}", key_filename);
    
    let client_builder = ClientBuilder::new()
        .application_name("Test OPC UA Client with PKI")
        .application_uri(app_uri)
        .pki_dir("./pki")
        .certificate_path(format!("own/{}", cert_filename))
        .private_key_path(format!("private/{}", key_filename))
        .create_sample_keypair(false)
        .trust_server_certs(true);
    
    let _client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create client with PKI directory"))?;
    
    Ok(())
}

async fn test_client_with_absolute_paths(cert_path: &str, key_path: &str, app_uri: &str) -> Result<()> {
    let cert_absolute = Path::new(cert_path).canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve certificate path: {}", e))?
        .to_string_lossy()
        .to_string();
    let key_absolute = Path::new(key_path).canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve private key path: {}", e))?
        .to_string_lossy()
        .to_string();
    
    println!("Certificate absolute path: {}", cert_absolute);
    println!("Private key absolute path: {}", key_absolute);
    
    let client_builder = ClientBuilder::new()
        .application_name("Test OPC UA Client with Absolute Paths")
        .application_uri(app_uri)
        .certificate_path(&cert_absolute)
        .private_key_path(&key_absolute)
        .create_sample_keypair(false)
        .trust_server_certs(true);
    
    let _client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create client with absolute paths"))?;
    
    Ok(())
}

async fn test_endpoint_discovery_simple(server_url: &str) -> Result<()> {
    // Use a simpler approach that avoids the runtime drop issue
    println!("Creating discovery client...");
    let client_builder = ClientBuilder::new()
        .application_name("Test OPC UA Discovery Client")
        .application_uri("urn:test-opcua-discovery-client")
        .create_sample_keypair(true)
        .trust_server_certs(true);
    
    let client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create discovery client"))?;
    
    println!("Discovering endpoints...");
    let endpoints = client.get_server_endpoints_from_url(server_url)?;
    
    println!("Found {} endpoints:", endpoints.len());
    for (i, endpoint) in endpoints.iter().enumerate() {
        println!("  {}: {} (Security: {:?} - {:?})", 
                 i + 1,
                 endpoint.endpoint_url.as_ref(),
                 endpoint.security_policy_uri.as_ref(),
                 endpoint.security_mode);
    }
    
    // Explicitly drop the client to avoid runtime issues
    drop(client);
    
    Ok(())
}

async fn test_endpoint_discovery(server_url: &str) -> Result<()> {
    let client_builder = ClientBuilder::new()
        .application_name("Test OPC UA Discovery Client")
        .application_uri("urn:test-opcua-discovery-client")
        .create_sample_keypair(true)
        .trust_server_certs(true);
    
    let client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create discovery client"))?;
    
    let endpoints = client.get_server_endpoints_from_url(server_url)?;
    
    println!("Found {} endpoints:", endpoints.len());
    for (i, endpoint) in endpoints.iter().enumerate() {
        println!("  {}: {} (Security: {:?} - {:?})", 
                 i + 1,
                 endpoint.endpoint_url.as_ref(),
                 endpoint.security_policy_uri.as_ref(),
                 endpoint.security_mode);
    }
    
    Ok(())
}

async fn test_secure_connection(server_url: &str, cert_path: &str, key_path: &str, app_uri: &str) -> Result<()> {
    // Test both approaches: PKI directory and absolute paths
    let mut success_count = 0;
    let mut total_attempts = 0;
    
    // Approach 1: PKI directory
    println!("Trying PKI directory approach...");
    total_attempts += 1;
    match try_secure_connection_pki(server_url, cert_path, key_path, app_uri).await {
        Ok(()) => {
            println!("✓ PKI approach successful!");
            success_count += 1;
        },
        Err(e) => {
            println!("✗ PKI approach failed: {}", e);
            analyze_security_error(&e);
        },
    }
    
    // Approach 2: Absolute paths
    println!("\nTrying absolute paths approach...");
    total_attempts += 1;
    match try_secure_connection_absolute(server_url, cert_path, key_path, app_uri).await {
        Ok(()) => {
            println!("✓ Absolute paths approach successful!");
            success_count += 1;
        },
        Err(e) => {
            println!("✗ Absolute paths approach failed: {}", e);
            analyze_security_error(&e);
        },
    }
    
    // Approach 3: Try without client certificates (server auth only)
    println!("\nTrying server-only authentication...");
    total_attempts += 1;
    match try_secure_connection_server_only(server_url).await {
        Ok(()) => {
            println!("✓ Server-only authentication successful!");
            success_count += 1;
        },
        Err(e) => {
            println!("✗ Server-only authentication failed: {}", e);
            analyze_security_error(&e);
        },
    }
    
    println!("\n=== Secure Connection Summary ===");
    println!("Successful connections: {}/{}", success_count, total_attempts);
    
    if success_count > 0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("All secure connection attempts failed"))
    }
}

async fn try_secure_connection_pki(server_url: &str, cert_path: &str, key_path: &str, app_uri: &str) -> Result<()> {
    let cert_filename = Path::new(cert_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("OpcPlc.der");
        
    let key_filename = Path::new(key_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("OpcPlc.pem");
    
    println!("Building PKI client with cert: {} and key: {}", cert_filename, key_filename);
    
    let client_builder = ClientBuilder::new()
        .application_name("Test Secure OPC UA Client")
        .application_uri(app_uri)
        .pki_dir("./pki")
        .certificate_path(format!("own/{}", cert_filename))
        .private_key_path(format!("private/{}", key_filename))
        .create_sample_keypair(false)
        .trust_server_certs(true);
    
    let mut client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create secure client"))?;
    
    println!("Discovering endpoints for secure connection...");
    // Find a secure endpoint
    let endpoints = client.get_server_endpoints_from_url(server_url)?;
    let secure_endpoint = endpoints.iter().find(|ep| {
        ep.security_policy_uri.as_ref() == "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256"
            && ep.security_mode == MessageSecurityMode::SignAndEncrypt
    });
    
    if let Some(endpoint) = secure_endpoint {
        println!("Found secure endpoint: {}", endpoint.endpoint_url.as_ref());
        println!("Attempting secure connection with PKI approach...");
        
        // Try to connect
        match client.connect_to_endpoint((*endpoint).clone(), IdentityToken::Anonymous) {
            Ok(_session) => {
                println!("Successfully connected with PKI approach!");
                Ok(())
            },
            Err(e) => {
                println!("PKI connection failed with error: {}", e);
                Err(anyhow::anyhow!("PKI connection failed: {}", e))
            }
        }
    } else {
        return Err(anyhow::anyhow!("No suitable secure endpoint found"));
    }
}

async fn try_secure_connection_absolute(server_url: &str, cert_path: &str, key_path: &str, app_uri: &str) -> Result<()> {
    let cert_absolute = Path::new(cert_path).canonicalize()?.to_string_lossy().to_string();
    let key_absolute = Path::new(key_path).canonicalize()?.to_string_lossy().to_string();
    
    println!("Building absolute path client with cert: {} and key: {}", cert_absolute, key_absolute);
    
    let client_builder = ClientBuilder::new()
        .application_name("Test Secure OPC UA Client (Absolute)")
        .application_uri(app_uri)
        .certificate_path(&cert_absolute)
        .private_key_path(&key_absolute)
        .create_sample_keypair(false)
        .trust_server_certs(true);
    
    let mut client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create secure client with absolute paths"))?;
    
    println!("Discovering endpoints for secure connection...");
    // Find a secure endpoint
    let endpoints = client.get_server_endpoints_from_url(server_url)?;
    let secure_endpoint = endpoints.iter().find(|ep| {
        ep.security_policy_uri.as_ref() == "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256"
            && ep.security_mode == MessageSecurityMode::SignAndEncrypt
    });
    
    if let Some(endpoint) = secure_endpoint {
        println!("Found secure endpoint: {}", endpoint.endpoint_url.as_ref());
        println!("Attempting secure connection with absolute paths approach...");
        
        // Try to connect
        match client.connect_to_endpoint((*endpoint).clone(), IdentityToken::Anonymous) {
            Ok(_session) => {
                println!("Successfully connected with absolute paths approach!");
                Ok(())
            },
            Err(e) => {
                println!("Absolute paths connection failed with error: {}", e);
                Err(anyhow::anyhow!("Absolute paths connection failed: {}", e))
            }
        }
    } else {
        return Err(anyhow::anyhow!("No suitable secure endpoint found"));
    }
}

async fn try_secure_connection_server_only(server_url: &str) -> Result<()> {
    println!("Creating client with server-only authentication...");
    
    let client_builder = ClientBuilder::new()
        .application_name("Test OPC UA Client")
        .application_uri("urn:TestOpcUaClient")
        .pki_dir("pki")
        .trust_server_certs(true)  // Trust any server certificate
        .verify_server_certs(false); // Don't verify server certificates
    
    let mut client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create client"))?;
    
    // Get security policies that don't require client certificates
    let endpoints = client.get_server_endpoints_from_url(server_url)?;
    let secure_endpoints: Vec<_> = endpoints.iter()
        .filter(|ep| ep.security_mode != MessageSecurityMode::None)
        .collect();
    
    if secure_endpoints.is_empty() {
        return Err(anyhow::anyhow!("No secure endpoints found"));
    }
    
    for endpoint in &secure_endpoints {
        println!("Trying server-only connection to: {} with {:?}", 
                endpoint.security_policy_uri.as_ref(), endpoint.security_mode);
        
        // Try to connect using the simple approach
        match client.connect_to_endpoint((*endpoint).clone(), IdentityToken::Anonymous) {
            Ok(session) => {
                println!("✓ Server-only connection successful!");
                // Note: session will be dropped automatically
                return Ok(());
            },
            Err(e) => {
                println!("Server-only connection failed: {}", e);
                continue;
            },
        }
    }
    
    Err(anyhow::anyhow!("All server-only connection attempts failed"))
}

fn analyze_security_error(error: &anyhow::Error) {
    let error_str = error.to_string().to_lowercase();
    
    println!("\n--- Security Error Analysis ---");
    
    if error_str.contains("badsecuritychecksfailed") {
        println!("🔍 BadSecurityChecksFailed suggests:");
        println!("  • Client certificate might not be trusted by server");
        println!("  • Private key doesn't match the certificate");
        println!("  • Application URI mismatch between client and certificate");
        println!("  • Server certificate might be rejected by client");
        println!("  💡 Check pki/rejected/ for server certificates that need to be moved to pki/trusted/");
    } else if error_str.contains("badcertificateinvalid") {
        println!("🔍 BadCertificateInvalid suggests:");
        println!("  • Certificate format is invalid");
        println!("  • Certificate has expired");
        println!("  • Certificate chain is incomplete");
    } else if error_str.contains("badidentitytokeninvalid") {
        println!("🔍 BadIdentityTokenInvalid suggests:");
        println!("  • User authentication failed");
        println!("  • Certificate-based authentication issue");
    } else if error_str.contains("connection") && error_str.contains("refused") {
        println!("🔍 Connection refused suggests:");
        println!("  • Server is not running");
        println!("  • Wrong endpoint URL");
        println!("  • Firewall blocking connection");
    } else {
        println!("🔍 General security error - check logs for more details");
    }
    
    println!("--- End Analysis ---\n");
}
