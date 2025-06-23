use opcua::crypto::SecurityPolicy;
use opcua::types::{
    ApplicationDescription, ByteString, EndpointDescription, MessageSecurityMode, UAString,
};

/// Utility functions for creating OPC UA endpoints
pub struct EndpointUtils;

impl EndpointUtils {
    /// Create a default endpoint with None security for testing purposes
    pub fn create_default_endpoint(endpoint_url: &str) -> EndpointDescription {
        EndpointDescription {
            endpoint_url: UAString::from(endpoint_url),
            security_mode: MessageSecurityMode::None,
            security_policy_uri: SecurityPolicy::None.to_uri().into(),
            server_certificate: ByteString::null(),
            user_identity_tokens: None,
            transport_profile_uri: UAString::null(),
            security_level: 0,
            server: ApplicationDescription::default(),
        }
    }

    /// Create an endpoint with specific security settings
    pub fn create_endpoint(
        endpoint_url: &str,
        security_mode: MessageSecurityMode,
        security_policy: SecurityPolicy,
        security_level: u8,
    ) -> EndpointDescription {
        EndpointDescription {
            endpoint_url: UAString::from(endpoint_url),
            security_mode,
            security_policy_uri: security_policy.to_uri().into(),
            server_certificate: ByteString::null(),
            user_identity_tokens: None,
            transport_profile_uri: UAString::null(),
            security_level,
            server: ApplicationDescription::default(),
        }
    }
}
