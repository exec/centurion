use dashmap::DashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ChannelMember {
    pub connection_id: u64,
    pub modes: Vec<char>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub topic: Option<String>,
    pub topic_set_by: Option<String>,
    pub topic_set_at: Option<DateTime<Utc>>,
    pub modes: Vec<char>,
    pub members: DashMap<u64, ChannelMember>,
    pub created_at: DateTime<Utc>,
    pub key: Option<String>,
    pub limit: Option<usize>,
}

impl Channel {
    pub fn new(name: String) -> Self {
        Self {
            name,
            topic: None,
            topic_set_by: None,
            topic_set_at: None,
            modes: Vec::new(),
            members: DashMap::new(),
            created_at: Utc::now(),
            key: None,
            limit: None,
        }
    }

    pub fn add_member(&self, connection_id: u64, is_operator: bool) {
        let mut modes = Vec::new();
        if is_operator {
            modes.push('o');
        }
        
        let member = ChannelMember {
            connection_id,
            modes,
            joined_at: Utc::now(),
        };
        
        self.members.insert(connection_id, member);
    }

    pub fn remove_member(&self, connection_id: u64) -> bool {
        self.members.remove(&connection_id).is_some()
    }

    pub fn is_member(&self, connection_id: u64) -> bool {
        self.members.contains_key(&connection_id)
    }

    pub fn is_operator(&self, connection_id: u64) -> bool {
        self.members
            .get(&connection_id)
            .map(|member| member.modes.contains(&'o'))
            .unwrap_or(false)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}