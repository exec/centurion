use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MessageExtensions {
    pub msgid: Option<String>,
    pub time: Option<DateTime<Utc>>,
    pub account: Option<String>,
    pub label: Option<String>,
    pub batch: Option<String>,
    pub reply: Option<String>,
    pub react: Option<String>,
    pub typing: Option<TypingState>,
    pub custom: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypingState {
    Active,
    Paused,
    Done,
}

impl MessageExtensions {
    pub fn new() -> Self {
        Self {
            msgid: None,
            time: None,
            account: None,
            label: None,
            batch: None,
            reply: None,
            react: None,
            typing: None,
            custom: HashMap::new(),
        }
    }
    
    pub fn from_tags(tags: &HashMap<String, Option<String>>) -> Self {
        let mut ext = Self::new();
        
        for (key, value) in tags {
            match key.as_str() {
                "msgid" => ext.msgid = value.clone(),
                "time" => {
                    if let Some(time_str) = value {
                        if let Ok(time) = DateTime::parse_from_rfc3339(time_str) {
                            ext.time = Some(time.with_timezone(&Utc));
                        }
                    }
                }
                "account" => ext.account = value.clone(),
                "label" => ext.label = value.clone(),
                "batch" => ext.batch = value.clone(),
                "+draft/reply" => ext.reply = value.clone(),
                "+draft/react" => ext.react = value.clone(),
                "+typing" => {
                    ext.typing = value.as_ref().and_then(|v| match v.as_str() {
                        "active" => Some(TypingState::Active),
                        "paused" => Some(TypingState::Paused),
                        "done" => Some(TypingState::Done),
                        _ => None,
                    });
                }
                _ => {
                    ext.custom.insert(key.clone(), value.clone());
                }
            }
        }
        
        ext
    }
    
    pub fn to_tags(&self) -> HashMap<String, Option<String>> {
        let mut tags = HashMap::new();
        
        if let Some(msgid) = &self.msgid {
            tags.insert("msgid".to_string(), Some(msgid.clone()));
        }
        
        if let Some(time) = &self.time {
            tags.insert("time".to_string(), Some(time.to_rfc3339()));
        }
        
        if let Some(account) = &self.account {
            tags.insert("account".to_string(), Some(account.clone()));
        }
        
        if let Some(label) = &self.label {
            tags.insert("label".to_string(), Some(label.clone()));
        }
        
        if let Some(batch) = &self.batch {
            tags.insert("batch".to_string(), Some(batch.clone()));
        }
        
        if let Some(reply) = &self.reply {
            tags.insert("+draft/reply".to_string(), Some(reply.clone()));
        }
        
        if let Some(react) = &self.react {
            tags.insert("+draft/react".to_string(), Some(react.clone()));
        }
        
        if let Some(typing) = &self.typing {
            let state = match typing {
                TypingState::Active => "active",
                TypingState::Paused => "paused",
                TypingState::Done => "done",
            };
            tags.insert("+typing".to_string(), Some(state.to_string()));
        }
        
        tags.extend(self.custom.clone());
        
        tags
    }
}

impl Default for MessageExtensions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Batch {
    pub reference: String,
    pub batch_type: String,
    pub params: Vec<String>,
    pub messages: Vec<crate::protocol::Message>,
    pub parent_batch: Option<String>,
}

impl Batch {
    pub fn new(reference: String, batch_type: String, params: Vec<String>) -> Self {
        Self {
            reference,
            batch_type,
            params,
            messages: Vec::new(),
            parent_batch: None,
        }
    }
    
    pub fn add_message(&mut self, msg: crate::protocol::Message) {
        self.messages.push(msg);
    }
}