//! Advanced member management for Legion Protocol channels
//! 
//! Handles authentication, authorization, roles, and permissions for encrypted channels.

use crate::legion::{LegionError, LegionResult};
use phalanx_crypto::PublicKey;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, Duration};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Maximum number of members per channel
const MAX_MEMBERS_PER_CHANNEL: usize = 1000;

/// Member inactivity timeout (7 days)
const MEMBER_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Production-grade member manager for Legion channels
#[derive(Debug)]
pub struct MemberManager {
    /// Channel membership data
    channel_members: RwLock<HashMap<String, ChannelMembership>>,
    /// Global member registry
    member_registry: RwLock<HashMap<String, MemberInfo>>,
    /// Permission policies
    permission_policies: RwLock<HashMap<String, PermissionPolicy>>,
    /// Invitation system
    invitations: RwLock<HashMap<String, Vec<Invitation>>>,
}

/// Complete membership information for a channel
#[derive(Debug, Clone)]
struct ChannelMembership {
    /// Channel name
    channel_name: String,
    /// Channel owner
    owner: String,
    /// Channel members with roles
    members: HashMap<String, ChannelMember>,
    /// Channel creation time
    created_at: SystemTime,
    /// Channel settings
    settings: ChannelSettings,
    /// Active invitations for this channel
    pending_invitations: HashSet<String>,
}

/// Individual member information within a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelMember {
    /// Member ID
    member_id: String,
    /// Member role
    role: MemberRole,
    /// When they joined
    joined_at: SystemTime,
    /// Last activity
    last_activity: SystemTime,
    /// Member permissions (if custom)
    custom_permissions: Option<HashSet<Permission>>,
    /// Member public key
    public_key: PublicKey,
}

/// Global member information
#[derive(Debug, Clone)]
struct MemberInfo {
    /// Member ID
    member_id: String,
    /// Member's current identity
    identity: phalanx_crypto::Identity,
    /// Channels this member is in
    channels: HashSet<String>,
    /// Member registration time
    registered_at: SystemTime,
    /// Last global activity
    last_seen: SystemTime,
    /// Member metadata
    metadata: HashMap<String, String>,
}

/// Member roles in Legion channels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemberRole {
    /// Channel owner (highest privileges)
    Owner,
    /// Channel administrator  
    Admin,
    /// Channel moderator
    Moderator,
    /// Regular member
    Member,
    /// Read-only member
    Readonly,
    /// Temporarily muted member
    Muted,
}

/// Individual permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Message permissions
    SendMessages,
    EditMessages,
    DeleteMessages,
    SendFiles,
    SendReactions,
    
    // Channel management
    ManageChannel,
    ManageMembers,
    ManageRoles,
    ManageKeys,
    ViewAuditLog,
    
    // Member management
    InviteMembers,
    KickMembers,
    BanMembers,
    MuteMembers,
    
    // Advanced features
    ManageFederation,
    AccessBackups,
    ViewMetrics,
}

/// Permission policy for a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    /// Channel name
    pub channel_name: String,
    /// Default permissions for new members
    pub default_permissions: HashSet<Permission>,
    /// Role-based permissions
    pub role_permissions: HashMap<MemberRole, HashSet<Permission>>,
    /// Whether to inherit server permissions
    pub inherit_server_permissions: bool,
    /// Custom permission overrides
    pub permission_overrides: HashMap<String, HashSet<Permission>>,
}

/// Channel settings
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelSettings {
    /// Maximum number of members
    max_members: usize,
    /// Whether channel is invite-only
    invite_only: bool,
    /// Whether to require admin approval for joins
    require_approval: bool,
    /// Message retention policy
    message_retention: Duration,
    /// Whether to allow external invitations
    allow_external_invites: bool,
}

