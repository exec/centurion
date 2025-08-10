//! Legion channel management and operations
//! 
//! High-level channel operations for Legion Protocol encrypted channels.

use crate::legion::{LegionError, LegionResult, LegionManager};
use legion_protocol::utils::{is_legion_encrypted_channel, ChannelType, get_channel_type};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

/// Channel operation types for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelOperation {
    Create,
    Join,
    Leave,
    SendMessage,
    EditMessage,
    DeleteMessage,
    KeyRotation,
    MemberAdd,
    MemberRemove,
    RoleChange,
    SettingsChange,
}

/// Channel event for logging and federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvent {
    pub channel: String,
    pub operation: ChannelOperation,
    pub actor: String,
    pub target: Option<String>,
    pub timestamp: u64,
    pub metadata: HashMap<String, String>,
}

/// Channel management operations
impl LegionManager {
    /// Validate channel name for Legion Protocol
    pub fn validate_channel_name(channel_name: &str) -> LegionResult<ChannelType> {
        let channel_type = get_channel_type(channel_name);
        
        match channel_type {
            ChannelType::LegionEncrypted => Ok(channel_type),
            ChannelType::IrcGlobal | ChannelType::IrcLocal => {
                Err(LegionError::Channel(format!(
                    "Channel {} is not a Legion encrypted channel", channel_name
                )))
            },
            ChannelType::Invalid => {
                Err(LegionError::Channel(format!(
                    "Invalid channel name: {}", channel_name
                )))
            },
        }
    }
    
    /// Handle channel creation request
    pub async fn handle_create_channel(&self, channel_name: String, creator_id: String) -> LegionResult<()> {
        // Validate channel name
        self::LegionManager::validate_channel_name(&channel_name)?;
        
        // Check if creator supports Legion
        if !self.supports_legion(&creator_id).await {
            return Err(LegionError::Member(format!(
                "Client {} does not support Legion Protocol", creator_id
            )));
        }
        
        // Create the channel
        self.create_channel(channel_name.clone(), creator_id.clone()).await?;
        
        // Log the event
        self.log_channel_event(ChannelEvent {
            channel: channel_name.clone(),
            operation: ChannelOperation::Create,
            actor: creator_id,
            target: None,
            timestamp: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            metadata: HashMap::new(),
        }).await?;
        
        Ok(())
    }
    
    /// Handle channel join request
    pub async fn handle_join_channel(&self, channel_name: String, client_id: String) -> LegionResult<()> {
        // Validate channel name
        self::LegionManager::validate_channel_name(&channel_name)?;
        
        // Check if channel exists
        if !self.channels.contains_key(&channel_name) {
            return Err(LegionError::Channel(format!("Channel does not exist: {}", channel_name)));
        }
        
        // Join the channel
        self.join_channel(channel_name.clone(), client_id.clone()).await?;
        
        // Log the event
        self.log_channel_event(ChannelEvent {
            channel: channel_name.clone(),
            operation: ChannelOperation::Join,
            actor: client_id,
            target: None,
            timestamp: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            metadata: HashMap::new(),
        }).await?;
        
        Ok(())
    }
    
    /// Handle channel leave request
    pub async fn handle_leave_channel(&self, channel_name: String, client_id: String) -> LegionResult<()> {
        // Validate channel name
        self::LegionManager::validate_channel_name(&channel_name)?;
        
        // Leave the channel
        self.leave_channel(channel_name.clone(), client_id.clone()).await?;
        
        // Log the event
        self.log_channel_event(ChannelEvent {
            channel: channel_name.clone(),
            operation: ChannelOperation::Leave,
            actor: client_id,
            target: None,
            timestamp: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            metadata: HashMap::new(),
        }).await?;
        
        Ok(())
    }
    
    /// Handle message send request
    pub async fn handle_send_message(&self, channel_name: String, sender_id: String, message: String) -> LegionResult<Vec<u8>> {
        // Validate channel name
        self::LegionManager::validate_channel_name(&channel_name)?;
        
        // Check permissions
        if !self.member_manager.has_permission(&channel_name, &sender_id, crate::legion::members::Permission::SendMessages).await? {
            return Err(LegionError::Member("Insufficient permissions to send messages".to_string()));
        }
        
        // Send the message
        let encrypted_data = self.send_message(channel_name.clone(), sender_id.clone(), message.clone()).await?;
        
        // Log the event
        let mut metadata = HashMap::new();
        metadata.insert("message_size".to_string(), encrypted_data.len().to_string());
        
        self.log_channel_event(ChannelEvent {
            channel: channel_name,
            operation: ChannelOperation::SendMessage,
            actor: sender_id,
            target: None,
            timestamp: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            metadata,
        }).await?;
        
        Ok(encrypted_data)
    }
    
