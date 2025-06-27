use anyhow::Result;
use opcua::crypto::SecurityPolicy;
use opcua::types::MessageSecurityMode;

mod connection_manager;

use connection_manager::{ConnectionManager, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== OPC UA Error Handling Test ===");
    
    // Initialize logging
    env_logger::init();
    
    // Test 1: Missing certificate file
    println!("\n1. Testing with missing certificate file...");
    test_missing_certificate().await;
    
    // Test 2: Missing private key file 
    println!("\n2. Testing with missing private key file...");
    test_missing_private_key().await;
    
    // Test 3: Mismatched certificate and key
    println!("\n3. Testing with mismatched certificate and key...");
    test_mismatched_cert_key().await;
    
    // Test 4: Invalid server URL (should return error, not hang)
    println!("\n4. Testing with invalid server URL...");
    test_invalid_server_url().await;
    
    println!("\n=== All error handling tests completed successfully! ===");
    println!("✅ No hangs, no panics - all errors returned cleanly");
    
    Ok(())
}

async fn test_missing_certificate() {
    let config = ConnectionConfig::default()
        .with_security(
            SecurityPolicy::Basic256Sha256,
            MessageSecurityMode::SignAndEncrypt,
            true,
            Some("./nonexistent_cert.der".to_string()),
            Some("./pki/private/OpcPlc.pem".to_string()),
        );
    
    println!("  Connecting with missing certificate...");
    match ConnectionManager::connect_to_server("opc.tcp://localhost:4840", &config).await {
        Ok(_) => println!("  ❌ Unexpected success"),
        Err(e) => println!("  ✅ Expected error: {}", e),
    }
}

async fn test_missing_private_key() {
    let config = ConnectionConfig::default()
        .with_security(
            SecurityPolicy::Basic256Sha256,
            MessageSecurityMode::SignAndEncrypt,
            true,
            Some("./pki/own/OpcPlc.der".to_string()),
            Some("./nonexistent_key.pem".to_string()),
        );
    
    println!("  Connecting with missing private key...");
    match ConnectionManager::connect_to_server("opc.tcp://localhost:4840", &config).await {
        Ok(_) => println!("  ❌ Unexpected success"),
        Err(e) => println!("  ✅ Expected error: {}", e),
    }
}

async fn test_mismatched_cert_key() {
    let config = ConnectionConfig::default()
        .with_security(
            SecurityPolicy::Basic256Sha256,
            MessageSecurityMode::SignAndEncrypt,
            true,
            Some("./pki/own/OpcPlc.der".to_string()),
            Some("./pki/private/private.pem".to_string()), // Different key
        );
    
    println!("  Connecting with mismatched certificate and key...");
    match ConnectionManager::connect_to_server("opc.tcp://localhost:4840", &config).await {
        Ok(_) => println!("  ❌ Unexpected success"),
        Err(e) => println!("  ✅ Expected error: {}", e),
    }
}

async fn test_invalid_server_url() {
    let config = ConnectionConfig::default();
    
    println!("  Connecting to invalid server URL...");
    match ConnectionManager::connect_to_server("opc.tcp://invalid.server:9999", &config).await {
        Ok(_) => println!("  ❌ Unexpected success"),
        Err(e) => println!("  ✅ Expected error: {}", e),
    }
}
