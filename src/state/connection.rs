use std::net::SocketAddr;
use tokio::sync::mpsc;
use chrono::{DateTime, Utc};

use crate::protocol::Message;

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: u64,
    pub addr: SocketAddr,
    pub nickname: Option<String>,
    pub username: Option<String>,
    pub realname: Option<String>,
    pub hostname: String,
    pub registered: bool,
    pub capabilities: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub tx: mpsc::Sender<Message>,
}

impl Connection {
    pub fn new(id: u64, addr: SocketAddr, tx: mpsc::Sender<Message>) -> Self {
        let now = Utc::now();
        Self {
            id,
            addr,
            nickname: None,
            username: None,
            realname: None,
            hostname: addr.ip().to_string(),
            registered: false,
            capabilities: Vec::new(),
            created_at: now,
            last_activity: now,
            tx,
        }
    }

    pub fn is_registered(&self) -> bool {
        self.registered && self.nickname.is_some() && self.username.is_some()
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    pub fn full_mask(&self) -> String {
        match &self.nickname {
            Some(nick) => format!("{}!{}@{}", nick, self.username.as_deref().unwrap_or("*"), self.hostname),
            None => format!("*!*@{}", self.hostname),
        }
    }
}