    /// Handle message receive request
    pub async fn handle_receive_message(&self, channel_name: String, recipient_id: String, encrypted_data: Vec<u8>) -> LegionResult<String> {
        // Validate channel name
        self::LegionManager::validate_channel_name(&channel_name)?;
        
        // Decrypt the message
        let message = self.receive_message(channel_name, recipient_id, encrypted_data).await?;
        
        Ok(message)
    }
    
    /// Handle key rotation request
    pub async fn handle_key_rotation(&self, channel_name: String, admin_id: String) -> LegionResult<()> {
        // Validate channel name
        self::LegionManager::validate_channel_name(&channel_name)?;
        
        // Check permissions
        if !self.member_manager.has_permission(&channel_name, &admin_id, crate::legion::members::Permission::ManageKeys).await? {
            return Err(LegionError::Member("Insufficient permissions to rotate keys".to_string()));
        }
        
        // Rotate keys
        self.rotate_channel_keys(channel_name.clone(), admin_id.clone()).await?;
        
        // Log the event
        self.log_channel_event(ChannelEvent {
            channel: channel_name,
            operation: ChannelOperation::KeyRotation,
            actor: admin_id,
            target: None,
            timestamp: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            metadata: HashMap::new(),
        }).await?;
        
        Ok(())
    }
    
    /// Get comprehensive channel information
    pub async fn get_channel_info(&self, channel_name: &str, requester_id: &str) -> LegionResult<ChannelInfo> {
        // Validate channel name
        self::LegionManager::validate_channel_name(channel_name)?;
        
        // Check if requester is a member
        if !self.member_manager.is_channel_member(channel_name, requester_id).await? {
            return Err(LegionError::Member("Not a member of this channel".to_string()));
        }
        
        // Get channel stats from Phalanx
        let phalanx_stats = self.channel_stats(channel_name).await?;
        
        // Get member stats
        let member_stats = self.member_manager.channel_stats(channel_name).await?;
        
        // Get key stats
        let key_stats = self.key_manager.channel_stats(channel_name).await?;
        
        let created_at = member_stats.created_at;
        
        Ok(ChannelInfo {
            name: channel_name.to_string(),
            phalanx_stats,
            member_stats,
            key_stats,
            created_at,
        })
    }
    
    /// List channels visible to a user
    pub async fn list_user_channels(&self, user_id: &str) -> LegionResult<Vec<ChannelSummary>> {
        let mut summaries = Vec::new();
        
        // Get all channels user is a member of
        let user_channels = self.member_manager.get_user_channels(user_id).await?;
        for channel_name in user_channels {
            if let Ok(info) = self.get_channel_info(&channel_name, user_id).await {
                summaries.push(ChannelSummary {
                    name: info.name,
                    member_count: info.member_stats.total_members,
                    active_members: info.member_stats.active_members,
                    last_activity: info.key_stats.last_rotation,
                    user_role: None, // TODO: Get user's role
                });
            }
        }
        
        Ok(summaries)
    }
    
    /// Log a channel event (placeholder for audit system)
    async fn log_channel_event(&self, event: ChannelEvent) -> LegionResult<()> {
        // In a production system, this would write to an audit log
        tracing::info!("Channel event: {:?}", event);
        Ok(())
    }
}

/// Comprehensive channel information
#[derive(Debug, Clone, Serialize)]
pub struct ChannelInfo {
    pub name: String,
    pub phalanx_stats: phalanx::group::GroupStats,
    pub member_stats: crate::legion::members::ChannelStats,
    pub key_stats: crate::legion::keys::ChannelKeyStats,
    pub created_at: SystemTime,
}

/// Channel summary for listing
#[derive(Debug, Clone, Serialize)]
pub struct ChannelSummary {
    pub name: String,
    pub member_count: usize,
    pub active_members: usize,
    pub last_activity: SystemTime,
    pub user_role: Option<crate::legion::members::MemberRole>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_channel_name_validation() {
        assert!(LegionManager::validate_channel_name("!encrypted").is_ok());
        assert!(LegionManager::validate_channel_name("#regular").is_err());
        assert!(LegionManager::validate_channel_name("invalid").is_err());
    }
}