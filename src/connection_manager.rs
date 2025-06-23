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

impl ConnectionConfig {    /// Create configuration for CLI discovery
    pub fn cli_discovery() -> Self {
        Self {
            application_name: "OPC UA TUI Client - CLI Discovery".to_string(),
            application_uri: "urn:opcua-tui-client-cli-discovery".to_string(),
            ..Default::default()
        }
    }

    /// Create configuration for UI discovery
    pub fn ui_discovery() -> Self {
        Self {
            application_name: "OPC UA TUI Client - UI Discovery".to_string(),
            application_uri: "urn:opcua-tui-client-ui-discovery".to_string(),
            ..Default::default()
        }
    }

    /// Create configuration for CLI connection
    pub fn cli_connection() -> Self {
        Self {
            application_name: "OPC UA TUI Client - CLI".to_string(),
            application_uri: "urn:opcua-tui-client-cli".to_string(),
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
    }    /// Set URL override behavior
    pub fn with_url_override(mut self, use_original_url: bool) -> Self {
        self.use_original_url = use_original_url;
        self
    }
}

impl ConnectionManager {    /// Discover endpoints from an OPC UA server
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
                    log::error!("Failed to discover endpoints from {}: {}", url, e);
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
        let endpoint = endpoint;

        // Apply URL override if requested
        if config.use_original_url {
            if let Some(original_url) = endpoint.endpoint_url.value() {
                log::info!("Using original URL override: {}", original_url);
            }
        }        tokio::task::spawn_blocking(move || -> Result<(Client, Arc<RwLock<Session>>)> {
            let mut client = Self::build_client(&config)?;

            log::info!(
                "Connecting to endpoint: {} (Security: {:?} - {:?})",
                endpoint.endpoint_url.as_ref(),
                endpoint.security_policy_uri.as_ref(),
                endpoint.security_mode
            );

            let session = client.connect_to_endpoint(endpoint, config.identity_token)?;

            log::info!("Successfully established OPC UA connection");

            Ok((client, session))
        })
        .await?
    }

    /// Connect to an OPC UA server by URL (discovers endpoints first)
    pub async fn connect_to_server(
        server_url: &str,
        config: &ConnectionConfig,
    ) -> Result<(Client, Arc<RwLock<Session>>)> {
        log::info!("Starting connection process to: {}", server_url);

        // First, discover endpoints
        let endpoints = Self::discover_endpoints(server_url, config).await?;

        if endpoints.is_empty() {
            return Err(anyhow!("Server returned no endpoints"));
        }

        // Find matching endpoint
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
    }    /// Build a configured OPC UA client for regular connections
    fn build_client(config: &ConnectionConfig) -> Result<Client> {
        let mut client_builder = ClientBuilder::new()
            .application_name(&config.application_name)
            .application_uri(&config.application_uri)
            .session_retry_limit(1)
            .session_retry_interval(1000);

        // Configure security
        if config.security_mode != MessageSecurityMode::None {
            if config.auto_trust {
                log::debug!("Auto-trusting server certificates");
                client_builder = client_builder.trust_server_certs(true);
            }

            if let (Some(cert_path), Some(_key_path)) = (&config.client_cert_path, &config.client_key_path) {
                log::debug!("Using client certificate: {}", cert_path);
                // For now, use sample keypair - in production you'd load actual files
                client_builder = client_builder.create_sample_keypair(true);
            } else {
                client_builder = client_builder.create_sample_keypair(true);
            }
        } else {
            client_builder = client_builder.trust_server_certs(true);
        }

        client_builder
            .client()
            .ok_or_else(|| anyhow!("Failed to create client"))
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
}
