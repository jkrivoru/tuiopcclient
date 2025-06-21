use super::types::*;

pub struct ConnectValidator;

impl ConnectValidator {
    pub fn validate_server_url(url: &str) -> Result<(), String> {
        if url.trim().is_empty() {
            return Err("Server URL cannot be empty".to_string());
        }
        if !url.starts_with("opc.tcp://") {
            return Err("URL must start with 'opc.tcp://'".to_string());
        }
        Ok(())
    }
    
    pub fn validate_authentication(auth_type: &AuthenticationType, inputs: &AuthInputs) -> Vec<String> {
        let mut errors = Vec::new();
        
        match auth_type {
            AuthenticationType::UserPassword => {
                if inputs.username.trim().is_empty() {
                    errors.push("Username is required".to_string());
                }
            }
            AuthenticationType::X509Certificate => {
                if inputs.cert_path.trim().is_empty() {
                    errors.push("Certificate path is required".to_string());
                }
                if inputs.key_path.trim().is_empty() {
                    errors.push("Private key path is required".to_string());
                }
            }
            _ => {}
        }
        
        errors
    }

    pub fn validate_security_fields(fields: &SecurityFields) -> Vec<String> {
        let mut errors = Vec::new();
        
        if !fields.auto_trust && fields.trusted_store_path.trim().is_empty() {
            errors.push("Trusted server store path is required when auto-trust is disabled".to_string());
        }
        
        errors
    }
}

pub struct AuthInputs {
    pub username: String,
    pub password: String,
    pub cert_path: String,
    pub key_path: String,
}

pub struct SecurityFields {
    pub auto_trust: bool,
    pub client_cert_path: String,
    pub client_key_path: String,
    pub trusted_store_path: String,
}

impl ConnectScreen {
    pub fn collect_auth_inputs(&self) -> AuthInputs {
        AuthInputs {
            username: self.username_input.value().trim().to_string(),
            password: self.password_input.value().to_string(),
            cert_path: self.user_certificate_input.value().trim().to_string(),
            key_path: self.user_private_key_input.value().trim().to_string(),
        }
    }

    pub fn collect_security_fields(&self) -> SecurityFields {
        SecurityFields {
            auto_trust: self.auto_trust_server_cert,
            client_cert_path: self.client_certificate_input.value().trim().to_string(),
            client_key_path: self.client_private_key_input.value().trim().to_string(),
            trusted_store_path: self.trusted_server_store_input.value().trim().to_string(),
        }
    }
}
