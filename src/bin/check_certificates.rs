use anyhow::Result;
use std::fs;

/// Simple test to check certificate file formats and contents
fn main() -> Result<()> {
    println!("=== Certificate File Analysis ===");
    
    let cert_path = "./pki/own/OpcPlc.der";
    let key_path = "./pki/private/OpcPlc.pem";
    
    // Check certificate file
    println!("\n--- Certificate File Analysis ---");
    if let Ok(cert_data) = fs::read(cert_path) {
        println!("Certificate file size: {} bytes", cert_data.len());
        println!("First 20 bytes: {:?}", &cert_data[..20.min(cert_data.len())]);
        
        // Check if it's DER format (should start with 0x30)
        if !cert_data.is_empty() && cert_data[0] == 0x30 {
            println!("✓ Appears to be DER format (starts with 0x30)");
        } else {
            println!("✗ Does not appear to be DER format");
        }
    } else {
        println!("✗ Cannot read certificate file");
    }
    
    // Check private key file
    println!("\n--- Private Key File Analysis ---");
    if let Ok(key_data) = fs::read(key_path) {
        println!("Private key file size: {} bytes", key_data.len());
        
        if let Ok(key_str) = String::from_utf8(key_data.clone()) {
            println!("First 100 characters:");
            println!("{}", &key_str[..100.min(key_str.len())]);
            
            if key_str.contains("-----BEGIN") {
                println!("✓ Appears to be PEM format");
                
                if key_str.contains("-----BEGIN PRIVATE KEY-----") {
                    println!("✓ Contains PKCS#8 private key");
                } else if key_str.contains("-----BEGIN RSA PRIVATE KEY-----") {
                    println!("✓ Contains RSA private key");
                } else if key_str.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----") {
                    println!("⚠ Contains ENCRYPTED private key - this might be the issue!");
                } else {
                    println!("? Unknown private key type");
                }
            } else {
                println!("✗ Does not appear to be PEM format");
            }
        } else {
            println!("✗ Cannot read as UTF-8 text");
            println!("First 20 bytes: {:?}", &key_data[..20.min(key_data.len())]);
        }
    } else {
        println!("✗ Cannot read private key file");
    }
    
    // Try to parse with OpenSSL crate directly
    println!("\n--- OpenSSL Parsing Test ---");
    test_openssl_parsing(cert_path, key_path);
    
    Ok(())
}

fn test_openssl_parsing(cert_path: &str, key_path: &str) {
    // Test certificate parsing
    match fs::read(cert_path) {
        Ok(cert_data) => {
            match openssl::x509::X509::from_der(&cert_data) {
                Ok(cert) => {
                    println!("✓ Certificate parsed successfully with OpenSSL");
                    if let Some(subject) = cert.subject_name().entries().next() {
                        if let Ok(data) = subject.data().as_utf8() {
                            println!("  Subject: {}", data);
                        }
                    }
                },
                Err(e) => println!("✗ Certificate parsing failed: {}", e),
            }
        },
        Err(e) => println!("✗ Cannot read certificate: {}", e),
    }
    
    // Test private key parsing
    match fs::read(key_path) {
        Ok(key_data) => {
            match openssl::pkey::PKey::private_key_from_pem(&key_data) {
                Ok(_key) => {
                    println!("✓ Private key parsed successfully with OpenSSL");
                },
                Err(e) => {
                    println!("✗ Private key parsing failed: {}", e);
                    
                    // Try different formats
                    if let Ok(key_str) = String::from_utf8(key_data.clone()) {
                        if key_str.contains("ENCRYPTED") {
                            println!("  Note: Key appears to be encrypted - try providing a password");
                        }
                    }
                    
                    // Try parsing as RSA key specifically
                    match openssl::rsa::Rsa::private_key_from_pem(&key_data) {
                        Ok(_rsa) => println!("✓ Private key parsed as RSA key"),
                        Err(e2) => println!("✗ RSA parsing also failed: {}", e2),
                    }
                },
            }
        },
        Err(e) => println!("✗ Cannot read private key: {}", e),
    }
}
