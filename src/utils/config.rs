use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub network: NetworkSettings,
    pub database: DatabaseSettings,
    pub security: SecuritySettings,
    pub limits: LimitSettings,
    pub features: FeatureSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerSettings {
    pub name: String,
    pub description: String,
    pub listen_addresses: Vec<String>,
    pub tls_listen_addresses: Vec<String>,
    pub motd_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkSettings {
    pub name: String,
    pub admin_name: String,
    pub admin_email: String,
    pub server_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecuritySettings {
    pub tls_cert_file: Option<String>,
    pub tls_key_file: Option<String>,
    pub require_tls: bool,
    pub min_tls_version: String,
    pub password_hash_algorithm: String,
    pub operator_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LimitSettings {
    pub max_clients: usize,
    pub max_clients_per_ip: usize,
    pub max_channels_per_user: usize,
    pub max_nickname_length: usize,
    pub max_channel_name_length: usize,
    pub max_topic_length: usize,
    pub max_message_length: usize,
    pub max_away_length: usize,
    pub ping_frequency: u64,
    pub ping_timeout: u64,
    pub flood_messages: usize,
    pub flood_interval: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureSettings {
    pub enable_sasl: bool,
    pub enable_message_tags: bool,
    pub enable_server_time: bool,
    pub enable_account_notify: bool,
    pub enable_extended_join: bool,
    pub enable_batch: bool,
    pub enable_labeled_response: bool,
    pub enable_echo_message: bool,
    pub enable_userhost_in_names: bool,
    pub enable_invite_notify: bool,
    pub enable_away_notify: bool,
    pub enable_chghost: bool,
    pub enable_cap_notify: bool,
    pub enable_multi_prefix: bool,
    pub enable_setname: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                name: "ironchatd.local".to_string(),
                description: "IronChat IRC Server".to_string(),
                listen_addresses: vec!["127.0.0.1:6667".to_string()],
                tls_listen_addresses: vec!["127.0.0.1:6697".to_string()],
                motd_file: None,
            },
            network: NetworkSettings {
                name: "IronChat".to_string(),
                admin_name: "Admin".to_string(),
                admin_email: "admin@example.com".to_string(),
                server_id: "001".to_string(),
            },
            database: DatabaseSettings {
                url: "sqlite://ironchatd.db".to_string(),
                max_connections: 10,
                connection_timeout: 30,
            },
            security: SecuritySettings {
                tls_cert_file: None,
                tls_key_file: None,
                require_tls: false,
                min_tls_version: "1.2".to_string(),
                password_hash_algorithm: "argon2".to_string(),
                operator_password: None,
            },
            limits: LimitSettings {
                max_clients: 10000,
                max_clients_per_ip: 10,
                max_channels_per_user: 50,
                max_nickname_length: 30,
                max_channel_name_length: 50,
                max_topic_length: 390,
                max_message_length: 512,
                max_away_length: 255,
                ping_frequency: 120,
                ping_timeout: 60,
                flood_messages: 10,
                flood_interval: 1,
            },
            features: FeatureSettings {
                enable_sasl: true,
                enable_message_tags: true,
                enable_server_time: true,
                enable_account_notify: true,
                enable_extended_join: true,
                enable_batch: true,
                enable_labeled_response: true,
                enable_echo_message: true,
                enable_userhost_in_names: true,
                enable_invite_notify: true,
                enable_away_notify: true,
                enable_chghost: true,
                enable_cap_notify: true,
                enable_multi_prefix: true,
                enable_setname: true,
            },
        }
    }
}

impl ServerConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::from(path.as_ref()))
            .build()?;
        
        config.try_deserialize()
    }
    
    pub fn load_with_defaults(path: Option<impl AsRef<Path>>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();
        
        // Start with defaults
        builder = builder.add_source(Config::try_from(&Self::default())?);
        
        // Override with file if provided
        if let Some(p) = path {
            builder = builder.add_source(File::from(p.as_ref()).required(false));
        }
        
        let config = builder.build()?;
        config.try_deserialize()
    }
}