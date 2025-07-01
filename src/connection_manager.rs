use anyhow::{anyhow, Result};
use opcua::client::prelude::*;
use opcua::crypto::SecurityPolicy;
use opcua::types::{EndpointDescription, MessageSecurityMode, UAString};
use parking_lot::RwLock;
use std::sync::Arc;

/// Unified connection manager for all OPC UA connection scenarios
pub struct ConnectionManager;

/// Configuration for OPC UA client connections
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub application_name: String,
    pub application_uri: String,
    pub security_policy: SecurityPolicy,
    pub security_mode: MessageSecurityMode,
    pub auto_trust: bool,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub identity_token: IdentityToken,
    pub use_original_url: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            application_name: "OPC UA TUI Client".to_string(),
            application_uri: "urn:opcua-tui-client".to_string(),
            security_policy: SecurityPolicy::None,
            security_mode: MessageSecurityMode::None,
            auto_trust: true,
            client_cert_path: None,
            client_key_path: None,
            identity_token: IdentityToken::Anonymous,
            use_original_url: false,
        }
    }
}

impl ConnectionConfig {
    /// Create configuration for UI discovery
    pub fn ui_discovery() -> Self {
        Self {
            application_name: "OPC UA TUI Client - UI Discovery".to_string(),
            application_uri: "urn:opcua-tui-client-ui-discovery".to_string(),
            ..Default::default()
        }
    }

    /// Create configuration for UI connection
    pub fn ui_connection() -> Self {
        Self {
            application_name: "OPC UA TUI Client - UI".to_string(),
            application_uri: "urn:opcua-tui-client-ui".to_string(),
            ..Default::default()
        }
    }

    /// Create configuration for CLI connections
    pub fn cli_connection() -> Self {
        Self {
            application_name: "OPC UA TUI Client - CLI".to_string(),
            application_uri: "urn:opcua-tui-client-cli".to_string(),
            ..Default::default()
        }
    }

    /// Create configuration for secure OPC UA connections with certificates
    /// Automatically extracts application URI from the certificate
    pub fn secure_connection() -> Self {
        let cert_path = "./pki/own/OpcPlc.der";
        let key_path = "./pki/private/OpcPlc.pem";

        // Try to extract application URI from certificate, fallback to default
        let application_uri = ConnectionManager::extract_application_uri_from_certificate(
            cert_path,
        )
        .unwrap_or_else(|e| {
            log::warn!("Failed to extract application URI from certificate: {e}");
            log::info!("Using default application URI");
            "urn:opcua-tui-client-secure".to_string()
        });

        log::info!("Using application URI from certificate: {application_uri}");

        Self {
            application_name: "OPC UA TUI Client - Secure".to_string(),
            application_uri,
            security_policy: SecurityPolicy::Basic256Sha256,
            security_mode: MessageSecurityMode::SignAndEncrypt,
            auto_trust: true,
            client_cert_path: Some(cert_path.to_string()),
            client_key_path: Some(key_path.to_string()),
            identity_token: IdentityToken::Anonymous,
            use_original_url: false,
        }
    }

    /// Set security configuration and automatically extract application URI from certificate
    pub fn with_security_auto_uri(
        mut self,
        policy: SecurityPolicy,
        mode: MessageSecurityMode,
        auto_trust: bool,
        client_cert: Option<String>,
        client_key: Option<String>,
    ) -> Self {
        self.security_policy = policy;
        self.security_mode = mode;
        self.auto_trust = auto_trust;
        self.client_cert_path = client_cert.clone();
        self.client_key_path = client_key;

        // Try to extract application URI from certificate if path is provided
        if let Some(cert_path) = &client_cert {
            if let Ok(extracted_uri) =
                ConnectionManager::extract_application_uri_from_certificate(cert_path)
            {
                log::info!(
                    "Automatically extracted application URI from certificate: {extracted_uri}"
                );
                self.application_uri = extracted_uri;
            } else {
                log::warn!(
                    "Failed to extract application URI from certificate, using current URI: {}",
                    self.application_uri
                );
            }
        }

        self
    }

    /// Set security configuration
    pub fn with_security(
        mut self,
        policy: SecurityPolicy,
        mode: MessageSecurityMode,
        auto_trust: bool,
        client_cert: Option<String>,
        client_key: Option<String>,
    ) -> Self {
        self.security_policy = policy;
        self.security_mode = mode;
        self.auto_trust = auto_trust;
        self.client_cert_path = client_cert;
        self.client_key_path = client_key;
        self
    }

