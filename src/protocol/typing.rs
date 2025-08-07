use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

use crate::protocol::Message;

#[derive(Debug, Clone, PartialEq)]
pub enum TypingState {
    Active,
    Paused,
    Done,
}

impl TypingState {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(TypingState::Active),
            "paused" => Some(TypingState::Paused),
            "done" => Some(TypingState::Done),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            TypingState::Active => "active",
            TypingState::Paused => "paused",
            TypingState::Done => "done",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypingStatus {
    pub user_id: u64,
    pub target: String,
    pub state: TypingState,
    pub timestamp: DateTime<Utc>,
}

impl TypingStatus {
    pub fn new(user_id: u64, target: String, state: TypingState) -> Self {
        Self {
            user_id,
            target,
            state,
            timestamp: Utc::now(),
        }
    }
    
    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.timestamp);
        
        match self.state {
            TypingState::Active => elapsed.num_seconds() > 6, // 6 seconds for active
            TypingState::Paused => elapsed.num_seconds() > 30, // 30 seconds for paused
            TypingState::Done => true, // Done is always expired
        }
    }
    
    pub fn should_expire_soon(&self) -> bool {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.timestamp);
        
        match self.state {
            TypingState::Active => elapsed.num_seconds() > 3, // Warn after 3 seconds
            TypingState::Paused => elapsed.num_seconds() > 25, // Warn after 25 seconds
            TypingState::Done => true,
        }
    }
}

pub struct TypingManager {
    // Key: (user_id, target) -> TypingStatus
    typing_status: HashMap<(u64, String), TypingStatus>,
    last_notification: HashMap<(u64, String), DateTime<Utc>>,
    throttle_duration: Duration,
}

impl TypingManager {
    pub fn new() -> Self {
        Self {
            typing_status: HashMap::new(),
            last_notification: HashMap::new(),
            throttle_duration: Duration::from_secs(3), // 3-second throttle
        }
    }
    
    pub fn update_typing_status(
        &mut self,
        user_id: u64,
        target: String,
        state: TypingState,
    ) -> Result<bool, TypingError> {
        let key = (user_id, target.clone());
        let now = Utc::now();
        
        // Check throttling
        if let Some(last_time) = self.last_notification.get(&key) {
            let elapsed = now.signed_duration_since(*last_time);
            if elapsed < chrono::Duration::from_std(self.throttle_duration).unwrap() {
                return Err(TypingError::Throttled);
            }
        }
        
        // Update status
        let status = TypingStatus::new(user_id, target, state);
        let should_broadcast = match status.state {
            TypingState::Done => {
                // Remove the status for "done"
                self.typing_status.remove(&key);
                true
            }
            _ => {
                self.typing_status.insert(key.clone(), status);
                true
            }
        };
        
        if should_broadcast {
            self.last_notification.insert(key, now);
        }
        
        Ok(should_broadcast)
    }
    
    pub fn get_typing_status(&self, user_id: u64, target: &str) -> Option<&TypingStatus> {
        self.typing_status.get(&(user_id, target.to_string()))
    }
    
    pub fn clear_user_typing(&mut self, user_id: u64) {
        self.typing_status.retain(|(uid, _), _| *uid != user_id);
        self.last_notification.retain(|(uid, _), _| *uid != user_id);
    }
    