/// Invitation to join a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    /// Unique invitation ID
    pub id: String,
    /// Channel being invited to
    pub channel: String,
    /// Inviter member ID
    pub inviter: String,
    /// Invited member ID
    pub invitee: String,
    /// Invitation message
    pub message: Option<String>,
    /// Expiration time
    pub expires_at: SystemTime,
    /// Whether invitation has been used
    pub used: bool,
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self {
            max_members: MAX_MEMBERS_PER_CHANNEL,
            invite_only: true,
            require_approval: false,
            message_retention: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            allow_external_invites: false,
        }
    }
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        let mut role_permissions = HashMap::new();
        
        // Owner permissions (everything)
        role_permissions.insert(MemberRole::Owner, Permission::all());
        
        // Admin permissions (most things)
        role_permissions.insert(MemberRole::Admin, Permission::admin_set());
        
        // Moderator permissions (moderation)
        role_permissions.insert(MemberRole::Moderator, Permission::moderator_set());
        
        // Member permissions (basic)
        role_permissions.insert(MemberRole::Member, Permission::member_set());
        
        // Readonly permissions (very limited)
        role_permissions.insert(MemberRole::Readonly, HashSet::new());
        
        // Muted permissions (none)
        role_permissions.insert(MemberRole::Muted, HashSet::new());
        
        Self {
            channel_name: String::new(),
            default_permissions: Permission::member_set(),
            role_permissions,
            inherit_server_permissions: true,
            permission_overrides: HashMap::new(),
        }
    }
}

impl Permission {
    /// Get all possible permissions
    pub fn all() -> HashSet<Permission> {
        use Permission::*;
        [
            SendMessages, EditMessages, DeleteMessages, SendFiles, SendReactions,
            ManageChannel, ManageMembers, ManageRoles, ManageKeys, ViewAuditLog,
            InviteMembers, KickMembers, BanMembers, MuteMembers,
            ManageFederation, AccessBackups, ViewMetrics
        ].into_iter().collect()
    }
    
    /// Get admin permission set
    pub fn admin_set() -> HashSet<Permission> {
        use Permission::*;
        [
            SendMessages, EditMessages, DeleteMessages, SendFiles, SendReactions,
            ManageChannel, ManageMembers, ManageRoles, ManageKeys, ViewAuditLog,
            InviteMembers, KickMembers, BanMembers, MuteMembers,
            ViewMetrics
        ].into_iter().collect()
    }
    
    /// Get moderator permission set
    pub fn moderator_set() -> HashSet<Permission> {
        use Permission::*;
        [
            SendMessages, EditMessages, SendFiles, SendReactions,
            InviteMembers, KickMembers, MuteMembers
        ].into_iter().collect()
    }
    
    /// Get member permission set
    pub fn member_set() -> HashSet<Permission> {
        use Permission::*;
        [
            SendMessages, SendFiles, SendReactions
        ].into_iter().collect()
    }
}

impl MemberManager {
    /// Create a new member manager
    pub async fn new() -> LegionResult<Self> {
        Ok(Self {
            channel_members: RwLock::new(HashMap::new()),
            member_registry: RwLock::new(HashMap::new()),
            permission_policies: RwLock::new(HashMap::new()),
            invitations: RwLock::new(HashMap::new()),
        })
    }
    
    /// Register a new member globally
    pub async fn register_member(&self, member_id: String, identity: phalanx_crypto::Identity) -> LegionResult<()> {
        let mut registry = self.member_registry.write().await;
        
        let member_info = MemberInfo {
            member_id: member_id.clone(),
            identity,
            channels: HashSet::new(),
            registered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: HashMap::new(),
        };
        
        registry.insert(member_id.clone(), member_info);
        tracing::info!("Registered new member: {}", member_id);
        Ok(())
    }
    
