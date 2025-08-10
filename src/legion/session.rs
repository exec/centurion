//! Legion Protocol session management
//! 
//! Handles client sessions with Legion Protocol capabilities,
//! identity management, and state tracking.

use crate::legion::{LegionError, LegionResult};
use legion_protocol::{Capability, IronSession, IronVersion};
use phalanx::{Identity, PublicKey};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashSet;
use tokio::sync::RwLock;

/// Maximum session idle time before cleanup (30 minutes)
const MAX_IDLE_TIME: Duration = Duration::from_secs(30 * 60);

/// Legion Protocol session for a connected client
#[derive(Debug)]
pub struct LegionSession {
    /// Client identifier
    client_id: String,
    /// Client's identity for encryption
    identity: RwLock<Identity>,
    /// Legion Protocol session state
    legion_session: RwLock<IronSession>,
    /// Client capabilities
    capabilities: HashSet<Capability>,
    /// Session creation time
    created_at: SystemTime,
    /// Last activity timestamp
    last_activity: RwLock<SystemTime>,
    /// Channels this client is in
    joined_channels: RwLock<HashSet<String>>,
    /// Whether Legion Protocol is fully negotiated
    legion_active: RwLock<bool>,
}

impl LegionSession {
    /// Create a new Legion session
    pub async fn new(client_id: String, capabilities: Vec<Capability>) -> LegionResult<Self> {
        let identity = Identity::generate();
        let mut legion_session = IronSession::new();
        
        // Check for Legion Protocol capabilities
        let has_legion = capabilities.iter().any(|cap| {
            matches!(cap, Capability::LegionProtocolV1 | Capability::IronProtocolV1)
        });
        
        if has_legion {
            // Prefer Legion Protocol over Iron Protocol
            if capabilities.contains(&Capability::LegionProtocolV1) {
                legion_session.set_version(IronVersion::V1);
            } else if capabilities.contains(&Capability::IronProtocolV1) {
                legion_session.set_version(IronVersion::V1);
            }
        }
        
        let now = SystemTime::now();
        
        Ok(Self {
            client_id,
            identity: RwLock::new(identity),
            legion_session: RwLock::new(legion_session),
            capabilities: capabilities.into_iter().collect(),
            created_at: now,
            last_activity: RwLock::new(now),
            joined_channels: RwLock::new(HashSet::new()),
            legion_active: RwLock::new(has_legion),
        })
    }
    
    /// Check if this session supports Legion Protocol
    pub fn supports_legion(&self) -> bool {
        self.capabilities.contains(&Capability::LegionProtocolV1) ||
        self.capabilities.contains(&Capability::IronProtocolV1)
    }
    
    /// Check if Legion Protocol is fully active
    pub async fn is_legion_active(&self) -> bool {
        *self.legion_active.read().await
    }
    
    /// Activate Legion Protocol for this session
    pub async fn activate_legion(&self) -> LegionResult<()> {
        if !self.supports_legion() {
            return Err(LegionError::Session(
                "Client does not support Legion Protocol".to_string()
            ));
        }
        
        let mut legion_session = self.legion_session.write().await;
        legion_session.complete_negotiation();
        
        *self.legion_active.write().await = true;
        self.update_activity().await;
        
        tracing::info!("Activated Legion Protocol for client: {}", self.client_id);
        Ok(())
    }
    
    /// Get client identity
    pub async fn identity(&self) -> LegionResult<Identity> {
        Ok(self.identity.read().await.clone())
    }
    
    /// Get client public key
    pub async fn public_key(&self) -> LegionResult<PublicKey> {
        let identity = self.identity.read().await;
        Ok(identity.public_key())
    }
    
    /// Update last activity timestamp
    pub async fn update_activity(&self) {
        *self.last_activity.write().await = SystemTime::now();
    }
    
    /// Check if session is inactive
    pub fn is_inactive(&self) -> bool {
        if let Ok(last_activity) = self.last_activity.try_read() {
            if let Ok(elapsed) = last_activity.elapsed() {
                return elapsed > MAX_IDLE_TIME;
            }
        }
        false
    }
    
