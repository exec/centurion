use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;

use crate::protocol::{Message, Reply};

#[derive(Error, Debug)]
pub enum ReadMarkerError {
    #[error("Need more parameters")]
    NeedMoreParams,
    
    #[error("Invalid parameters")]
    InvalidParams,
    
    #[error("Internal error")]
    InternalError,
    
    #[error("Target not found")]
    TargetNotFound,
    
    #[error("Permission denied")]
    PermissionDenied,
}

#[derive(Debug, Clone)]
pub struct ReadMarker {
    pub user_id: u64,
    pub target: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl ReadMarker {
    pub fn new(user_id: u64, target: String, timestamp: Option<DateTime<Utc>>) -> Self {
        Self {
            user_id,
            target,
            timestamp,
            updated_at: Utc::now(),
        }
    }
    
    pub fn update_timestamp(&mut self, timestamp: Option<DateTime<Utc>>) -> bool {
        // Timestamps can only increase (or be set to None)
        match (&self.timestamp, &timestamp) {
            (None, _) => {
                self.timestamp = timestamp;
                self.updated_at = Utc::now();
                true
            }
            (Some(_), None) => false, // Can't go from Some to None
            (Some(current), Some(new)) => {
                if new >= current {
                    self.timestamp = timestamp;
                    self.updated_at = Utc::now();
                    true
                } else {
                    false
                }
            }
        }
    }
    
    pub fn timestamp_string(&self) -> String {
        match &self.timestamp {
            Some(ts) => ts.to_rfc3339(),
            None => "*".to_string(),
        }
    }
}

pub struct ReadMarkerManager {
    // Key: (user_id, target) -> ReadMarker
    markers: HashMap<(u64, String), ReadMarker>,
}

impl ReadMarkerManager {
    pub fn new() -> Self {
        Self {
            markers: HashMap::new(),
        }
    }
    
    pub fn set_read_marker(
        &mut self,
        user_id: u64,
        target: String,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<DateTime<Utc>, ReadMarkerError> {
        let key = (user_id, target.clone());
        
        match self.markers.get_mut(&key) {
            Some(marker) => {
                if marker.update_timestamp(timestamp) {
                    Ok(marker.updated_at)
                } else {
                    Err(ReadMarkerError::InvalidParams)
                }
            }
            None => {
                let marker = ReadMarker::new(user_id, target, timestamp);
                let server_timestamp = marker.updated_at;
                self.markers.insert(key, marker);
                Ok(server_timestamp)
            }
        }
    }
    
    pub fn get_read_marker(&self, user_id: u64, target: &str) -> Option<&ReadMarker> {
        self.markers.get(&(user_id, target.to_string()))
    }
    
    pub fn remove_user_markers(&mut self, user_id: u64) {
        self.markers.retain(|(uid, _), _| *uid != user_id);
    }
    
    pub fn remove_target_markers(&mut self, target: &str) {
        self.markers.retain(|(_, tgt), _| tgt != target);
    }
    
    pub fn get_user_markers(&self, user_id: u64) -> Vec<&ReadMarker> {
        self.markers
            .iter()
            .filter(|((uid, _), _)| *uid == user_id)
            .map(|(_, marker)| marker)
            .collect()
    }
}

pub struct ReadMarkerProcessor;

impl ReadMarkerProcessor {
    pub fn parse_markread_command(params: &[String]) -> Result<MarkReadCommand, ReadMarkerError> {
        if params.is_empty() {
            return Err(ReadMarkerError::NeedMoreParams);
        }
        
        let target = params[0].clone();
        
        if target.is_empty() {
            return Err(ReadMarkerError::InvalidParams);
        }
        
        match params.len() {
            1 => {
                // GET command: MARKREAD <target>
                Ok(MarkReadCommand::Get { target })
            }
            2 => {
                // SET command: MARKREAD <target> <timestamp>
                let timestamp_str = &params[1];
                
                if timestamp_str == "*" {
                    Ok(MarkReadCommand::Set { 
                        target, 
                        timestamp: None 
                    })
                } else {
                    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                        .map_err(|_| ReadMarkerError::InvalidParams)?
                        .with_timezone(&Utc);
                    
                    Ok(MarkReadCommand::Set { 
                        target, 
                        timestamp: Some(timestamp) 
                    })
                }
            }
            _ => Err(ReadMarkerError::InvalidParams),
        }
    }
    
    pub fn create_markread_response(
        target: String,
        timestamp: Option<DateTime<Utc>>,
        server_name: &str,
    ) -> Message {
        let timestamp_str = match timestamp {
            Some(ts) => ts.to_rfc3339(),
            None => "*".to_string(),
        };
        
        Message::new("MARKREAD")
            .with_prefix(server_name)
            .add_param(target)
            .add_param(timestamp_str)
    }
    
