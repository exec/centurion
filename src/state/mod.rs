use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod channel;
pub mod connection;

pub use self::channel::{Channel, ChannelMember};
pub use self::connection::Connection;

use crate::history::{HistoryStorage};

pub struct ServerState {
    pub connections: DashMap<u64, Connection>,
    pub channels: DashMap<String, Channel>,
    pub nicknames: DashMap<String, u64>,
    next_connection_id: AtomicU64,
    pub server_name: String,
    pub history: HistoryStorage,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            channels: DashMap::new(),
            nicknames: DashMap::new(),
            next_connection_id: AtomicU64::new(1),
            server_name: "ironchatd.local".to_string(),
            history: HistoryStorage::default(),
        }
    }

    pub fn generate_connection_id(&self) -> u64 {
        self.next_connection_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn is_nickname_available(&self, nickname: &str) -> bool {
        !self.nicknames.contains_key(&nickname.to_lowercase())
    }

    pub fn register_nickname(&self, nickname: String, connection_id: u64) -> bool {
        let key = nickname.to_lowercase();
        match self.nicknames.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(_) => false,
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(connection_id);
                true
            }
        }
    }

    pub fn unregister_nickname(&self, nickname: &str) {
        self.nicknames.remove(&nickname.to_lowercase());
    }
}