use super::types::*;
use super::validator::AuthInputs;
use crate::client::ConnectionStatus;
use anyhow::{anyhow, Result};
use log::{error, info, warn};
use opcua::client::prelude::*;
use opcua::types::EndpointDescription;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

pub struct SecurityConfig {
    pub auto_trust: bool,
    pub client_cert_path: String,
    pub client_key_path: String,
}

pub struct ConnectionBuilder {
    endpoint: EndpointDescription,
    identity_token: Option<IdentityToken>,
    security_config: Option<SecurityConfig>,
}

impl ConnectionBuilder {
    pub fn new(endpoint: EndpointDescription) -> Self {
        Self {
            endpoint,
            identity_token: None,
            security_config: None,
        }
    }

    pub fn with_identity(
        mut self,
        auth_type: &AuthenticationType,
        inputs: &AuthInputs,
    ) -> Result<Self> {
        self.identity_token = Some(match auth_type {
            AuthenticationType::Anonymous => IdentityToken::Anonymous,
            AuthenticationType::UserPassword => {
                self.validate_user_password(inputs)?;
                IdentityToken::UserName(inputs.username.clone(), inputs.password.clone())
            }
            AuthenticationType::X509Certificate => {
                self.validate_x509_certificate(inputs)?;
                IdentityToken::X509(
                    PathBuf::from(&inputs.cert_path),
                    PathBuf::from(&inputs.key_path),
                )
            }
        });
        Ok(self)
    }

    pub fn with_security(mut self, config: SecurityConfig) -> Self {
        self.security_config = Some(config);
        self
    }
    pub async fn connect(self) -> Result<(Client, Arc<RwLock<Session>>)> {
        use crate::connection_manager::{ConnectionConfig, ConnectionManager};

        let identity_token = self
            .identity_token
            .clone()
            .ok_or_else(|| anyhow!("Identity token not set"))?;

        // Parse security policy from endpoint
        let security_policy = Self::parse_security_policy(&self.endpoint.security_policy_uri);

        log::info!(
            "Connection attempt with endpoint security URI: '{}' -> policy: {:?}, mode: {:?}",
            self.endpoint.security_policy_uri.as_ref(),
            security_policy,
            self.endpoint.security_mode
        );

        // Log security config for debugging
        if let Some(ref sec_config) = self.security_config {
            log::info!(
                "Security config - cert: '{}', key: '{}', auto_trust: {}",
                sec_config.client_cert_path,
                sec_config.client_key_path,
                sec_config.auto_trust
            );
        } else {
            log::info!("No security config provided");
        }

        // Use secure connection configuration if:
        // 1. The endpoint uses a secure policy (not None), OR
        // 2. The endpoint uses secure mode (not None), OR
        // 3. Client certificates are provided
        let use_secure_config = security_policy != opcua::crypto::SecurityPolicy::None
            || self.endpoint.security_mode != opcua::types::MessageSecurityMode::None
            || self
                .security_config
                .as_ref()
                .map(|c| !c.client_cert_path.is_empty() && !c.client_key_path.is_empty())
                .unwrap_or(false);

        log::info!("Use secure config decision: {} (policy={:?} != None: {}, mode={:?} != None: {}, certs provided: {})", 
                   use_secure_config,
                   security_policy, security_policy != opcua::crypto::SecurityPolicy::None,
                   self.endpoint.security_mode, self.endpoint.security_mode != opcua::types::MessageSecurityMode::None,
                   self.security_config.as_ref().map(|c| !c.client_cert_path.is_empty() && !c.client_key_path.is_empty()).unwrap_or(false));

        let config = if use_secure_config {
            log::info!("Using secure connection configuration");
            // Use secure connection configuration with automatic URI extraction from certificate
            ConnectionConfig::secure_connection()
                .with_security_auto_uri(
                    security_policy,
                    self.endpoint.security_mode,
                    self.security_config
                        .as_ref()
                        .map(|c| c.auto_trust)
                        .unwrap_or(true),
                    self.security_config.as_ref().and_then(|c| {
                        if c.client_cert_path.is_empty() {
                            None
                        } else {
                            Some(c.client_cert_path.clone())
                        }
                    }),
                    self.security_config.as_ref().and_then(|c| {
                        if c.client_key_path.is_empty() {
                            None
                        } else {
                            Some(c.client_key_path.clone())
                        }
                    }),
                )
                .with_authentication(identity_token)
        } else {
            log::info!("Using regular UI connection configuration");
            // Use regular UI connection configuration for non-secure connections
            ConnectionConfig::ui_connection()
                .with_security(
                    security_policy,
                    self.endpoint.security_mode,
                    self.security_config
                        .as_ref()
                        .map(|c| c.auto_trust)
                        .unwrap_or(true),
                    self.security_config.as_ref().and_then(|c| {
                        if c.client_cert_path.is_empty() {
                            None
                        } else {
                            Some(c.client_cert_path.clone())
                        }
                    }),
                    self.security_config.as_ref().and_then(|c| {
                        if c.client_key_path.is_empty() {
                            None
                        } else {
                            Some(c.client_key_path.clone())
                        }
                    }),
                )
                .with_authentication(identity_token)
        };

        log::info!(
            "Final connection config: policy={:?}, mode={:?}, app_uri={}",
            config.security_policy,
            config.security_mode,
            config.application_uri
        );
        ConnectionManager::connect_to_endpoint(self.endpoint, &config).await
    }

