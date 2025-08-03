use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;

use crate::protocol::{Message, Reply};

#[derive(Error, Debug)]
pub enum RedactionError {
    #[error("Invalid target")]
    InvalidTarget,
    
    #[error("Redaction forbidden")]
    RedactionForbidden,
    
    #[error("Redaction window expired")]
    RedactionWindowExpired,
    
    #[error("Unknown message ID")]
    UnknownMsgId,
    
    #[error("Message not redactable")]
    MessageNotRedactable,
}

#[derive(Debug, Clone)]
pub struct RedactableMessage {
    pub msgid: String,
    pub sender_id: u64,
    pub target: String,
    pub message_type: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub redacted: bool,
    pub redaction_reason: Option<String>,
    pub redacted_by: Option<u64>,
    pub redacted_at: Option<DateTime<Utc>>,
}

impl RedactableMessage {
    pub fn new(
        msgid: String,
        sender_id: u64,
        target: String,
        message_type: String,
        content: String,
    ) -> Self {
        Self {
            msgid,
            sender_id,
            target,
            message_type,
            content,
            timestamp: Utc::now(),
            redacted: false,
            redaction_reason: None,
            redacted_by: None,
            redacted_at: None,
        }
    }
    
    pub fn is_redactable(&self) -> bool {
        // Only PRIVMSG, NOTICE, and TAGMSG are redactable
        matches!(self.message_type.as_str(), "PRIVMSG" | "NOTICE" | "TAGMSG")
    }
    
    pub fn can_be_redacted_by(&self, user_id: u64, is_operator: bool) -> bool {
        if self.redacted {
            return false;
        }
        
        // Original sender can always redact their own messages
        if self.sender_id == user_id {
            return true;
        }
        
        // Channel operators can redact messages in their channels
        if is_operator && self.target.starts_with('#') {
            return true;
        }
        
        false
    }
    
    pub fn is_within_redaction_window(&self, window_seconds: u64) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.timestamp);
        elapsed.num_seconds() <= window_seconds as i64
    }
    
    pub fn redact(&mut self, redacted_by: u64, reason: Option<String>) -> bool {
        if self.redacted {
            return false;
        }
        
        self.redacted = true;
        self.redaction_reason = reason;
        self.redacted_by = Some(redacted_by);
        self.redacted_at = Some(Utc::now());
        
        true
    }
}

pub struct RedactionManager {
    messages: HashMap<String, RedactableMessage>,
    redaction_window_seconds: u64,
    max_stored_messages: usize,
}

impl RedactionManager {
    pub fn new(redaction_window_seconds: u64, max_stored_messages: usize) -> Self {
        Self {
            messages: HashMap::new(),
            redaction_window_seconds,
            max_stored_messages,
        }
    }
    
    pub fn store_message(&mut self, message: RedactableMessage) {
        // Clean up old messages if we're at capacity
        if self.messages.len() >= self.max_stored_messages {
            self.cleanup_old_messages();
        }
        
        self.messages.insert(message.msgid.clone(), message);
    }
    
    pub fn redact_message(
        &mut self,
        msgid: &str,
        redacted_by: u64,
        reason: Option<String>,
        is_operator: bool,
    ) -> Result<&RedactableMessage, RedactionError> {
        let message = self.messages.get_mut(msgid)
            .ok_or(RedactionError::UnknownMsgId)?;
        
        if !message.is_redactable() {
            return Err(RedactionError::MessageNotRedactable);
        }
        
        if !message.can_be_redacted_by(redacted_by, is_operator) {
            return Err(RedactionError::RedactionForbidden);
        }
        
        if !message.is_within_redaction_window(self.redaction_window_seconds) {
            return Err(RedactionError::RedactionWindowExpired);
        }
        
        if !message.redact(redacted_by, reason) {
            return Err(RedactionError::RedactionForbidden);
        }
        
        Ok(message)
    }
    
    pub fn get_message(&self, msgid: &str) -> Option<&RedactableMessage> {
        self.messages.get(msgid)
    }
    
    pub fn is_message_redacted(&self, msgid: &str) -> bool {
        self.messages.get(msgid)
            .map(|msg| msg.redacted)
            .unwrap_or(false)
    }
    
