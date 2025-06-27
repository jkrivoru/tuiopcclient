use anyhow::Result;
use opcua::client::prelude::*;
use opcua::types::MessageSecurityMode;
use std::path::Path;

/// Simple test to directly attempt secure connection
fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    println!("=== Direct Secure Connection Test ===");
    
    // Try multiple server URLs based on the hostname mismatch we discovered
    let server_urls = vec![
        "opc.tcp://localhost:50001",           // Try localhost first
        "opc.tcp://ed36dce9f0e4:50001",        // Server cert hostname
        "opc.tcp://f6527cc2559e:50001",        // Original hostname
        "opc.tcp://127.0.0.1:50001",           // Direct IP
    ];
    let cert_path = "./pki/own/OpcPlc.der";
    let key_path = "./pki/private/OpcPlc.pem";
    let app_uri = "urn:OpcPlc:f6527cc2559e";
    
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
    
    // Try each server URL until one works
    for (i, server_url) in server_urls.iter().enumerate() {
        println!("\n=== Attempt {} - Testing: {} ===", i + 1, server_url);
        
        match try_secure_connection(server_url, cert_path, key_path, app_uri) {
            Ok(()) => {
                println!("🎉 SUCCESS: Secure connection established with {}!", server_url);
                return Ok(());
            },
            Err(e) => {
                println!("❌ FAILED with {}: {}", server_url, e);
                analyze_connection_error(&e, server_url);
            }
        }
    }
    
    println!("\n❌ All connection attempts failed");
    Err(anyhow::anyhow!("Could not establish secure connection with any server URL"))
}

fn try_secure_connection(server_url: &str, cert_path: &str, key_path: &str, app_uri: &str) -> Result<()> {
    
    // Create client using PKI approach
    let cert_filename = Path::new(cert_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("OpcPlc.der");
        
    let key_filename = Path::new(key_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("OpcPlc.pem");
    
    println!("Building secure client...");
    let client_builder = ClientBuilder::new()
        .application_name("Direct Secure Test Client")
        .application_uri(app_uri)
        .pki_dir("./pki")
        .certificate_path(format!("own/{}", cert_filename))
        .private_key_path(format!("private/{}", key_filename))
        .create_sample_keypair(false)
        .trust_server_certs(true)
        .verify_server_certs(false);  // Disable hostname verification
    
    let mut client = client_builder
        .client()
        .ok_or_else(|| anyhow::anyhow!("Failed to create secure client"))?;
    
    println!("✓ Client created successfully");
    
    // Discover endpoints
    println!("Discovering endpoints...");
    let endpoints = client.get_server_endpoints_from_url(server_url)?;
    
    println!("Found {} endpoints:", endpoints.len());
    for (i, endpoint) in endpoints.iter().enumerate() {
        println!("  {}: {} (Policy: {}, Mode: {:?})", 
                 i + 1,
                 endpoint.endpoint_url.as_ref(),
                 endpoint.security_policy_uri.as_ref(),
                 endpoint.security_mode);
    }
    
    // Find secure endpoint
    let secure_endpoint = endpoints.iter().find(|ep| {
        ep.security_policy_uri.as_ref() == "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256"
            && ep.security_mode == MessageSecurityMode::SignAndEncrypt
    });
    
    if let Some(endpoint) = secure_endpoint {
        println!("\n✓ Found secure endpoint: {}", endpoint.endpoint_url.as_ref());
        println!("Attempting secure connection...");
        
        // Try to connect
        match client.connect_to_endpoint(endpoint.clone(), IdentityToken::Anonymous) {
            Ok(_session) => {
                println!("🎉 SUCCESS: Secure connection established!");
                return Ok(());
            },
            Err(e) => {
                println!("❌ FAILED: Secure connection failed");
                println!("Error: {}", e);
                
                // Try to provide more specific error information
                if e.to_string().contains("BadSecurityChecksFailed") {
                    println!("\n🔍 BadSecurityChecksFailed suggests:");
                    println!("  - Certificate/private key mismatch");
                    println!("  - Application URI in certificate doesn't match client");
                    println!("  - Certificate format issues");
                    println!("  - Certificate not trusted by server");
                } else if e.to_string().contains("BadCertificateUriInvalid") {
                    println!("\n🔍 BadCertificateUriInvalid suggests:");
                    println!("  - Application URI in certificate: (needs to be extracted)");
                    println!("  - Client Application URI: {}", app_uri);
                    println!("  - These must match exactly");
                }
                
                return Err(anyhow::anyhow!("Secure connection failed: {}", e));
            }
        }
    } else {
        println!("❌ No suitable secure endpoint found");
        println!("Available endpoints:");
        for endpoint in &endpoints {
            println!("  - Policy: {}, Mode: {:?}", 
                     endpoint.security_policy_uri.as_ref(),
                     endpoint.security_mode);
        }
        return Err(anyhow::anyhow!("No secure endpoint available"));
    }
}

fn analyze_connection_error(error: &anyhow::Error, server_url: &str) {
    let error_str = error.to_string().to_lowercase();
    
    println!("\n🔍 Analysis for {}:", server_url);
    
    if error_str.contains("badcertificatehostnameinvalid") {
        println!("  • Hostname mismatch detected");
        println!("  • Server certificate hostname doesn't match the URL hostname");
        println!("  • Try connecting to the hostname that matches the certificate");
        println!("  • Check server certificate Subject Alternative Names (SAN)");
    } else if error_str.contains("badsecuritychecksfailed") {
        println!("  • Security validation failed");
        println!("  • Could be certificate trust, key mismatch, or application URI issues");
    } else if error_str.contains("connection") && error_str.contains("refused") {
        println!("  • Server not reachable on this URL");
        println!("  • Check if server is running on this address/port");
    } else if error_str.contains("badcertificateinvalid") {
        println!("  • Certificate validation failed");
        println!("  • Check certificate format, expiration, or trust status");
    } else {
        println!("  • Unexpected error: {}", error);
    }
}
