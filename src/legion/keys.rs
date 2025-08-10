//! Advanced key management for Legion Protocol
//! 
//! Handles key rotation, storage, backup, and recovery for production deployments.

use crate::legion::{LegionError, LegionResult};
use phalanx::{Identity, protocol::KeyRotationMessage};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use blake3::Hasher;
use async_trait::async_trait;

/// Key rotation interval - 24 hours by default
const DEFAULT_ROTATION_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Maximum number of old keys to retain for message decryption
const MAX_OLD_KEYS: usize = 10;

/// Production-grade key manager for Legion Protocol
#[derive(Debug)]
pub struct KeyManager {
    /// Channel key storage
    channel_keys: RwLock<HashMap<String, ChannelKeys>>,
    /// Key rotation policies
    rotation_policies: RwLock<HashMap<String, RotationPolicy>>,
    /// Key derivation context
    kdf_context: Vec<u8>,
    /// Backup storage handler
    backup_handler: Option<Box<dyn BackupHandler>>,
}

/// Complete key information for a channel
#[derive(Debug, Clone)]
struct ChannelKeys {
    /// Current active key rotation info
    current_rotation: KeyRotationInfo,
    /// Previous keys for backward compatibility
    old_rotations: Vec<KeyRotationInfo>,
    /// Channel creation timestamp
    created_at: SystemTime,
    /// Last rotation timestamp  
    last_rotation: SystemTime,
    /// Next scheduled rotation
    next_rotation: SystemTime,
}

/// Key rotation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationInfo {
    /// Rotation sequence number
    pub sequence: u64,
    /// Rotation timestamp
    pub timestamp: u64,
    /// Key rotation message
    pub rotation_message: KeyRotationMessage,
    /// Key fingerprint for identification
    pub key_fingerprint: [u8; 32],
}

/// Key rotation policy for a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    /// Automatic rotation interval
    pub rotation_interval: Duration,
    /// Whether to rotate on member changes
    pub rotate_on_member_change: bool,
    /// Maximum key age before forced rotation
    pub max_key_age: Duration,
    /// Whether to require admin approval for rotation
    pub require_admin_approval: bool,
    /// Backup policy
    pub backup_policy: BackupPolicy,
}

/// Backup policy for keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupPolicy {
    /// No backup
    None,
    /// Local encrypted backup
    Local { 
        /// Backup directory path
        path: String,
        /// Encryption key for backups
        encryption_key: [u8; 32],
    },
    /// Remote backup service
    Remote {
        /// Service endpoint
        endpoint: String,
        /// Authentication credentials
        credentials: String,
    },
    /// Custom backup handler
    Custom {
        /// Handler configuration
        config: HashMap<String, String>,
    },
}

/// Trait for backup storage handlers
#[async_trait]
pub trait BackupHandler: Send + Sync + std::fmt::Debug {
    /// Store a key rotation backup
    async fn store_backup(&self, channel: &str, rotation: &KeyRotationInfo) -> LegionResult<()>;
    
    /// Retrieve a key rotation from backup
    async fn retrieve_backup(&self, channel: &str, sequence: u64) -> LegionResult<Option<KeyRotationInfo>>;
    
    /// List available backups for a channel
    async fn list_backups(&self, channel: &str) -> LegionResult<Vec<u64>>;
    
    /// Delete old backups
    async fn cleanup_backups(&self, channel: &str, keep_count: usize) -> LegionResult<()>;
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            rotation_interval: DEFAULT_ROTATION_INTERVAL,
            rotate_on_member_change: true,
            max_key_age: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            require_admin_approval: false,
            backup_policy: BackupPolicy::None,
        }
    }
}

impl KeyManager {
    /// Create a new key manager
    pub async fn new() -> LegionResult<Self> {
        let mut kdf_context = Vec::new();
        kdf_context.extend_from_slice(b"LEGION_KEY_MANAGER_V1");
        kdf_context.extend_from_slice(&SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_be_bytes());
        
        Ok(Self {
            channel_keys: RwLock::new(HashMap::new()),
            rotation_policies: RwLock::new(HashMap::new()),
            kdf_context,
            backup_handler: None,
        })
    }
    
    /// Set backup handler for key storage
    pub async fn set_backup_handler(&mut self, handler: Box<dyn BackupHandler>) {
        self.backup_handler = Some(handler);
    }
    
    /// Set rotation policy for a channel
    pub async fn set_rotation_policy(&self, channel: String, policy: RotationPolicy) -> LegionResult<()> {
        let mut policies = self.rotation_policies.write().await;
        policies.insert(channel.clone(), policy);
        
        tracing::info!("Set rotation policy for channel: {}", channel);
        Ok(())
    }
    
