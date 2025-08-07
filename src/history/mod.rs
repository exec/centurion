use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::protocol::Message;

pub mod storage;
pub mod queries;

pub use storage::{HistoryStorage, HistoryConfig};
pub use queries::{HistoryQuery, QueryResult};

/// Types of messages that can be stored in history
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Privmsg,
    Notice,
    Join,
    Part,
    Quit,
    Kick,
    Mode,
    Tagmsg,
    Nick,
    Topic,
    Invite,
}

/// A single history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    /// Message ID for reference
    pub msgid: String,
    /// Server-generated timestamp
    pub timestamp: SystemTime,
    /// Message type
    pub message_type: MessageType,
    /// Sender nickname
    pub nick: String,
    /// Account name if authenticated, "*" if not
    pub account: String,
    /// Message content or event details
    pub content: String,
    /// Additional parameters (e.g., for MODE, KICK)
    pub params: Vec<String>,
    /// Message tags
    pub tags: BTreeMap<String, Option<String>>,
    /// Target (channel or user)
    pub target: String,
    /// For DMs, the correspondent's nickname
    pub correspondent: Option<String>,
    /// Whether sender is a bot
    pub is_bot: bool,
}

impl HistoryItem {
    pub fn new(
        msgid: String,
        message_type: MessageType,
        nick: String,
        account: String,
        content: String,
        target: String,
    ) -> Self {
        Self {
            msgid,
            timestamp: SystemTime::now(),
            message_type,
            nick,
            account,
            content,
            params: Vec::new(),
            tags: BTreeMap::new(),
            target,
            correspondent: None,
            is_bot: false,
        }
    }

    /// Check if this item has a specific message ID
    pub fn has_msgid(&self, msgid: &str) -> bool {
        self.msgid == msgid
    }

    /// Get timestamp in seconds since epoch
    pub fn timestamp_secs(&self) -> u64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Convert to IRC message for replay
    pub fn to_irc_message(&self, server_name: &str) -> Message {
        let prefix = format!("{}!{}@{}", self.nick, self.account, server_name);
        
        let mut msg = match self.message_type {
            MessageType::Privmsg => Message::new("PRIVMSG")
                .with_prefix(prefix)
                .with_params(vec![self.target.clone(), self.content.clone()]),
            MessageType::Notice => Message::new("NOTICE")
                .with_prefix(prefix)
                .with_params(vec![self.target.clone(), self.content.clone()]),
            MessageType::Tagmsg => Message::new("TAGMSG")
                .with_prefix(prefix)
                .with_params(vec![self.target.clone()]),
            MessageType::Join => Message::new("JOIN")
                .with_prefix(prefix)
                .with_params(vec![self.target.clone()]),
            MessageType::Part => {
                let mut params = vec![self.target.clone()];
                if !self.content.is_empty() {
                    params.push(self.content.clone());
                }
                Message::new("PART").with_prefix(prefix).with_params(params)
            },
            MessageType::Quit => Message::new("QUIT")
                .with_prefix(prefix)
                .with_params(vec![self.content.clone()]),
            MessageType::Kick => {
                let mut params = vec![self.target.clone()];
                params.extend(self.params.clone());
                if !self.content.is_empty() {
                    params.push(self.content.clone());
                }
                Message::new("KICK").with_prefix(prefix).with_params(params)
            },
            MessageType::Mode => {
                let mut params = vec![self.target.clone(), self.content.clone()];
                params.extend(self.params.clone());
                Message::new("MODE").with_prefix(prefix).with_params(params)
            },
            MessageType::Nick => Message::new("NICK")
                .with_prefix(prefix)
                .with_params(vec![self.content.clone()]),
            MessageType::Topic => Message::new("TOPIC")
                .with_prefix(prefix)
                .with_params(vec![self.target.clone(), self.content.clone()]),
            MessageType::Invite => Message::new("INVITE")
                .with_prefix(prefix)
                .with_params(vec![self.params[0].clone(), self.target.clone()]),
        };

        // Add stored tags
        for (key, value) in &self.tags {
            msg = msg.with_tag(key.clone(), value.clone());
        }

        // Add server-time and msgid tags
        msg = msg.with_tag("time".to_string(), Some(format_timestamp(self.timestamp)));
        msg = msg.with_tag("msgid".to_string(), Some(self.msgid.clone()));

        msg
    }
}

/// Format timestamp for IRC server-time tag
pub fn format_timestamp(timestamp: SystemTime) -> String {
    let since_epoch = timestamp.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since_epoch.as_secs();
    let nanos = since_epoch.subsec_nanos();
    
    // Convert to RFC3339 format with milliseconds
    let datetime = chrono::DateTime::from_timestamp(secs as i64, nanos).unwrap();
    datetime.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
}

/// Parse timestamp from IRC format
pub fn parse_timestamp(timestamp_str: &str) -> Result<SystemTime, Box<dyn std::error::Error>> {
    let timestamp_str = timestamp_str.trim_start_matches("timestamp=");
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp_str)?;
    let system_time = SystemTime::UNIX_EPOCH + 
        std::time::Duration::from_secs(parsed.timestamp() as u64) +
        std::time::Duration::from_nanos(parsed.timestamp_subsec_nanos() as u64);
    Ok(system_time)
}