    fn parse_security_policy(uri: &opcua::types::UAString) -> opcua::crypto::SecurityPolicy {
        match uri.as_ref() {
            "http://opcfoundation.org/UA/SecurityPolicy#None" => {
                opcua::crypto::SecurityPolicy::None
            }
            "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15" => {
                opcua::crypto::SecurityPolicy::Basic128Rsa15
            }
            "http://opcfoundation.org/UA/SecurityPolicy#Basic256" => {
                opcua::crypto::SecurityPolicy::Basic256
            }
            "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256" => {
                opcua::crypto::SecurityPolicy::Basic256Sha256
            }
            "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep" => {
                opcua::crypto::SecurityPolicy::Aes128Sha256RsaOaep
            }
            "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss" => {
                opcua::crypto::SecurityPolicy::Aes256Sha256RsaPss
            }
            _ => opcua::crypto::SecurityPolicy::None,
        }
    }

    fn validate_user_password(&self, inputs: &AuthInputs) -> Result<()> {
        if inputs.username.trim().is_empty() {
            return Err(anyhow!(
                "Username is required for user/password authentication"
            ));
        }
        Ok(())
    }

    fn validate_x509_certificate(&self, inputs: &AuthInputs) -> Result<()> {
        // Validate certificate file
        if inputs.cert_path.trim().is_empty() {
            return Err(anyhow!(
                "Certificate file path is required for X509 authentication"
            ));
        }

        let cert_path = std::path::Path::new(&inputs.cert_path);
        if !cert_path.exists() {
            return Err(anyhow!(
                "Certificate file does not exist: {}",
                inputs.cert_path
            ));
        }

        // Check certificate file extension
        let cert_ext = cert_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !["der", "pem", "crt", "cer"].contains(&cert_ext.to_lowercase().as_str()) {
            return Err(anyhow!(
                "Certificate file must have .der, .pem, .crt, or .cer extension"
            ));
        }

        // Validate private key file
        if inputs.key_path.trim().is_empty() {
            return Err(anyhow!(
                "Private key file path is required for X509 authentication"
            ));
        }

        let key_path = std::path::Path::new(&inputs.key_path);
        if !key_path.exists() {
            return Err(anyhow!(
                "Private key file does not exist: {}",
                inputs.key_path
            ));
        }

        // Check private key file extension
        let key_ext = key_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !["pem", "key"].contains(&key_ext.to_lowercase().as_str()) {
            return Err(anyhow!("Private key file must have .pem or .key extension"));
        }

        // Log successful validation
        info!("X509 certificate validation successful");
        info!("Certificate: {}", inputs.cert_path);
        info!("Private key: {}", inputs.key_path);

        Ok(())
    }
}