    /// Create a new channel with an owner
    pub async fn create_channel(&self, channel_name: String, owner_id: String, owner_identity: phalanx_crypto::Identity) -> LegionResult<()> {
        // Register owner if not already registered
        if !self.is_member_registered(&owner_id).await {
            self.register_member(owner_id.clone(), owner_identity.clone()).await?;
        }
        
        let owner_member = ChannelMember {
            member_id: owner_id.clone(),
            role: MemberRole::Owner,
            joined_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            custom_permissions: None,
            public_key: owner_identity.public_key(),
        };
        
        let mut members = HashMap::new();
        members.insert(owner_id.clone(), owner_member);
        
        let membership = ChannelMembership {
            channel_name: channel_name.clone(),
            owner: owner_id.clone(),
            members,
            created_at: SystemTime::now(),
            settings: ChannelSettings::default(),
            pending_invitations: HashSet::new(),
        };
        
        let mut channel_members = self.channel_members.write().await;
        channel_members.insert(channel_name.clone(), membership);
        
        // Update member's channel list
        {
            let mut registry = self.member_registry.write().await;
            if let Some(member) = registry.get_mut(&owner_id) {
                member.channels.insert(channel_name.clone());
            }
        }
        
        // Set default permission policy
        self.set_permission_policy(channel_name.clone(), PermissionPolicy::default()).await?;
        
        tracing::info!("Created channel: {} with owner: {}", channel_name, owner_id);
        Ok(())
    }
    
