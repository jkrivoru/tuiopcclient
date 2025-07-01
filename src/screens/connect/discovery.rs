use super::types::{
    ConnectScreen, EndpointInfo, SecurityMode, SecurityPolicy as LocalSecurityPolicy,
};
use anyhow::{anyhow, Result};
use log::{error, info, warn};
use opcua::types::{EndpointDescription, MessageSecurityMode, UAString};

pub struct EndpointConverter {
    use_original_url: bool,
    original_url: String,
}

impl EndpointConverter {
    pub fn new(use_original_url: bool, original_url: String) -> Self {
        Self {
            use_original_url,
            original_url,
        }
    }

    pub fn convert(&self, endpoint: EndpointDescription) -> Option<EndpointInfo> {
        let security_policy = self.parse_security_policy(&endpoint.security_policy_uri)?;
        let security_mode = self.parse_security_mode(endpoint.security_mode)?;
        let display_name = self.create_display_name(&security_policy, &security_mode);

        Some(EndpointInfo {
            security_policy,
            security_mode,
            display_name,
            original_endpoint: if self.use_original_url {
                self.create_endpoint_with_original_url(endpoint)
            } else {
                endpoint
            },
        })
    }
    fn parse_security_policy(&self, uri: &UAString) -> Option<LocalSecurityPolicy> {
        let uri_str = uri.as_ref();
        match uri_str {
            crate::screens::connect::constants::security_policies::NONE => {
                Some(LocalSecurityPolicy::None)
            }
            crate::screens::connect::constants::security_policies::BASIC128_RSA15 => {
                Some(LocalSecurityPolicy::Basic128Rsa15)
            }
            crate::screens::connect::constants::security_policies::BASIC256 => {
                Some(LocalSecurityPolicy::Basic256)
            }
            crate::screens::connect::constants::security_policies::BASIC256_SHA256 => {
                Some(LocalSecurityPolicy::Basic256Sha256)
            }
            crate::screens::connect::constants::security_policies::AES128_SHA256_RSA_OAEP => {
                Some(LocalSecurityPolicy::Aes128Sha256RsaOaep)
            }
            crate::screens::connect::constants::security_policies::AES256_SHA256_RSA_PSS => {
                Some(LocalSecurityPolicy::Aes256Sha256RsaPss)
            }
            _ => {
                warn!("Unknown security policy: {uri_str}");
                None
            }
        }
    }

    fn parse_security_mode(&self, mode: MessageSecurityMode) -> Option<SecurityMode> {
        match mode {
            MessageSecurityMode::None => Some(SecurityMode::None),
            MessageSecurityMode::Sign => Some(SecurityMode::Sign),
            MessageSecurityMode::SignAndEncrypt => Some(SecurityMode::SignAndEncrypt),
            _ => {
                warn!("Unknown security mode: {mode:?}");
                None
            }
        }
    }
    fn create_display_name(&self, policy: &LocalSecurityPolicy, mode: &SecurityMode) -> String {
        match (policy, mode) {
            (LocalSecurityPolicy::None, SecurityMode::None) => "None - No Security".to_string(),
            (policy, mode) => format!("{policy:?} - {mode:?}"),
        }
    }

    fn create_endpoint_with_original_url(
        &self,
        mut endpoint: EndpointDescription,
    ) -> EndpointDescription {
        endpoint.endpoint_url = UAString::from(self.original_url.clone());
        endpoint
    }
}

impl ConnectScreen {
    pub async fn discover_endpoints(&mut self) -> Result<()> {
        info!("Discovering endpoints...");

        let url = self.get_server_url();
        if !url.starts_with("opc.tcp://") {
            error!("Invalid OPC UA server URL: must start with 'opc.tcp://'");
            return Err(anyhow!("Invalid URL format"));
        }

        info!("Querying OPC UA server for available endpoints: {url}");

        if self.use_original_url {
            info!("Original URL override is enabled - will use '{url}' instead of server-provided endpoint URLs");
        }

        let endpoints_result = self.discover_from_server(&url).await?;

        if endpoints_result.is_empty() {
            error!("Server returned no endpoints");
            return Err(anyhow!("Server returned no endpoints"));
        }

        let converter = EndpointConverter::new(self.use_original_url, url);
        self.discovered_endpoints = endpoints_result
            .into_iter()
            .filter_map(|endpoint| converter.convert(endpoint))
            .collect();

        if self.discovered_endpoints.is_empty() {
            error!("No valid endpoints found after filtering");
            return Err(anyhow!("No valid endpoints found"));
        }

        // Log discovered endpoints
        for (i, endpoint) in self.discovered_endpoints.iter().enumerate() {
            info!("Endpoint {}: {}", i + 1, endpoint.display_name);
        }

        Ok(())
    }
    async fn discover_from_server(&self, url: &str) -> Result<Vec<EndpointDescription>> {
        use crate::connection_manager::{ConnectionConfig, ConnectionManager};

        let config = ConnectionConfig::ui_discovery();
        ConnectionManager::discover_endpoints(url, &config).await
    }
}