    /// Get session age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed().unwrap_or(Duration::ZERO)
    }
    
    /// Add a channel to the session
    pub async fn join_channel(&self, channel_name: String) -> LegionResult<()> {
        if !legion_protocol::utils::is_legion_encrypted_channel(&channel_name) {
            return Err(LegionError::Channel(format!("Not a Legion channel: {}", channel_name)));
        }
        
        let mut channels = self.joined_channels.write().await;
        channels.insert(channel_name.clone());
        
        // Add to Legion session if active
        if *self.legion_active.read().await {
            let mut legion_session = self.legion_session.write().await;
            legion_session.add_encrypted_channel(channel_name);
        }
        
        self.update_activity().await;
        Ok(())
    }
    
    /// Remove a channel from the session
    pub async fn leave_channel(&self, channel_name: &str) -> LegionResult<()> {
        let mut channels = self.joined_channels.write().await;
        channels.remove(channel_name);
        
        // Remove from Legion session if active
        if *self.legion_active.read().await {
            let mut legion_session = self.legion_session.write().await;
            legion_session.remove_encrypted_channel(channel_name);
        }
        
        self.update_activity().await;
        Ok(())
    }
    
    /// Get list of joined channels
    pub async fn joined_channels(&self) -> HashSet<String> {
        self.joined_channels.read().await.clone()
    }
    
    /// Check if client is in a specific channel
    pub async fn is_in_channel(&self, channel_name: &str) -> bool {
        self.joined_channels.read().await.contains(channel_name)
    }
    
    /// Get client capabilities
    pub fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }
    
    /// Get client ID
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    
    /// Get session statistics
    pub async fn stats(&self) -> SessionStats {
        let channels = self.joined_channels.read().await;
        let last_activity = *self.last_activity.read().await;
        let legion_active = *self.legion_active.read().await;
        
        SessionStats {
            client_id: self.client_id.clone(),
            created_at: self.created_at,
            last_activity,
            age: self.age(),
            legion_active,
            supports_legion: self.supports_legion(),
            joined_channels: channels.len(),
            capabilities_count: self.capabilities.len(),
        }
    }
    
    /// Generate session handshake for joining groups
    pub async fn create_handshake(&self, group_id: [u8; 32]) -> LegionResult<phalanx::protocol::HandshakeMessage> {
        let identity = self.identity.read().await;
        
        let capabilities = vec![
            "legion-protocol/v1".to_string(),
            "phalanx/v1".to_string(),
        ];
        
        let client_info = format!("centurion-client/{}", env!("CARGO_PKG_VERSION"));
        
        phalanx::protocol::HandshakeMessage::new(
            &*identity,
            group_id,
            capabilities,
            client_info,
        ).map_err(|e| LegionError::Session(format!("Handshake creation failed: {}", e)))
    }
    
    /// Validate and process a handshake message
    pub async fn process_handshake(
        &self,
        handshake: phalanx::protocol::HandshakeMessage
    ) -> LegionResult<phalanx::protocol::HandshakePayload> {
        handshake.verify_and_decrypt()
            .map_err(|e| LegionError::Session(format!("Handshake processing failed: {}", e)))
    }
}

/// Session statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub client_id: String,
    pub created_at: SystemTime,
    pub last_activity: SystemTime,
    pub age: Duration,
    pub legion_active: bool,
    pub supports_legion: bool,
    pub joined_channels: usize,
    pub capabilities_count: usize,
}

impl SessionStats {
    /// Convert to JSON-friendly format
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "client_id": self.client_id,
            "created_at": self.created_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            "last_activity": self.last_activity.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            "age_seconds": self.age.as_secs(),
            "legion_active": self.legion_active,
            "supports_legion": self.supports_legion,
            "joined_channels": self.joined_channels,
            "capabilities_count": self.capabilities_count
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_session_creation() {
        let capabilities = vec![Capability::LegionProtocolV1, Capability::MessageTags];
        let session = LegionSession::new("test_client".to_string(), capabilities).await.unwrap();
        
        assert!(session.supports_legion());
        assert_eq!(session.client_id(), "test_client");
        assert!(!session.is_inactive());
    }
    
    #[tokio::test]
    async fn test_channel_management() {
        let capabilities = vec![Capability::LegionProtocolV1];
        let session = LegionSession::new("test_client".to_string(), capabilities).await.unwrap();
        
        session.activate_legion().await.unwrap();
        session.join_channel("!encrypted".to_string()).await.unwrap();
        
        assert!(session.is_in_channel("!encrypted").await);
        assert_eq!(session.joined_channels().await.len(), 1);
        
        session.leave_channel("!encrypted").await.unwrap();
        assert!(!session.is_in_channel("!encrypted").await);
        assert_eq!(session.joined_channels().await.len(), 0);
    }
    
    #[tokio::test]
    async fn test_legion_activation() {
        let capabilities = vec![Capability::LegionProtocolV1];
        let session = LegionSession::new("test_client".to_string(), capabilities).await.unwrap();
        
        assert!(!session.is_legion_active().await);
        
        session.activate_legion().await.unwrap();
        assert!(session.is_legion_active().await);
    }
}