    fn cleanup_old_messages(&mut self) {
        let now = Utc::now();
        let cutoff_duration = chrono::Duration::seconds(self.redaction_window_seconds as i64 * 2);
        
        self.messages.retain(|_, msg| {
            let age = now.signed_duration_since(msg.timestamp);
            age < cutoff_duration
        });
    }
    
    pub fn create_redaction_message(
        target: String,
        msgid: String,
        reason: Option<String>,
        redacted_by_mask: String,
    ) -> Message {
        let mut msg = Message::new("REDACT")
            .with_prefix(redacted_by_mask)
            .add_param(target)
            .add_param(msgid);
        
        if let Some(reason_text) = reason {
            msg = msg.add_param(reason_text);
        }
        
        msg
    }
}

pub fn validate_redact_command(params: &[String]) -> Result<(String, String, Option<String>), RedactionError> {
    if params.len() < 2 {
        return Err(RedactionError::InvalidTarget);
    }
    
    let target = params[0].clone();
    let msgid = params[1].clone();
    let reason = params.get(2).cloned();
    
    // Validate target format
    if target.is_empty() || (!target.starts_with('#') && !target.starts_with('&') && target.contains(' ')) {
        return Err(RedactionError::InvalidTarget);
    }
    
    // Validate msgid format (should be non-empty and reasonable length)
    if msgid.is_empty() || msgid.len() > 100 {
        return Err(RedactionError::UnknownMsgId);
    }
    
    Ok((target, msgid, reason))
}

impl From<RedactionError> for Reply {
    fn from(error: RedactionError) -> Self {
        match error {
            RedactionError::InvalidTarget => Reply::None { nick: "*".to_string() }, // Will be replaced with FAIL
            RedactionError::RedactionForbidden => Reply::None { nick: "*".to_string() },
            RedactionError::RedactionWindowExpired => Reply::None { nick: "*".to_string() },
            RedactionError::UnknownMsgId => Reply::None { nick: "*".to_string() },
            RedactionError::MessageNotRedactable => Reply::None { nick: "*".to_string() },
        }
    }
}

pub fn create_redaction_fail_message(
    nick: String,
    error: RedactionError,
    context: &str,
) -> Message {
    let error_code = match error {
        RedactionError::InvalidTarget => "INVALID_TARGET",
        RedactionError::RedactionForbidden => "REDACT_FORBIDDEN", 
        RedactionError::RedactionWindowExpired => "REDACT_WINDOW_EXPIRED",
        RedactionError::UnknownMsgId => "UNKNOWN_MSGID",
        RedactionError::MessageNotRedactable => "REDACT_FORBIDDEN",
    };
    
    Message::new("FAIL")
        .add_param("REDACT")
        .add_param(error_code)
        .add_param(context.to_string())
        .add_param(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_redactable_message_creation() {
        let msg = RedactableMessage::new(
            "test123".to_string(),
            1,
            "#channel".to_string(),
            "PRIVMSG".to_string(),
            "Hello world".to_string(),
        );
        
        assert_eq!(msg.msgid, "test123");
        assert_eq!(msg.sender_id, 1);
        assert!(!msg.redacted);
        assert!(msg.is_redactable());
    }
    
    #[test]
    fn test_redaction_permissions() {
        let msg = RedactableMessage::new(
            "test123".to_string(),
            1,
            "#channel".to_string(),
            "PRIVMSG".to_string(),
            "Hello world".to_string(),
        );
        
        // Sender can redact their own message
        assert!(msg.can_be_redacted_by(1, false));
        
        // Other users cannot redact without being operators
        assert!(!msg.can_be_redacted_by(2, false));
        
        // Operators can redact in channels
        assert!(msg.can_be_redacted_by(2, true));
    }
    
    #[test]
    fn test_redaction_window() {
        let mut msg = RedactableMessage::new(
            "test123".to_string(),
            1,
            "#channel".to_string(),
            "PRIVMSG".to_string(),
            "Hello world".to_string(),
        );
        
        // Should be within window initially
        assert!(msg.is_within_redaction_window(3600));
        
        // Simulate old message
        msg.timestamp = Utc::now() - chrono::Duration::hours(2);
        assert!(!msg.is_within_redaction_window(3600));
    }
}