    /// Get rotation policy for a channel
    pub async fn get_rotation_policy(&self, channel: &str) -> RotationPolicy {
        let policies = self.rotation_policies.read().await;
        policies.get(channel).cloned().unwrap_or_default()
    }
    
    /// Store a key rotation
    pub async fn store_key_rotation(&self, channel: &str, rotation_msg: &KeyRotationMessage) -> LegionResult<()> {
        let rotation_info = KeyRotationInfo {
            sequence: rotation_msg.sequence,
            timestamp: rotation_msg.timestamp,
            rotation_message: rotation_msg.clone(),
            key_fingerprint: self.compute_key_fingerprint(rotation_msg),
        };
        
        let mut keys = self.channel_keys.write().await;
        let now = SystemTime::now();
        
        if let Some(channel_keys) = keys.get_mut(channel) {
            // Move current to old keys
            channel_keys.old_rotations.push(channel_keys.current_rotation.clone());
            
            // Limit old keys
            if channel_keys.old_rotations.len() > MAX_OLD_KEYS {
                channel_keys.old_rotations.remove(0);
            }
            
            // Update current
            channel_keys.current_rotation = rotation_info.clone();
            channel_keys.last_rotation = now;
            
            // Schedule next rotation
            let policy = self.get_rotation_policy(channel).await;
            channel_keys.next_rotation = now + policy.rotation_interval;
        } else {
            // First rotation for this channel
            let channel_keys = ChannelKeys {
                current_rotation: rotation_info.clone(),
                old_rotations: Vec::new(),
                created_at: now,
                last_rotation: now,
                next_rotation: now + DEFAULT_ROTATION_INTERVAL,
            };
            keys.insert(channel.to_string(), channel_keys);
        }
        
        // Store backup if handler is available
        if let Some(handler) = &self.backup_handler {
            handler.store_backup(channel, &rotation_info).await?;
        }
        
        tracing::info!("Stored key rotation for channel: {} (sequence: {})", channel, rotation_msg.sequence);
        Ok(())
    }
    
    /// Get current key rotation for a channel
    pub async fn get_current_rotation(&self, channel: &str) -> LegionResult<Option<KeyRotationInfo>> {
        let keys = self.channel_keys.read().await;
        Ok(keys.get(channel).map(|k| k.current_rotation.clone()))
    }
    
    /// Get key rotation by sequence number
    pub async fn get_rotation_by_sequence(&self, channel: &str, sequence: u64) -> LegionResult<Option<KeyRotationInfo>> {
        let keys = self.channel_keys.read().await;
        
        if let Some(channel_keys) = keys.get(channel) {
            // Check current rotation
            if channel_keys.current_rotation.sequence == sequence {
                return Ok(Some(channel_keys.current_rotation.clone()));
            }
            
            // Check old rotations
            for rotation in &channel_keys.old_rotations {
                if rotation.sequence == sequence {
                    return Ok(Some(rotation.clone()));
                }
            }
        }
        
        // Try backup if available
        if let Some(handler) = &self.backup_handler {
            return handler.retrieve_backup(channel, sequence).await;
        }
        
        Ok(None)
    }
    