impl ConnectScreen {
    pub async fn connect_with_settings(&mut self) -> Result<Option<ConnectionStatus>> {
        self.show_auth_validation = true;

        let validation_errors = self.validate_authentication_fields();
        if !validation_errors.is_empty() {
            for error in &validation_errors {
                log::error!("Authentication Validation: {}", error);
            }
            return Ok(None);
        }

        self.connect_in_progress = true;
        self.pending_connection = true;
        Ok(None)
    }

    pub async fn perform_connection(&mut self) -> Result<Option<ConnectionStatus>> {
        info!("Starting connection process...");

        if self.discovered_endpoints.is_empty() {
            error!("No endpoints available for connection");
            return Ok(Some(ConnectionStatus::Error(
                "No endpoints available".to_string(),
            )));
        }

        if self.selected_endpoint_index >= self.discovered_endpoints.len() {
            error!("Invalid endpoint selection");
            return Ok(Some(ConnectionStatus::Error(
                "Invalid endpoint selection".to_string(),
            )));
        }

        let selected_endpoint = &self.discovered_endpoints[self.selected_endpoint_index];
        let endpoint = selected_endpoint.original_endpoint.clone();

        let auth_desc = self.get_auth_description();
        info!(
            "Connecting to endpoint: {} with {}",
            selected_endpoint.display_name, auth_desc
        );

        let auth_inputs = self.collect_auth_inputs();
        let security_config = self.collect_security_config();
        info!(
            "Security Config: {}, {}, {}",
            security_config.client_cert_path,
            security_config.client_key_path,
            security_config.auto_trust
        );
        let connection_result = match ConnectionBuilder::new(endpoint)
            .with_identity(&self.authentication_type, &auth_inputs)?
            .with_security(security_config)
            .connect()
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!("Connection failed: {}", e);
                return Ok(Some(ConnectionStatus::Error(format!(
                    "Connection failed: {}",
                    e
                ))));
            }
        };

        let (client, session) = connection_result;

        self.client = Some(client);
        self.session = Some(session);

        info!("OPC UA connection established successfully");
        Ok(Some(ConnectionStatus::Connecting))
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(session) = self.session.take() {
            let disconnect_result = tokio::task::spawn_blocking(move || {
                session.write().disconnect();
            })
            .await;

            if let Err(e) = disconnect_result {
                warn!("Error during session disconnect: {}", e);
            }
        }

        self.client = None;
        info!("Disconnected from OPC UA server");
        Ok(())
    }

    /// Take ownership of the client (moving it out)
    pub fn take_client(&mut self) -> Option<Client> {
        self.client.take()
    }

    /// Take ownership of the session (moving it out)
    pub fn take_session(&mut self) -> Option<Arc<RwLock<Session>>> {
        self.session.take()
    }

    fn get_auth_description(&self) -> String {
        match self.authentication_type {
            AuthenticationType::Anonymous => "Anonymous".to_string(),
            AuthenticationType::UserPassword => format!("User: {}", self.username_input.value()),
            AuthenticationType::X509Certificate => format!(
                "Certificate: {}",
                std::path::Path::new(self.user_certificate_input.value())
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
            ),
        }
    }
    fn collect_security_config(&self) -> SecurityConfig {
        SecurityConfig {
            auto_trust: self.auto_trust_server_cert,
            client_cert_path: self.client_certificate_input.value().trim().to_string(),
            client_key_path: self.client_private_key_input.value().trim().to_string(),
        }
    }

    pub fn get_client(&self) -> Option<&Client> {
        self.client.as_ref()
    }

    pub fn get_session(&self) -> Option<&Arc<RwLock<Session>>> {
        self.session.as_ref()
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some() && self.session.is_some()
    }
}
