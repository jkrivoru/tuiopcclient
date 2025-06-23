pub mod security_policies {
    pub const NONE: &str = "http://opcfoundation.org/UA/SecurityPolicy#None";
    pub const BASIC128_RSA15: &str = "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15";
    pub const BASIC256: &str = "http://opcfoundation.org/UA/SecurityPolicy#Basic256";
    pub const BASIC256_SHA256: &str = "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256";
    pub const AES128_SHA256_RSA_OAEP: &str =
        "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep";
    pub const AES256_SHA256_RSA_PSS: &str =
        "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss";
}

pub mod ui {
    pub const DEFAULT_SERVER_URL: &str = "opc.tcp://localhost:4840";
    pub const DISCOVERY_CLIENT_NAME: &str = "OPC UA TUI Client - Discovery";
    pub const DISCOVERY_CLIENT_URI: &str = "urn:opcua-tui-client-discovery";
    pub const CLIENT_NAME: &str = "OPC UA TUI Client";
    pub const CLIENT_URI: &str = "urn:opcua-tui-client";
}