    /// Set authentication
    pub fn with_authentication(mut self, identity_token: IdentityToken) -> Self {
        self.identity_token = identity_token;
        self
    }

    /// Set URL override behavior
    pub fn with_url_override(mut self, use_original_url: bool) -> Self {
        self.use_original_url = use_original_url;
        self
    }
}

impl ConnectionManager {
    /// Discover endpoints from an OPC UA server
    pub async fn discover_endpoints(
        server_url: &str,
        config: &ConnectionConfig,
    ) -> Result<Vec<EndpointDescription>> {
        let url = server_url.to_string();
        let config = config.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<EndpointDescription>> {
            let client = Self::build_discovery_client(&config)?;

            match client.get_server_endpoints_from_url(&url) {
                Ok(endpoints) => {
                    log::info!(
                        "Successfully discovered {} endpoints from server: {}",
                        endpoints.len(),
                        url
                    );

                    // Log discovered endpoints for debugging
                    for (i, endpoint) in endpoints.iter().enumerate() {
                        log::debug!(
                            "  Endpoint {}: {} (Security: {:?} - {:?})",
                            i + 1,
                            endpoint.endpoint_url.as_ref(),
                            endpoint.security_policy_uri.as_ref(),
                            endpoint.security_mode
                        );
                    }

                    Ok(endpoints)
                }
                Err(e) => {
                    log::error!("Failed to discover endpoints from {url}: {e}");
                    Err(anyhow!("Failed to discover endpoints: {}", e))
                }
            }
        })
        .await?
    }

    /// Connect to an OPC UA server using a discovered endpoint
    pub async fn connect_to_endpoint(
        endpoint: EndpointDescription,
        config: &ConnectionConfig,
    ) -> Result<(Client, Arc<RwLock<Session>>)> {
        let config = config.clone();

        // Apply URL override if requested
        if config.use_original_url {
            if let Some(original_url) = endpoint.endpoint_url.value() {
                log::info!("Using original URL override: {original_url}");
            }
        }
        tokio::task::spawn_blocking(move || -> Result<(Client, Arc<RwLock<Session>>)> {
            log::debug!("Building client with config: {config:?}");

            let mut client = match Self::build_client(&config) {
                Ok(c) => {
                    log::debug!("Client built successfully");
                    c
                }
                Err(e) => {
                    log::error!("Failed to build client: {e}");
                    return Err(anyhow!("Client build failed: {}", e));
                }
            };

            log::info!(
                "Connecting to endpoint: {} (Security: {:?} - {:?})",
                endpoint.endpoint_url.as_ref(),
                endpoint.security_policy_uri.as_ref(),
                endpoint.security_mode
            );

            log::debug!("Attempting connection with identity token: {:?}", config.identity_token);

            // Wrap the connection attempt in a panic-catching mechanism
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.connect_to_endpoint(endpoint, config.identity_token)
            })) {
                Ok(Ok(session)) => {
                    log::info!("Successfully established OPC UA connection");
                    Ok((client, session))
                }
                Ok(Err(e)) => {
                    log::error!("Failed to connect to endpoint: {e}");
                    log::debug!("Connection error details: {e:?}");

                    // Provide more specific error analysis
                    let error_msg = e.to_string();
                    if error_msg.contains("BadSecurityChecksFailed") {
                        log::error!("Security checks failed - likely certificate/private key mismatch or untrusted certificate");
                    } else if error_msg.contains("BadCertificateInvalid") {
                        log::error!("Certificate is invalid - check certificate format and validity");
                    } else if error_msg.contains("BadIdentityTokenInvalid") {
                        log::error!("Identity token is invalid - check authentication credentials");
                    }

                    Err(anyhow!("Endpoint connection failed: {}", e))
                }
                Err(panic_info) => {
                    // Handle panics gracefully
                    let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic occurred during connection".to_string()
                    };

                    log::error!("Connection attempt panicked: {panic_msg}");

                    Err(anyhow!("Connection failed due to certificate/key issue: {}", panic_msg))
                }
            }
        })
            .await?
    }

    /// Connect to an OPC UA server by URL (discovers endpoints first)
    pub async fn connect_to_server(
        server_url: &str,
        config: &ConnectionConfig,
    ) -> Result<(Client, Arc<RwLock<Session>>)> {
        log::info!("Starting connection process to: {server_url}");
        log::debug!("Using config: {config:?}");

        // First, discover endpoints
        log::debug!("Discovering endpoints from: {server_url}");
        let endpoints = Self::discover_endpoints(server_url, config).await?;
        log::debug!("Discovered {} endpoints", endpoints.len());

        if endpoints.is_empty() {
            return Err(anyhow!("Server returned no endpoints"));
        }

        // Log all discovered endpoints for debugging
        for (i, endpoint) in endpoints.iter().enumerate() {
            log::debug!(
                "Endpoint {}: {} (Policy: {}, Mode: {:?})",
                i + 1,
                endpoint.endpoint_url.as_ref(),
                Self::policy_uri_to_name(endpoint.security_policy_uri.as_ref()),
                endpoint.security_mode
            );
        }

        // Find matching endpoint
        log::debug!(
            "Looking for endpoint with policy {:?} and mode {:?}",
            config.security_policy,
            config.security_mode
        );
        let selected_endpoint = Self::find_matching_endpoint(
            &endpoints,
            config.security_policy,
            config.security_mode,
            config.use_original_url,
            server_url,
        )?;

        log::info!(
            "Selected endpoint: {} (Security: {:?} - {:?})",
            selected_endpoint.endpoint_url.as_ref(),
            selected_endpoint.security_policy_uri.as_ref(),
            selected_endpoint.security_mode
        );

        // Connect to the selected endpoint
        Self::connect_to_endpoint(selected_endpoint, config).await
    }

    /// Build a configured OPC UA client for regular connections
    fn build_client(config: &ConnectionConfig) -> Result<Client> {
        log::debug!(
            "Building client with application URI: {}",
            config.application_uri
        );

        let mut client_builder = ClientBuilder::new()
            .application_name(&config.application_name)
            .application_uri(&config.application_uri) // Use config URI, not hardcoded
            .session_retry_limit(1)
            .pki_dir("pki")
            .session_retry_interval(1000)
            .verify_server_certs(false); // Disable hostname verification for secure connections

        // Configure security
        if config.security_mode != MessageSecurityMode::None {
            if config.auto_trust {
                log::debug!("Auto-trusting server certificates");
                client_builder = client_builder.trust_server_certs(true);
            }

            if let (Some(cert_path), Some(key_path)) =
                (&config.client_cert_path, &config.client_key_path)
            {
                log::info!("Using client certificate: {cert_path}");
                log::info!("Using client private key: {key_path}");

                // Check if certificate files exist
                if !std::path::Path::new(cert_path).exists() {
                    log::error!("Client certificate file not found: {cert_path}");
                    return Err(anyhow!("Client certificate file not found: {}", cert_path));
                }
                if !std::path::Path::new(key_path).exists() {
                    log::error!("Client private key file not found: {key_path}");
                    return Err(anyhow!("Client private key file not found: {}", key_path));
                }

                // Use PKI directory structure approach (more reliable than absolute paths)
                let cert_filename = std::path::Path::new(cert_path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cert.der");
                let key_filename = std::path::Path::new(key_path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("private.pem");

                log::debug!("Using certificate file: own/{cert_filename}");
                log::debug!("Using private key file: private/{key_filename}");

                // Validate certificate and key files with OpenSSL to catch format and compatibility issues
                if let Err(e) = Self::test_certificate_with_openssl(cert_path, key_path) {
                    log::error!("Certificate validation failed: {e}");
                    return Err(anyhow!("Certificate validation failed: {}. Please check certificate and private key formats and compatibility.", e));
                }

                client_builder = client_builder
                    .certificate_path(format!("own/{cert_filename}"))
                    .private_key_path(format!("private/{key_filename}"))
                    .create_sample_keypair(false);
            } else {
                // Fallback to sample keypair if no certificates provided
                log::debug!("No client certificates provided, using sample keypair");
                client_builder = client_builder.create_sample_keypair(true);
            }
        } else {
            client_builder = client_builder.trust_server_certs(true);
        }

        // Wrap client creation in panic handler
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client_builder.client())) {
            Ok(Some(client)) => {
                log::debug!("Client created successfully");
                Ok(client)
            }
            Ok(None) => {
                log::error!("Failed to create client - builder returned None");
                Err(anyhow!("Failed to create client"))
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic during client creation".to_string()
                };

                log::error!("Client creation panicked: {panic_msg}");
                log::error!("This usually indicates certificate/private key loading issues");
                Err(anyhow!("Client creation failed: {}", panic_msg))
            }
        }
    }

    /// Build a configured OPC UA client for discovery operations (with short timeout)
    fn build_discovery_client(config: &ConnectionConfig) -> Result<Client> {
        let client_builder = ClientBuilder::new()
            .application_name(&config.application_name)
            .application_uri(&config.application_uri)
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(1)
            .session_timeout(10000); // 10 second session timeout for discovery only

        client_builder
            .client()
            .ok_or_else(|| anyhow!("Failed to create discovery client"))
    }

    /// Find an endpoint that matches the requested security settings
    fn find_matching_endpoint(
        endpoints: &[EndpointDescription],
        requested_policy: SecurityPolicy,
        requested_mode: MessageSecurityMode,
        use_original_url: bool,
        original_url: &str,
    ) -> Result<EndpointDescription> {
        // Convert security policy to URI string for comparison
        let requested_policy_uri = Self::security_policy_to_uri(requested_policy)?;

        // Find matching endpoint
        let matching_endpoint = endpoints.iter().find(|endpoint| {
            endpoint.security_policy_uri.as_ref() == requested_policy_uri
                && endpoint.security_mode == requested_mode
        });

        match matching_endpoint {
            Some(endpoint) => {
                let mut selected_endpoint = endpoint.clone();

                // If use_original_url is enabled, override the endpoint URL
                if use_original_url {
                    log::info!(
                        "Using original URL override: {} -> {}",
                        endpoint.endpoint_url.as_ref(),
                        original_url
                    );
                    selected_endpoint.endpoint_url = UAString::from(original_url);
                }

                Ok(selected_endpoint)
            }
            None => {
                // No matching endpoint found, provide helpful error message
                let available_endpoints: Vec<String> = endpoints
                    .iter()
                    .map(|ep| {
                        format!(
                            "  - {} (Security: {} - {:?})",
                            ep.endpoint_url.as_ref(),
                            Self::policy_uri_to_name(ep.security_policy_uri.as_ref()),
                            ep.security_mode
                        )
                    })
                    .collect();

                Err(anyhow!(
                    "No endpoint found matching security policy '{:?}' and mode '{:?}'.\n\nAvailable endpoints:\n{}",
                    requested_policy,
                    requested_mode,
                    available_endpoints.join("\n")
                ))
            }
        }
    }

    /// Convert SecurityPolicy enum to URI string
    fn security_policy_to_uri(policy: SecurityPolicy) -> Result<&'static str> {
        match policy {
            SecurityPolicy::None => Ok("http://opcfoundation.org/UA/SecurityPolicy#None"),
            SecurityPolicy::Basic128Rsa15 => {
                Ok("http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15")
            }
            SecurityPolicy::Basic256 => Ok("http://opcfoundation.org/UA/SecurityPolicy#Basic256"),
            SecurityPolicy::Basic256Sha256 => {
                Ok("http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256")
            }
            SecurityPolicy::Aes128Sha256RsaOaep => {
                Ok("http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep")
            }
            SecurityPolicy::Aes256Sha256RsaPss => {
                Ok("http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss")
            }
            SecurityPolicy::Unknown => Err(anyhow!("Unknown security policy not supported")),
        }
    }

    /// Convert URI string to human-readable policy name
    fn policy_uri_to_name(uri: &str) -> &str {
        match uri {
            "http://opcfoundation.org/UA/SecurityPolicy#None" => "None",
            "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15" => "Basic128Rsa15",
            "http://opcfoundation.org/UA/SecurityPolicy#Basic256" => "Basic256",
            "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256" => "Basic256Sha256",
            "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep" => {
                "Aes128Sha256RsaOaep"
            }
            "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss" => {
                "Aes256Sha256RsaPss"
            }
            _ => "Unknown",
        }
    }

    /// Test certificate and private key using OpenSSL library
    /// This validates format and compatibility before the OPC UA client tries to use them
    fn test_certificate_with_openssl(cert_path: &str, key_path: &str) -> Result<()> {
        use std::fs;

        log::debug!(
            "Testing certificate compatibility with OpenSSL for: {cert_path} and {key_path}"
        );

        // Read certificate file
        let cert_data =
            fs::read(cert_path).map_err(|e| anyhow!("Cannot read certificate file: {}", e))?;

        if cert_data.is_empty() {
            return Err(anyhow!("Certificate file is empty"));
        }

        // Try to parse certificate with OpenSSL
        let cert = if cert_data.starts_with(b"-----BEGIN CERTIFICATE-----") {
            // PEM format
            openssl::x509::X509::from_pem(&cert_data)
                .map_err(|e| anyhow!("Failed to parse PEM certificate: {}", e))?
        } else {
            // DER format
            openssl::x509::X509::from_der(&cert_data)
                .map_err(|e| anyhow!("Failed to parse DER certificate: {}", e))?
        };

        log::debug!("Certificate parsed successfully with OpenSSL");

        // Read private key file
        let key_data =
            fs::read(key_path).map_err(|e| anyhow!("Cannot read private key file: {}", e))?;

        if key_data.is_empty() {
            return Err(anyhow!("Private key file is empty"));
        }

        // Try to parse private key with OpenSSL
        let private_key = openssl::pkey::PKey::private_key_from_pem(&key_data)
            .map_err(|e| anyhow!("Failed to parse private key: {}. Make sure the key is in PEM format and not encrypted.", e))?;

        log::debug!("Private key parsed successfully with OpenSSL");

        // Test if the private key matches the certificate's public key
        let cert_public_key = cert
            .public_key()
            .map_err(|e| anyhow!("Failed to extract public key from certificate: {}", e))?;

        // Compare the public keys (this validates that the private key matches the certificate)
        let private_key_public = private_key
            .public_key_to_pem()
            .map_err(|e| anyhow!("Failed to extract public key from private key: {}", e))?;
        let cert_key_public = cert_public_key
            .public_key_to_pem()
            .map_err(|e| anyhow!("Failed to convert certificate public key to PEM: {}", e))?;

        if private_key_public != cert_key_public {
            return Err(anyhow!(
                "Private key does not match the certificate's public key"
            ));
        }

        log::info!("Certificate and private key validation successful - they are compatible");
        Ok(())
    }

    /// Extract application URI from an X.509 certificate
    /// This reads the Subject Alternative Name (SAN) extension to find the application URI
    fn extract_application_uri_from_certificate(cert_path: &str) -> Result<String> {
        use std::fs;

        log::debug!("Extracting application URI from certificate: {cert_path}");

        // Read certificate file
        let cert_data =
            fs::read(cert_path).map_err(|e| anyhow!("Cannot read certificate file: {}", e))?;

        if cert_data.is_empty() {
            return Err(anyhow!("Certificate file is empty"));
        }

        // Parse certificate with OpenSSL
        let cert = if cert_data.starts_with(b"-----BEGIN CERTIFICATE-----") {
            // PEM format
            openssl::x509::X509::from_pem(&cert_data)
                .map_err(|e| anyhow!("Failed to parse PEM certificate: {}", e))?
        } else {
            // DER format
            openssl::x509::X509::from_der(&cert_data)
                .map_err(|e| anyhow!("Failed to parse DER certificate: {}", e))?
        };

        log::debug!("Certificate parsed successfully, extracting Subject Alternative Name");

        // Get Subject Alternative Name extension
        let subject_alt_names = cert.subject_alt_names();

        if let Some(san_list) = subject_alt_names {
            for san in san_list {
                // Check if this is a URI entry
                if let Some(uri) = san.uri() {
                    let uri_str = uri.to_string();
                    log::debug!("Found URI in SAN: {uri_str}");

                    // OPC UA application URIs typically start with "urn:"
                    // and often contain "opcua" or similar identifiers
                    if uri_str.starts_with("urn:") {
                        log::info!("Found application URI in certificate: {uri_str}");
                        return Ok(uri_str);
                    }
                }
            }
        }

        // If no URI found in SAN, try to get it from subject CN as fallback
        let subject = cert.subject_name();
        for entry in subject.entries() {
            let obj = entry.object();
            if obj.nid() == openssl::nid::Nid::COMMONNAME {
                if let Ok(data) = entry.data().as_utf8() {
                    let cn = data.to_string();
                    log::debug!("Found CN in certificate: {cn}");

                    // If CN looks like a URN, use it
                    if cn.starts_with("urn:") {
                        log::info!("Using CN as application URI: {cn}");
                        return Ok(cn);
                    }
                }
            }
        }

        Err(anyhow!(
            "No application URI found in certificate Subject Alternative Name or Common Name"
        ))
    }
}