    pub fn clear_target_typing(&mut self, target: &str) {
        self.typing_status.retain(|(_, tgt), _| tgt != target);
        self.last_notification.retain(|(_, tgt), _| tgt != target);
    }
    
    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        
        // Remove expired typing statuses
        self.typing_status.retain(|key, status| {
            if status.is_expired(0) {
                // Also remove from last_notification
                self.last_notification.remove(key);
                false
            } else {
                true
            }
        });
    }
    
    pub fn get_typing_users_for_target(&self, target: &str) -> Vec<u64> {
        self.typing_status
            .iter()
            .filter(|((_, tgt), status)| {
                tgt == target && !status.is_expired(0) && status.state != TypingState::Done
            })
            .map(|((uid, _), _)| *uid)
            .collect()
    }
    
    pub fn on_message_sent(&mut self, user_id: u64, target: &str) {
        // Clear typing status when user sends a message
        let key = (user_id, target.to_string());
        self.typing_status.remove(&key);
    }
    
    pub fn on_user_left_channel(&mut self, user_id: u64, channel: &str) {
        // Clear typing status when user leaves channel
        let key = (user_id, channel.to_string());
        self.typing_status.remove(&key);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TypingError {
    #[error("Typing notification throttled")]
    Throttled,
    
    #[error("Invalid typing state")]
    InvalidState,
    
    #[error("Invalid target")]
    InvalidTarget,
    
    #[error("Permission denied")]
    PermissionDenied,
}

pub struct TypingProcessor;

impl TypingProcessor {
    pub fn extract_typing_from_message(message: &Message) -> Option<TypingState> {
        // Check for +typing client tag
        message.tags
            .get("+typing")
            .and_then(|opt| opt.as_ref())
            .and_then(|value| TypingState::from_str(value))
    }
    
    pub fn create_typing_tagmsg(
        target: String,
        state: TypingState,
        sender_mask: String,
    ) -> Message {
        Message::new("TAGMSG")
            .with_prefix(sender_mask)
            .with_params(vec![target])
            .add_tag("+typing".to_string(), Some(state.as_str().to_string()))
    }
    
    pub fn should_send_typing_to_target(
        target: &str,
        sender_id: u64,
        server_state: &crate::state::ServerState,
    ) -> bool {
        // For channels, sender must be a member
        if target.starts_with('#') || target.starts_with('&') {
            if let Some(channel) = server_state.channels.get(target) {
                return channel.is_member(sender_id);
            }
            return false;
        }
        
        // For private messages, target must be a valid user
        server_state.nicknames.contains_key(&target.to_lowercase())
    }
    
    pub fn validate_typing_message(message: &Message) -> Result<(), TypingError> {
        // Must be PRIVMSG, NOTICE, TAGMSG, or batch end
        if !matches!(message.command.as_str(), "PRIVMSG" | "NOTICE" | "TAGMSG") {
            return Err(TypingError::InvalidState);
        }
        
        // Must have target parameter
        if message.params.is_empty() {
            return Err(TypingError::InvalidTarget);
        }
        
        // Check if +typing tag is present and valid
        if let Some(typing_value) = message.tags.get("+typing") {
            if let Some(value) = typing_value {
                if TypingState::from_str(value).is_none() {
                    return Err(TypingError::InvalidState);
                }
            }
        }
        
        Ok(())
    }
    
    pub fn filter_slash_commands(content: &str) -> bool {
        // Don't send typing notifications for slash commands
        !content.trim_start().starts_with('/')
    }
    
    pub fn should_throttle_typing(
        last_typing: Option<DateTime<Utc>>,
        min_interval_seconds: u64,
    ) -> bool {
        if let Some(last_time) = last_typing {
            let now = Utc::now();
            let elapsed = now.signed_duration_since(last_time);
            elapsed.num_seconds() < min_interval_seconds as i64
        } else {
            false
        }
    }
}

impl Default for TypingManager {
    fn default() -> Self {
        Self::new()
    }
}

// Privacy and throttling helpers
pub struct TypingPrivacyControls {
    pub suppress_own_typing: bool,
    pub allow_typing_in_channels: bool,
    pub allow_typing_in_private: bool,
    pub typing_timeout_seconds: u64,
}

impl Default for TypingPrivacyControls {
    fn default() -> Self {
        Self {
            suppress_own_typing: false,
            allow_typing_in_channels: true,
            allow_typing_in_private: true,
            typing_timeout_seconds: 30,
        }
    }
}

impl TypingPrivacyControls {
    pub fn should_send_typing(&self, target: &str, is_channel: bool) -> bool {
        if is_channel {
            self.allow_typing_in_channels
        } else {
            self.allow_typing_in_private
        }
    }
    
    pub fn should_suppress_own(&self) -> bool {
        self.suppress_own_typing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_typing_state_parsing() {
        assert_eq!(TypingState::from_str("active"), Some(TypingState::Active));
        assert_eq!(TypingState::from_str("paused"), Some(TypingState::Paused));
        assert_eq!(TypingState::from_str("done"), Some(TypingState::Done));
        assert_eq!(TypingState::from_str("invalid"), None);
    }
    
    #[test]
    fn test_typing_status_expiration() {
        let mut status = TypingStatus::new(1, "#channel".to_string(), TypingState::Active);
        
        // Should not be expired immediately
        assert!(!status.is_expired(0));
        
        // Simulate old timestamp
        status.timestamp = Utc::now() - chrono::Duration::seconds(10);
        assert!(status.is_expired(0));
    }
    
    #[test]
    fn test_typing_manager_throttling() {
        let mut manager = TypingManager::new();
        
        // First update should succeed
        let result = manager.update_typing_status(
            1,
            "#channel".to_string(),
            TypingState::Active,
        );
        assert!(result.is_ok());
        assert!(result.unwrap());
        
        // Immediate second update should be throttled
        let result = manager.update_typing_status(
            1,
            "#channel".to_string(),
            TypingState::Active,
        );
        assert!(matches!(result, Err(TypingError::Throttled)));
    }
    
    #[test]
    fn test_typing_message_extraction() {
        let mut msg = Message::new("TAGMSG")
            .with_params(vec!["#channel".to_string()])
            .add_tag("+typing".to_string(), Some("active".to_string()));
        
        let typing_state = TypingProcessor::extract_typing_from_message(&msg);
        assert_eq!(typing_state, Some(TypingState::Active));
        
        // Test with no typing tag
        msg.tags.clear();
        let typing_state = TypingProcessor::extract_typing_from_message(&msg);
        assert_eq!(typing_state, None);
    }
    
    #[test]
    fn test_slash_command_filtering() {
        assert!(!TypingProcessor::filter_slash_commands("/help"));
        assert!(!TypingProcessor::filter_slash_commands("  /msg user hello"));
        assert!(TypingProcessor::filter_slash_commands("normal message"));
        assert!(TypingProcessor::filter_slash_commands("not a /command"));
    }
}