    pub fn create_markread_fail(
        error: ReadMarkerError,
        context: &str,
    ) -> Message {
        let error_code = match error {
            ReadMarkerError::NeedMoreParams => "NEED_MORE_PARAMS",
            ReadMarkerError::InvalidParams => "INVALID_PARAMS",
            ReadMarkerError::InternalError => "INTERNAL_ERROR",
            ReadMarkerError::TargetNotFound => "INVALID_PARAMS",
            ReadMarkerError::PermissionDenied => "INVALID_PARAMS",
        };
        
        Message::new("FAIL")
            .add_param("MARKREAD")
            .add_param(error_code)
            .add_param(context.to_string())
            .add_param(error.to_string())
    }
    
    pub fn validate_target(target: &str, user_id: u64, server_state: &crate::state::ServerState) -> bool {
        // For channels, user must be a member
        if target.starts_with('#') || target.starts_with('&') {
            if let Some(channel) = server_state.channels.get(target) {
                return channel.is_member(user_id);
            }
            return false;
        }
        
        // For private messages, target must be a valid user
        if let Some(_) = server_state.nicknames.get(&target.to_lowercase()) {
            return true;
        }
        
        false
    }
    
    pub fn should_send_on_join() -> bool {
        // Read markers should be automatically sent when a user joins a channel
        true
    }
}

#[derive(Debug, Clone)]
pub enum MarkReadCommand {
    Get { target: String },
    Set { target: String, timestamp: Option<DateTime<Utc>> },
}

pub fn validate_timestamp_format(timestamp: &str) -> Result<DateTime<Utc>, ReadMarkerError> {
    if timestamp == "*" {
        return Err(ReadMarkerError::InvalidParams);
    }
    
    DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| ReadMarkerError::InvalidParams)
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn timestamp_from_message_time_tag(tags: &std::collections::HashMap<String, Option<String>>) -> Option<DateTime<Utc>> {
    tags.get("time")
        .and_then(|opt| opt.as_ref())
        .and_then(|time_str| DateTime::parse_from_rfc3339(time_str).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_markread_command_parsing() {
        // Test GET command
        let params = vec!["#channel".to_string()];
        let cmd = ReadMarkerProcessor::parse_markread_command(&params).unwrap();
        match cmd {
            MarkReadCommand::Get { target } => assert_eq!(target, "#channel"),
            _ => panic!("Expected Get command"),
        }
        
        // Test SET command with timestamp
        let params = vec![
            "#channel".to_string(),
            "2024-01-01T12:00:00Z".to_string(),
        ];
        let cmd = ReadMarkerProcessor::parse_markread_command(&params).unwrap();
        match cmd {
            MarkReadCommand::Set { target, timestamp } => {
                assert_eq!(target, "#channel");
                assert!(timestamp.is_some());
            }
            _ => panic!("Expected Set command"),
        }
        
        // Test SET command with wildcard
        let params = vec!["#channel".to_string(), "*".to_string()];
        let cmd = ReadMarkerProcessor::parse_markread_command(&params).unwrap();
        match cmd {
            MarkReadCommand::Set { target, timestamp } => {
                assert_eq!(target, "#channel");
                assert!(timestamp.is_none());
            }
            _ => panic!("Expected Set command"),
        }
    }
    
    #[test]
    fn test_read_marker_timestamp_updates() {
        let mut marker = ReadMarker::new(
            1, 
            "#channel".to_string(), 
            Some(Utc::now())
        );
        
        let future_time = Utc::now() + chrono::Duration::hours(1);
        let past_time = Utc::now() - chrono::Duration::hours(1);
        
        // Should allow updating to future time
        assert!(marker.update_timestamp(Some(future_time)));
        
        // Should not allow updating to past time
        assert!(!marker.update_timestamp(Some(past_time)));
    }
    
    #[test]
    fn test_read_marker_manager() {
        let mut manager = ReadMarkerManager::new();
        let now = Utc::now();
        
        // Set a read marker
        let server_ts = manager.set_read_marker(
            1, 
            "#channel".to_string(), 
            Some(now)
        ).unwrap();
        
        // Retrieve it
        let marker = manager.get_read_marker(1, "#channel").unwrap();
        assert_eq!(marker.timestamp, Some(now));
        assert_eq!(marker.updated_at, server_ts);
        
        // Update with newer timestamp
        let future = now + chrono::Duration::minutes(5);
        let updated_ts = manager.set_read_marker(
            1, 
            "#channel".to_string(), 
            Some(future)
        ).unwrap();
        
        let updated_marker = manager.get_read_marker(1, "#channel").unwrap();
        assert_eq!(updated_marker.timestamp, Some(future));
        assert!(updated_marker.updated_at > server_ts);
    }
}