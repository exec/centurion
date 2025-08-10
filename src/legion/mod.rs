//! Legion Protocol integration for Centurion server
//! 
//! Provides production-grade support for Legion Protocol encrypted channels,
//! member management, and Phalanx protocol integration.

pub mod channels;
pub mod keys;
pub mod members;
pub mod session;
pub mod federation;
pub mod channel_manager;

use crate::error::CenturionError;
use legion_protocol::{IronSession, IronVersion, Capability};
use phalanx::{Identity, PhalanxGroup, AsyncPhalanxGroup};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Result type for Legion operations
pub type LegionResult<T> = Result<T, LegionError>;

/// Comprehensive error types for Legion Protocol operations
#[derive(Debug, thiserror::Error)]
pub enum LegionError {
    /// Phalanx protocol error
    #[error("Phalanx error: {0}")]
    Phalanx(#[from] phalanx::PhalanxError),
    
    /// Legion Protocol error
    #[error("Legion Protocol error: {0}")]
    Protocol(#[from] legion_protocol::IronError),
    
    /// Channel operation error
    #[error("Channel error: {0}")]
    Channel(String),
    
    /// Member operation error
    #[error("Member error: {0}")]
    Member(String),
    
    /// Key management error
    #[error("Key management error: {0}")]
    Key(String),
    
    /// Session error
    #[error("Session error: {0}")]
    Session(String),
    
    /// Federation error
    #[error("Federation error: {0}")]
    Federation(String),
    
    /// Generic server error
    #[error("Server error: {0}")]
    Server(#[from] CenturionError),
}

/// Legion Protocol manager for the Centurion server
#[derive(Debug)]
pub struct LegionManager {
    /// Server identity for Legion operations
    server_identity: Arc<RwLock<Identity>>,
    /// Active Legion channels
    channels: Arc<DashMap<String, AsyncPhalanxGroup>>,
    /// Client sessions with Legion capabilities
    sessions: Arc<DashMap<String, session::LegionSession>>,
    /// Key manager for rotation and storage
    key_manager: Arc<keys::KeyManager>,
    /// Member manager for authentication and authorization
    member_manager: Arc<members::MemberManager>,
    /// Federation manager for cross-server communication
    federation_manager: Arc<federation::FederationManager>,
}

impl LegionManager {
    /// Create a new Legion Protocol manager
    pub async fn new() -> LegionResult<Self> {
        let server_identity = Identity::generate();
        let key_manager = keys::KeyManager::new().await?;
        let member_manager = members::MemberManager::new().await?;
        let federation_manager = federation::FederationManager::new().await?;
        
        Ok(Self {
            server_identity: Arc::new(RwLock::new(server_identity)),
            channels: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            key_manager: Arc::new(key_manager),
            member_manager: Arc::new(member_manager),
            federation_manager: Arc::new(federation_manager),
        })
    }
    
    /// Get server identity for Legion operations
    pub async fn server_identity(&self) -> Identity {
        self.server_identity.read().await.clone()
    }
    
    /// Check if a client supports Legion Protocol
    pub async fn supports_legion(&self, client_id: &str) -> bool {
        if let Some(session) = self.sessions.get(client_id) {
            session.supports_legion()
        } else {
            false
        }
    }
    
    /// Create a new Legion session for a client
    pub async fn create_session(&self, client_id: String, capabilities: Vec<Capability>) -> LegionResult<()> {
        let session = session::LegionSession::new(client_id.clone(), capabilities).await?;
        self.sessions.insert(client_id, session);
        Ok(())
    }
    
    /// Remove a client session
    pub async fn remove_session(&self, client_id: &str) -> LegionResult<()> {
        self.sessions.remove(client_id);
        Ok(())
    }
    
    /// Create a new Legion encrypted channel
    pub async fn create_channel(&self, channel_name: String, creator_id: String) -> LegionResult<()> {
        // Validate channel name
        if !legion_protocol::utils::is_legion_encrypted_channel(&channel_name) {
            return Err(LegionError::Channel(format!("Invalid Legion channel name: {}", channel_name)));
        }
        
        // Check if channel already exists
        if self.channels.contains_key(&channel_name) {
            return Err(LegionError::Channel(format!("Channel {} already exists", channel_name)));
        }
        
        // Get creator session
        let creator_session = self.sessions.get(&creator_id)
            .ok_or_else(|| LegionError::Member(format!("Creator session not found: {}", creator_id)))?;
        
        if !creator_session.supports_legion() {
            return Err(LegionError::Member("Creator does not support Legion Protocol".to_string()));
        }
        
        // Create the encrypted group
        let creator_identity = creator_session.identity().await?;
        let group = AsyncPhalanxGroup::new(creator_identity);
        
        // Register channel
        self.channels.insert(channel_name.clone(), group);
        
        // Add creator as owner
        self.member_manager.add_channel_member(
            &channel_name,
            &creator_id,
            members::MemberRole::Owner
        ).await?;
        
        tracing::info!("Created Legion channel: {} by {}", channel_name, creator_id);
        Ok(())
    }
    
    /// Join a Legion encrypted channel
    pub async fn join_channel(&self, channel_name: String, client_id: String) -> LegionResult<()> {
        // Validate inputs
        if !legion_protocol::utils::is_legion_encrypted_channel(&channel_name) {
            return Err(LegionError::Channel(format!("Not a Legion channel: {}", channel_name)));
        }
        
        // Get client session
        let client_session = self.sessions.get(&client_id)
            .ok_or_else(|| LegionError::Member(format!("Client session not found: {}", client_id)))?;
        
        if !client_session.supports_legion() {
            return Err(LegionError::Member("Client does not support Legion Protocol".to_string()));
        }
        
        // Get channel
        let channel = self.channels.get(&channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Check permissions
        let can_join = self.member_manager.can_join_channel(&channel_name, &client_id).await?;
        if !can_join {
            return Err(LegionError::Member(format!("Permission denied to join {}", channel_name)));
        }
        
        // Add member to channel
        let client_identity = client_session.identity().await?;
        channel.add_member(client_identity.public_key(), phalanx::group::MemberRole::Member).await?;
        
        // Update member tracking
        self.member_manager.add_channel_member(
            &channel_name,
            &client_id,
            members::MemberRole::Member
        ).await?;
        
        tracing::info!("Client {} joined Legion channel: {}", client_id, channel_name);
        Ok(())
    }
    
    /// Leave a Legion encrypted channel
    pub async fn leave_channel(&self, channel_name: String, client_id: String) -> LegionResult<()> {
        // Get channel
        let channel = self.channels.get(&channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Get client session
        let client_session = self.sessions.get(&client_id)
            .ok_or_else(|| LegionError::Member(format!("Client session not found: {}", client_id)))?;
        
        // Remove from Phalanx group
        let client_identity = client_session.identity().await?;
        let member_id = client_identity.id();
        channel.remove_member(&member_id).await?;
        
        // Update member tracking
        self.member_manager.remove_channel_member(&channel_name, &client_id).await?;
        
        tracing::info!("Client {} left Legion channel: {}", client_id, channel_name);
        Ok(())
    }
    
    /// Send an encrypted message to a Legion channel
    pub async fn send_message(
        &self,
        channel_name: String,
        sender_id: String,
        message: String
    ) -> LegionResult<Vec<u8>> {
        // Get channel
        let channel = self.channels.get(&channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Verify sender is member
        if !self.member_manager.is_channel_member(&channel_name, &sender_id).await? {
            return Err(LegionError::Member(format!("Sender {} is not a member of {}", sender_id, channel_name)));
        }
        
        // Create message content
        let content = phalanx::message::MessageContent::text(message);
        
        // Encrypt message
        let encrypted_msg = channel.encrypt_message(&content).await?;
        
        // Serialize for transmission
        let serialized = bincode::serialize(&encrypted_msg)
            .map_err(|e| LegionError::Channel(format!("Message serialization failed: {}", e)))?;
        
        tracing::debug!("Encrypted message in channel {} from {}", channel_name, sender_id);
        Ok(serialized)
    }
    
    /// Receive and decrypt a message from a Legion channel
    pub async fn receive_message(
        &self,
        channel_name: String,
        recipient_id: String,
        encrypted_data: Vec<u8>
    ) -> LegionResult<String> {
        // Get channel
        let channel = self.channels.get(&channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Verify recipient is member
        if !self.member_manager.is_channel_member(&channel_name, &recipient_id).await? {
            return Err(LegionError::Member(format!("Recipient {} is not a member of {}", recipient_id, channel_name)));
        }
        
        // Deserialize message
        let encrypted_msg: phalanx::message::GroupMessage = bincode::deserialize(&encrypted_data)
            .map_err(|e| LegionError::Channel(format!("Message deserialization failed: {}", e)))?;
        
        // Decrypt message
        let content = channel.decrypt_message(&encrypted_msg).await?;
        
        // Extract text
        let message_text = content.as_string()
            .map_err(|e| LegionError::Channel(format!("Message decode failed: {}", e)))?;
        
        tracing::debug!("Decrypted message in channel {} for {}", channel_name, recipient_id);
        Ok(message_text)
    }
    
    /// Get channel statistics
    pub async fn channel_stats(&self, channel_name: &str) -> LegionResult<phalanx::group::GroupStats> {
        let channel = self.channels.get(channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        Ok(channel.stats().await)
    }
    
    /// List all Legion channels
    pub async fn list_channels(&self) -> Vec<String> {
        self.channels.iter().map(|entry| entry.key().clone()).collect()
    }
    
    /// Perform key rotation for a channel
    pub async fn rotate_channel_keys(&self, channel_name: String, admin_id: String) -> LegionResult<()> {
        // Get channel
        let channel = self.channels.get(&channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Check admin permissions
        let is_admin = self.member_manager.is_channel_admin(&channel_name, &admin_id).await?;
        if !is_admin {
            return Err(LegionError::Member(format!("User {} is not an admin of {}", admin_id, channel_name)));
        }
        
        // Perform rotation
        let rotation_msg = channel.rotate_keys().await?;
        
        // Store rotation info
        self.key_manager.store_key_rotation(&channel_name, &rotation_msg).await?;
        
        tracing::info!("Rotated keys for channel {} by admin {}", channel_name, admin_id);
        Ok(())
    }
    
    /// Clean up inactive channels and sessions
    pub async fn cleanup(&self) -> LegionResult<()> {
        // Remove inactive sessions
        let inactive_sessions: Vec<String> = self.sessions
            .iter()
            .filter_map(|entry| {
                let session = entry.value();
                if session.is_inactive() {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();
        
        for session_id in inactive_sessions {
            self.remove_session(&session_id).await?;
        }
        
        // Perform key manager cleanup
        self.key_manager.cleanup().await?;
        
        // Perform member manager cleanup  
        self.member_manager.cleanup().await?;
        
        tracing::debug!("Completed Legion Protocol cleanup");
        Ok(())
    }
}

/// Convert Legion errors to Centurion errors
impl From<LegionError> for CenturionError {
    fn from(err: LegionError) -> Self {
        match err {
            LegionError::Server(server_err) => server_err,
            _ => CenturionError::Generic(err.to_string()),
        }
    }
}