    /// Check if channel needs key rotation
    pub async fn needs_rotation(&self, channel: &str) -> bool {
        let keys = self.channel_keys.read().await;
        
        if let Some(channel_keys) = keys.get(channel) {
            let now = SystemTime::now();
            
            // Check scheduled rotation time
            if now >= channel_keys.next_rotation {
                return true;
            }
            
            // Check maximum key age
            let policy = drop(keys);
            let policy = self.get_rotation_policy(channel).await;
            let keys = self.channel_keys.read().await;
            let channel_keys = keys.get(channel).unwrap();
            
            if let Ok(age) = now.duration_since(channel_keys.last_rotation) {
                if age >= policy.max_key_age {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Schedule key rotation for a channel
    pub async fn schedule_rotation(&self, channel: &str, when: SystemTime) -> LegionResult<()> {
        let mut keys = self.channel_keys.write().await;
        
        if let Some(channel_keys) = keys.get_mut(channel) {
            channel_keys.next_rotation = when;
            tracing::info!("Scheduled key rotation for channel: {}", channel);
            Ok(())
        } else {
            Err(LegionError::Key(format!("Channel not found: {}", channel)))
        }
    }
    
    /// Force key rotation on member change
    pub async fn on_member_change(&self, channel: &str) -> LegionResult<bool> {
        let policy = self.get_rotation_policy(channel).await;
        
        if policy.rotate_on_member_change {
            // Schedule immediate rotation
            self.schedule_rotation(channel, SystemTime::now()).await?;
            tracing::info!("Scheduled immediate key rotation due to member change in: {}", channel);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get channel key statistics
    pub async fn channel_stats(&self, channel: &str) -> LegionResult<ChannelKeyStats> {
        let keys = self.channel_keys.read().await;
        
        if let Some(channel_keys) = keys.get(channel) {
            let now = SystemTime::now();
            let age = now.duration_since(channel_keys.last_rotation).unwrap_or(Duration::ZERO);
            let next_rotation_in = channel_keys.next_rotation.duration_since(now).unwrap_or(Duration::ZERO);
            
            Ok(ChannelKeyStats {
                channel: channel.to_string(),
                current_sequence: channel_keys.current_rotation.sequence,
                total_rotations: channel_keys.old_rotations.len() + 1,
                key_age: age,
                next_rotation_in,
                created_at: channel_keys.created_at,
                last_rotation: channel_keys.last_rotation,
                key_fingerprint: channel_keys.current_rotation.key_fingerprint,
            })
        } else {
            Err(LegionError::Key(format!("Channel not found: {}", channel)))
        }
    }
    
    /// List all managed channels
    pub async fn list_channels(&self) -> Vec<String> {
        let keys = self.channel_keys.read().await;
        keys.keys().cloned().collect()
    }
    
    /// Clean up old keys and perform maintenance
    pub async fn cleanup(&self) -> LegionResult<()> {
        let mut cleaned_channels = 0;
        let mut cleaned_keys = 0;
        
        {
            let mut keys = self.channel_keys.write().await;
            
            for (channel, channel_keys) in keys.iter_mut() {
                let old_count = channel_keys.old_rotations.len();
                
                // Keep only the most recent old keys
                if channel_keys.old_rotations.len() > MAX_OLD_KEYS {
                    let keep_from = channel_keys.old_rotations.len() - MAX_OLD_KEYS;
                    channel_keys.old_rotations.drain(..keep_from);
                    cleaned_keys += old_count - channel_keys.old_rotations.len();
                }
                
                cleaned_channels += 1;
            }
        }
        
        // Cleanup backups if handler is available
        if let Some(handler) = &self.backup_handler {
            for channel in self.list_channels().await {
                handler.cleanup_backups(&channel, MAX_OLD_KEYS).await?;
            }
        }
        
        tracing::info!("Cleaned up {} channels, removed {} old keys", cleaned_channels, cleaned_keys);
        Ok(())
    }
    
    /// Compute key fingerprint for identification
    fn compute_key_fingerprint(&self, rotation_msg: &KeyRotationMessage) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&self.kdf_context);
        hasher.update(&rotation_msg.sequence.to_be_bytes());
        hasher.update(&rotation_msg.timestamp.to_be_bytes());
        
        for (pub_key, ephemeral) in &rotation_msg.member_keys {
            hasher.update(&pub_key.id());
            hasher.update(ephemeral.as_bytes());
        }
        
        hasher.finalize().into()
    }
}

/// Key statistics for a channel
#[derive(Debug, Clone, Serialize)]
pub struct ChannelKeyStats {
    pub channel: String,
    pub current_sequence: u64,
    pub total_rotations: usize,
    pub key_age: Duration,
    pub next_rotation_in: Duration,
    pub created_at: SystemTime,
    pub last_rotation: SystemTime,
    pub key_fingerprint: [u8; 32],
}

impl ChannelKeyStats {
    /// Convert to JSON-friendly format
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "channel": self.channel,
            "current_sequence": self.current_sequence,
            "total_rotations": self.total_rotations,
            "key_age_seconds": self.key_age.as_secs(),
            "next_rotation_seconds": self.next_rotation_in.as_secs(),
            "created_at": self.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            "last_rotation": self.last_rotation.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            "key_fingerprint": hex::encode(self.key_fingerprint)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phalanx::{Identity, protocol::KeyRotationMessage};
    
    #[tokio::test]
    async fn test_key_manager_creation() {
        let manager = KeyManager::new().await.unwrap();
        assert!(manager.list_channels().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_rotation_policy() {
        let manager = KeyManager::new().await.unwrap();
        let channel = "#test";
        
        let policy = RotationPolicy {
            rotation_interval: Duration::from_secs(3600),
            rotate_on_member_change: true,
            ..Default::default()
        };
        
        manager.set_rotation_policy(channel.to_string(), policy.clone()).await.unwrap();
        let retrieved = manager.get_rotation_policy(channel).await;
        
        assert_eq!(retrieved.rotation_interval, policy.rotation_interval);
        assert_eq!(retrieved.rotate_on_member_change, policy.rotate_on_member_change);
    }
}