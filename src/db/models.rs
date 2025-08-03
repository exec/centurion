use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub nickname: String,
    pub username: String,
    pub realname: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub account_name: Option<String>,
    pub is_operator: bool,
    pub is_services: bool,
    pub modes: String,
    pub away_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub vhost: Option<String>,
    pub metadata: serde_json::Value,
}

impl User {
    pub fn new(nickname: String, username: String, realname: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            nickname,
            username,
            realname,
            password_hash,
            email: None,
            account_name: None,
            is_operator: false,
            is_services: false,
            modes: String::new(),
            away_message: None,
            created_at: Utc::now(),
            last_seen: Utc::now(),
            vhost: None,
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Channel {
    pub name: String,
    pub topic: Option<String>,
    pub topic_set_by: Option<String>,
    pub topic_set_at: Option<DateTime<Utc>>,
    pub modes: String,
    pub key: Option<String>,
    pub limit: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub founder: Option<String>,
    pub successor: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
    pub entry_message: Option<String>,
    pub metadata: serde_json::Value,
}

impl Channel {
    pub fn new(name: String, founder: Option<String>) -> Self {
        Self {
            name,
            topic: None,
            topic_set_by: None,
            topic_set_at: None,
            modes: String::new(),
            key: None,
            limit: None,
            created_at: Utc::now(),
            founder,
            successor: None,
            description: None,
            url: None,
            email: None,
            entry_message: None,
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChannelMember {
    pub channel_name: String,
    pub user_id: String,
    pub modes: String,
    pub joined_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BanEntry {
    pub channel_name: String,
    pub mask: String,
    pub set_by: String,
    pub set_at: DateTime<Utc>,
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerConfig {
    pub id: i32,
    pub server_name: String,
    pub network_name: String,
    pub server_description: String,
    pub admin_name: String,
    pub admin_email: String,
    pub motd: Option<String>,
    pub max_clients: i32,
    pub max_channels_per_user: i32,
    pub max_nickname_length: i32,
    pub max_channel_name_length: i32,
    pub max_topic_length: i32,
    pub max_kick_reason_length: i32,
    pub max_away_length: i32,
    pub max_message_length: i32,
    pub default_modes: String,
    pub default_channel_modes: String,
    pub ping_frequency: i32,
    pub ping_timeout: i32,
    pub flood_messages: i32,
    pub flood_interval: i32,
    pub throttle_duration: i32,
    pub metadata: serde_json::Value,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            id: 1,
            server_name: "ironchatd.local".to_string(),
            network_name: "IronChat".to_string(),
            server_description: "IronChat IRC Server".to_string(),
            admin_name: "Admin".to_string(),
            admin_email: "admin@example.com".to_string(),
            motd: None,
            max_clients: 10000,
            max_channels_per_user: 50,
            max_nickname_length: 30,
            max_channel_name_length: 50,
            max_topic_length: 390,
            max_kick_reason_length: 255,
            max_away_length: 255,
            max_message_length: 512,
            default_modes: String::new(),
            default_channel_modes: "nt".to_string(),
            ping_frequency: 120,
            ping_timeout: 60,
            flood_messages: 10,
            flood_interval: 1,
            throttle_duration: 60,
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub sender_id: String,
    pub target: String,
    pub message_type: String,
    pub content: String,
    pub tags: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OperatorCredential {
    pub id: String,
    pub name: String,
    pub password_hash: String,
    pub host_mask: Option<String>,
    pub privileges: String,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}