    /// Add a member to a channel
    pub async fn add_channel_member(&self, channel_name: &str, member_id: &str, role: MemberRole) -> LegionResult<()> {
        let mut channel_members = self.channel_members.write().await;
        let channel = channel_members.get_mut(channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Check member limit
        if channel.members.len() >= channel.settings.max_members {
            return Err(LegionError::Member(format!("Channel {} is at maximum capacity", channel_name)));
        }
        
        // Get member info
        let registry = self.member_registry.read().await;
        let member_info = registry.get(member_id)
            .ok_or_else(|| LegionError::Member(format!("Member not registered: {}", member_id)))?;
        
        let channel_member = ChannelMember {
            member_id: member_id.to_string(),
            role: role.clone(),
            joined_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            custom_permissions: None,
            public_key: member_info.identity.public_key(),
        };
        
        channel.members.insert(member_id.to_string(), channel_member);
        drop(registry);
        drop(channel_members);
        
        // Update member's channel list
        {
            let mut registry = self.member_registry.write().await;
            if let Some(member) = registry.get_mut(member_id) {
                member.channels.insert(channel_name.to_string());
                member.last_seen = SystemTime::now();
            }
        }
        
        tracing::info!("Added member {} to channel: {} with role: {:?}", member_id, channel_name, role);
        Ok(())
    }
    
    /// Remove a member from a channel
    pub async fn remove_channel_member(&self, channel_name: &str, member_id: &str) -> LegionResult<()> {
        let mut channel_members = self.channel_members.write().await;
        let channel = channel_members.get_mut(channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Cannot remove owner
        if channel.owner == member_id {
            return Err(LegionError::Member("Cannot remove channel owner".to_string()));
        }
        
        channel.members.remove(member_id);
        drop(channel_members);
        
        // Update member's channel list
        {
            let mut registry = self.member_registry.write().await;
            if let Some(member) = registry.get_mut(member_id) {
                member.channels.remove(channel_name);
                member.last_seen = SystemTime::now();
            }
        }
        
        tracing::info!("Removed member {} from channel: {}", member_id, channel_name);
        Ok(())
    }
    
    /// Check if a member can join a channel
    pub async fn can_join_channel(&self, channel_name: &str, member_id: &str) -> LegionResult<bool> {
        let channel_members = self.channel_members.read().await;
        let channel = channel_members.get(channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        // Check if already a member
        if channel.members.contains_key(member_id) {
            return Ok(true);
        }
        
        // Check member limit
        if channel.members.len() >= channel.settings.max_members {
            return Ok(false);
        }
        
        // Check invite-only
        if channel.settings.invite_only {
            // Check for pending invitation
            let invitations = self.invitations.read().await;
            if let Some(channel_invitations) = invitations.get(channel_name) {
                let has_valid_invite = channel_invitations.iter().any(|inv| {
                    inv.invitee == member_id && 
                    !inv.used && 
                    SystemTime::now() < inv.expires_at
                });
                return Ok(has_valid_invite);
            }
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Check if a member is in a channel
    pub async fn is_channel_member(&self, channel_name: &str, member_id: &str) -> LegionResult<bool> {
        let channel_members = self.channel_members.read().await;
        if let Some(channel) = channel_members.get(channel_name) {
            Ok(channel.members.contains_key(member_id))
        } else {
            Ok(false)
        }
    }
    
    /// Check if a member is an admin or owner of a channel
    pub async fn is_channel_admin(&self, channel_name: &str, member_id: &str) -> LegionResult<bool> {
        let channel_members = self.channel_members.read().await;
        if let Some(channel) = channel_members.get(channel_name) {
            if let Some(member) = channel.members.get(member_id) {
                Ok(matches!(member.role, MemberRole::Owner | MemberRole::Admin))
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
    
    /// Check if a member has a specific permission in a channel
    pub async fn has_permission(&self, channel_name: &str, member_id: &str, permission: Permission) -> LegionResult<bool> {
        let channel_members = self.channel_members.read().await;
        let channel = channel_members.get(channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        let member = channel.members.get(member_id)
            .ok_or_else(|| LegionError::Member(format!("Member not in channel: {}", member_id)))?;
        
        // Check custom permissions first
        if let Some(custom_perms) = &member.custom_permissions {
            return Ok(custom_perms.contains(&permission));
        }
        
        // Check role-based permissions
        let policies = self.permission_policies.read().await;
        let policy = policies.get(channel_name).cloned().unwrap_or_default();
        
        if let Some(role_perms) = policy.role_permissions.get(&member.role) {
            return Ok(role_perms.contains(&permission));
        }
        
        // Check default permissions
        Ok(policy.default_permissions.contains(&permission))
    }
    
    /// Set permission policy for a channel
    pub async fn set_permission_policy(&self, channel_name: String, mut policy: PermissionPolicy) -> LegionResult<()> {
        policy.channel_name = channel_name.clone();
        
        let mut policies = self.permission_policies.write().await;
        policies.insert(channel_name.clone(), policy);
        
        tracing::info!("Set permission policy for channel: {}", channel_name);
        Ok(())
    }
    
    /// Create an invitation
    pub async fn create_invitation(&self, channel_name: String, inviter_id: String, invitee_id: String, expires_in: Duration) -> LegionResult<String> {
        // Check if inviter can invite
        if !self.has_permission(&channel_name, &inviter_id, Permission::InviteMembers).await? {
            return Err(LegionError::Member("Insufficient permissions to invite".to_string()));
        }
        
        let invitation_id = format!("inv_{}_{}", channel_name, uuid::Uuid::new_v4());
        let expires_at = SystemTime::now() + expires_in;
        
        let invitation = Invitation {
            id: invitation_id.clone(),
            channel: channel_name.clone(),
            inviter: inviter_id,
            invitee: invitee_id,
            message: None,
            expires_at,
            used: false,
        };
        
        let mut invitations = self.invitations.write().await;
        invitations.entry(channel_name.clone())
            .or_insert_with(Vec::new)
            .push(invitation);
        
        tracing::info!("Created invitation {} for channel: {}", invitation_id, channel_name);
        Ok(invitation_id)
    }
    
    /// Use an invitation
    pub async fn use_invitation(&self, invitation_id: &str) -> LegionResult<(String, String)> {
        let mut invitations = self.invitations.write().await;
        
        for (channel_name, channel_invitations) in invitations.iter_mut() {
            if let Some(invitation) = channel_invitations.iter_mut().find(|inv| inv.id == invitation_id) {
                if invitation.used {
                    return Err(LegionError::Member("Invitation already used".to_string()));
                }
                
                if SystemTime::now() >= invitation.expires_at {
                    return Err(LegionError::Member("Invitation expired".to_string()));
                }
                
                invitation.used = true;
                return Ok((channel_name.clone(), invitation.invitee.clone()));
            }
        }
        
        Err(LegionError::Member("Invitation not found".to_string()))
    }
    
    /// Check if member is registered globally
    pub async fn is_member_registered(&self, member_id: &str) -> bool {
        let registry = self.member_registry.read().await;
        registry.contains_key(member_id)
    }
    
    /// Get channel member count
    pub async fn channel_member_count(&self, channel_name: &str) -> usize {
        let channel_members = self.channel_members.read().await;
        channel_members.get(channel_name)
            .map(|channel| channel.members.len())
            .unwrap_or(0)
    }
    
    /// Get channel statistics
    pub async fn channel_stats(&self, channel_name: &str) -> LegionResult<ChannelStats> {
        let channel_members = self.channel_members.read().await;
        let channel = channel_members.get(channel_name)
            .ok_or_else(|| LegionError::Channel(format!("Channel not found: {}", channel_name)))?;
        
        let now = SystemTime::now();
        let active_members = channel.members.values()
            .filter(|member| {
                now.duration_since(member.last_activity).unwrap_or(Duration::MAX) < MEMBER_INACTIVITY_TIMEOUT
            })
            .count();
        
        let role_counts = channel.members.values().fold(HashMap::new(), |mut acc, member| {
            *acc.entry(member.role.clone()).or_insert(0) += 1;
            acc
        });
        
        Ok(ChannelStats {
            channel_name: channel_name.to_string(),
            owner: channel.owner.clone(),
            total_members: channel.members.len(),
            active_members,
            created_at: channel.created_at,
            role_counts,
            settings: channel.settings.clone(),
        })
    }
    
    /// Clean up expired invitations and inactive members
    pub async fn cleanup(&self) -> LegionResult<()> {
        let mut cleaned_invitations = 0;
        
        // Clean expired invitations
        {
            let mut invitations = self.invitations.write().await;
            let now = SystemTime::now();
            
            for channel_invitations in invitations.values_mut() {
                let initial_len = channel_invitations.len();
                channel_invitations.retain(|inv| !inv.used && now < inv.expires_at);
                cleaned_invitations += initial_len - channel_invitations.len();
            }
        }
        
        tracing::info!("Cleaned up {} expired invitations", cleaned_invitations);
        Ok(())
    }
    
    /// Get list of channels a user is a member of
    pub async fn get_user_channels(&self, user_id: &str) -> LegionResult<Vec<String>> {
        let registry = self.member_registry.read().await;
        if let Some(member) = registry.get(user_id) {
            Ok(member.channels.iter().cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }
}

/// Channel statistics
#[derive(Debug, Clone, Serialize)]
pub struct ChannelStats {
    pub channel_name: String,
    pub owner: String,
    pub total_members: usize,
    pub active_members: usize,
    pub created_at: SystemTime,
    pub role_counts: HashMap<MemberRole, usize>,
    pub settings: ChannelSettings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use phalanx_crypto::Identity;
    
    #[tokio::test]
    async fn test_member_manager_creation() {
        let manager = MemberManager::new().await.unwrap();
        assert!(!manager.is_member_registered("test_user").await);
    }
    
    #[tokio::test]
    async fn test_channel_creation_and_membership() {
        let manager = MemberManager::new().await.unwrap();
        let owner_id = "owner".to_string();
        let channel_name = "!test".to_string();
        let identity = Identity::generate();
        
        manager.create_channel(channel_name.clone(), owner_id.clone(), identity).await.unwrap();
        
        assert!(manager.is_channel_member(&channel_name, &owner_id).await.unwrap());
        assert!(manager.is_channel_admin(&channel_name, &owner_id).await.unwrap());
        assert_eq!(manager.channel_member_count(&channel_name).await, 1);
    }
    
    #[tokio::test]
    async fn test_permissions() {
        let manager = MemberManager::new().await.unwrap();
        let owner_id = "owner".to_string();
        let channel_name = "!test".to_string();
        let identity = Identity::generate();
        
        manager.create_channel(channel_name.clone(), owner_id.clone(), identity).await.unwrap();
        
        assert!(manager.has_permission(&channel_name, &owner_id, Permission::ManageChannel).await.unwrap());
        assert!(manager.has_permission(&channel_name, &owner_id, Permission::SendMessages).await.unwrap());
    }
}