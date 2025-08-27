//! Federation support for Legion Protocol
//! 
//! Handles cross-server encrypted communication and Herald bridge integration.

use crate::legion::{LegionError, LegionResult};
use phalanx_crypto::{Identity, PublicKey};
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

/// Federation manager for cross-server Legion Protocol communication
#[derive(Debug)]
pub struct FederationManager {
    /// Connected servers
    federated_servers: RwLock<HashMap<String, FederatedServer>>,
    /// Bridge configurations
    bridge_configs: RwLock<HashMap<String, BridgeConfig>>,
    /// Federation policies
    policies: RwLock<FederationPolicy>,
}

/// Information about a federated server
#[derive(Debug, Clone)]
struct FederatedServer {
    /// Server hostname
    hostname: String,
    /// Server public key
    public_key: PublicKey,
    /// Connection status
    status: FederationStatus,
    /// Last communication
    last_seen: SystemTime,
    /// Supported capabilities
    capabilities: Vec<String>,
    /// Trust level
    trust_level: TrustLevel,
}

/// Federation status
#[derive(Debug, Clone, PartialEq)]
enum FederationStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

/// Trust levels for federated servers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum TrustLevel {
    /// Fully trusted server
    Trusted,
    /// Partially trusted (limited operations)
    Limited,
    /// Under verification
    Pending,
    /// Blocked server
    Blocked,
}

/// Bridge configuration for Herald integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Bridge name
    pub name: String,
    /// Bridge type (Herald, Matrix, etc.)
    pub bridge_type: String,
    /// Remote server endpoint
    pub endpoint: String,
    /// Authentication credentials
    pub credentials: BridgeCredentials,
    /// Channel mappings
    pub channel_mappings: HashMap<String, String>,
    /// Enabled features
    pub features: Vec<String>,
}

/// Bridge authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeCredentials {
    /// Username/password authentication
    UserPass { username: String, password: String },
    /// Token-based authentication
    Token { token: String },
    /// Certificate-based authentication
    Certificate { cert_path: String, key_path: String },
    /// Legion Protocol identity
    LegionIdentity { identity_bytes: Vec<u8> },
}

/// Federation policies and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FederationPolicy {
    /// Whether federation is enabled
    enabled: bool,
    /// Auto-accept federation requests
    auto_accept: bool,
    /// Maximum federated servers
    max_servers: usize,
    /// Default trust level for new servers
    default_trust_level: TrustLevel,
    /// Allowed bridge types
    allowed_bridge_types: Vec<String>,
    /// Security requirements
    security_requirements: SecurityRequirements,
}

/// Security requirements for federation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecurityRequirements {
    /// Require TLS encryption
    require_tls: bool,
    /// Minimum supported Legion Protocol version
    min_legion_version: String,
    /// Required capabilities
    required_capabilities: Vec<String>,
    /// Certificate validation mode
    cert_validation: CertValidation,
}

/// Certificate validation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
enum CertValidation {
    /// Strict certificate validation
    Strict,
    /// Validate with custom CA
    CustomCA { ca_path: String },
    /// Trust on first use
    TOFU,
    /// No validation (insecure)
    None,
}

impl Default for FederationPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_accept: false,
            max_servers: 10,
            default_trust_level: TrustLevel::Pending,
            allowed_bridge_types: vec![
                "herald".to_string(),
                "matrix".to_string(),
            ],
            security_requirements: SecurityRequirements {
                require_tls: true,
                min_legion_version: "v1".to_string(),
                required_capabilities: vec![
                    "legion-protocol/v1".to_string(),
                    "phalanx/v1".to_string(),
                ],
                cert_validation: CertValidation::Strict,
            },
        }
    }
}

impl FederationManager {
    /// Create a new federation manager
    pub async fn new() -> LegionResult<Self> {
        Ok(Self {
            federated_servers: RwLock::new(HashMap::new()),
            bridge_configs: RwLock::new(HashMap::new()),
            policies: RwLock::new(FederationPolicy::default()),
        })
    }
    
    /// Enable or disable federation
    pub async fn set_federation_enabled(&self, enabled: bool) -> LegionResult<()> {
        let mut policies = self.policies.write().await;
        policies.enabled = enabled;
        
        tracing::info!("Federation {}", if enabled { "enabled" } else { "disabled" });
        Ok(())
    }
    
    /// Check if federation is enabled
    pub async fn is_federation_enabled(&self) -> bool {
        let policies = self.policies.read().await;
        policies.enabled
    }
    
    /// Add a federated server
    pub async fn add_federated_server(&self, hostname: String, public_key: PublicKey, trust_level: TrustLevel) -> LegionResult<()> {
        if !self.is_federation_enabled().await {
            return Err(LegionError::Federation("Federation is disabled".to_string()));
        }
        
        let policies = self.policies.read().await;
        let servers = self.federated_servers.read().await;
        
        if servers.len() >= policies.max_servers {
            return Err(LegionError::Federation("Maximum federated servers reached".to_string()));
        }
        drop(servers);
        drop(policies);
        
        let server = FederatedServer {
            hostname: hostname.clone(),
            public_key,
            status: FederationStatus::Connecting,
            last_seen: SystemTime::now(),
            capabilities: Vec::new(),
            trust_level,
        };
        
        let mut servers = self.federated_servers.write().await;
        servers.insert(hostname.clone(), server);
        
        tracing::info!("Added federated server: {}", hostname);
        Ok(())
    }
    
    /// Remove a federated server
    pub async fn remove_federated_server(&self, hostname: &str) -> LegionResult<()> {
        let mut servers = self.federated_servers.write().await;
        if servers.remove(hostname).is_some() {
            tracing::info!("Removed federated server: {}", hostname);
            Ok(())
        } else {
            Err(LegionError::Federation(format!("Server not found: {}", hostname)))
        }
    }
    
    /// Update server status
    pub async fn update_server_status(&self, hostname: &str, status: FederationStatus) -> LegionResult<()> {
        let mut servers = self.federated_servers.write().await;
        if let Some(server) = servers.get_mut(hostname) {
            server.status = status;
            server.last_seen = SystemTime::now();
            Ok(())
        } else {
            Err(LegionError::Federation(format!("Server not found: {}", hostname)))
        }
    }
    
    /// Get federated server info
    pub async fn get_server_info(&self, hostname: &str) -> LegionResult<FederatedServerInfo> {
        let servers = self.federated_servers.read().await;
        if let Some(server) = servers.get(hostname) {
            Ok(FederatedServerInfo {
                hostname: server.hostname.clone(),
                status: format!("{:?}", server.status),
                last_seen: server.last_seen,
                capabilities: server.capabilities.clone(),
                trust_level: format!("{:?}", server.trust_level),
            })
        } else {
            Err(LegionError::Federation(format!("Server not found: {}", hostname)))
        }
    }
    
    /// List all federated servers
    pub async fn list_federated_servers(&self) -> Vec<FederatedServerInfo> {
        let servers = self.federated_servers.read().await;
        servers.values().map(|server| FederatedServerInfo {
            hostname: server.hostname.clone(),
            status: format!("{:?}", server.status),
            last_seen: server.last_seen,
            capabilities: server.capabilities.clone(),
            trust_level: format!("{:?}", server.trust_level),
        }).collect()
    }
    
    /// Configure a bridge
    pub async fn configure_bridge(&self, config: BridgeConfig) -> LegionResult<()> {
        if !self.is_federation_enabled().await {
            return Err(LegionError::Federation("Federation is disabled".to_string()));
        }
        
        // Validate bridge type
        let policies = self.policies.read().await;
        if !policies.allowed_bridge_types.contains(&config.bridge_type) {
            return Err(LegionError::Federation(format!("Bridge type not allowed: {}", config.bridge_type)));
        }
        drop(policies);
        
        let mut bridges = self.bridge_configs.write().await;
        bridges.insert(config.name.clone(), config.clone());
        
        tracing::info!("Configured bridge: {} (type: {})", config.name, config.bridge_type);
        Ok(())
    }
    
    /// Remove bridge configuration
    pub async fn remove_bridge(&self, bridge_name: &str) -> LegionResult<()> {
        let mut bridges = self.bridge_configs.write().await;
        if bridges.remove(bridge_name).is_some() {
            tracing::info!("Removed bridge: {}", bridge_name);
            Ok(())
        } else {
            Err(LegionError::Federation(format!("Bridge not found: {}", bridge_name)))
        }
    }
    
    /// Send message through federation
    pub async fn send_federated_message(&self, target_server: &str, channel: &str, message: Vec<u8>) -> LegionResult<()> {
        let servers = self.federated_servers.read().await;
        let server = servers.get(target_server)
            .ok_or_else(|| LegionError::Federation(format!("Target server not found: {}", target_server)))?;
        
        // Check if server is trusted and connected
        if server.trust_level == TrustLevel::Blocked {
            return Err(LegionError::Federation("Target server is blocked".to_string()));
        }
        
        if server.status != FederationStatus::Connected {
            return Err(LegionError::Federation("Target server is not connected".to_string()));
        }
        
        // TODO: Implement actual message sending through Herald bridge or direct connection
        tracing::info!("Would send federated message to {} in channel {}", target_server, channel);
        
        Ok(())
    }
    
    /// Process incoming federated message
    pub async fn process_federated_message(&self, source_server: &str, channel: &str, message: Vec<u8>) -> LegionResult<()> {
        let servers = self.federated_servers.read().await;
        let server = servers.get(source_server)
            .ok_or_else(|| LegionError::Federation(format!("Source server not found: {}", source_server)))?;
        
        // Check if server is trusted
        if server.trust_level == TrustLevel::Blocked {
            return Err(LegionError::Federation("Source server is blocked".to_string()));
        }
        
        // TODO: Process and route the federated message
        tracing::info!("Processing federated message from {} in channel {}", source_server, channel);
        
        Ok(())
    }
    
    /// Get federation statistics
    pub async fn federation_stats(&self) -> FederationStats {
        let servers = self.federated_servers.read().await;
        let bridges = self.bridge_configs.read().await;
        let policies = self.policies.read().await;
        
        let connected_servers = servers.values()
            .filter(|s| s.status == FederationStatus::Connected)
            .count();
        
        let trusted_servers = servers.values()
            .filter(|s| s.trust_level == TrustLevel::Trusted)
            .count();
        
        FederationStats {
            enabled: policies.enabled,
            total_servers: servers.len(),
            connected_servers,
            trusted_servers,
            total_bridges: bridges.len(),
            active_bridges: 0, // TODO: Track active bridges
        }
    }
    
    /// Cleanup inactive servers and expired configurations
    pub async fn cleanup(&self) -> LegionResult<()> {
        let mut cleaned_servers = 0;
        let now = SystemTime::now();
        
        // Remove servers that haven't been seen in 24 hours
        {
            let mut servers = self.federated_servers.write().await;
            let initial_count = servers.len();
            
            servers.retain(|_, server| {
                let inactive = now.duration_since(server.last_seen)
                    .unwrap_or(std::time::Duration::MAX)
                    .as_secs() < 24 * 60 * 60;
                inactive || server.status == FederationStatus::Connected
            });
            
            cleaned_servers = initial_count - servers.len();
        }
        
        tracing::info!("Cleaned up {} inactive federated servers", cleaned_servers);
        Ok(())
    }
}

/// Public information about a federated server
#[derive(Debug, Clone, Serialize)]
pub struct FederatedServerInfo {
    pub hostname: String,
    pub status: String,
    pub last_seen: SystemTime,
    pub capabilities: Vec<String>,
    pub trust_level: String,
}

/// Federation statistics
#[derive(Debug, Clone, Serialize)]
pub struct FederationStats {
    pub enabled: bool,
    pub total_servers: usize,
    pub connected_servers: usize,
    pub trusted_servers: usize,
    pub total_bridges: usize,
    pub active_bridges: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use phalanx_crypto::Identity;
    
    #[tokio::test]
    async fn test_federation_manager_creation() {
        let manager = FederationManager::new().await.unwrap();
        assert!(!manager.is_federation_enabled().await);
        assert_eq!(manager.list_federated_servers().await.len(), 0);
    }
    
    #[tokio::test]
    async fn test_federation_enable_disable() {
        let manager = FederationManager::new().await.unwrap();
        
        manager.set_federation_enabled(true).await.unwrap();
        assert!(manager.is_federation_enabled().await);
        
        manager.set_federation_enabled(false).await.unwrap();
        assert!(!manager.is_federation_enabled().await);
    }
    
    #[tokio::test]
    async fn test_federated_server_management() {
        let manager = FederationManager::new().await.unwrap();
        manager.set_federation_enabled(true).await.unwrap();
        
        let identity = Identity::generate();
        let public_key = identity.public_key();
        
        manager.add_federated_server(
            "example.com".to_string(),
            public_key,
            TrustLevel::Trusted
        ).await.unwrap();
        
        assert_eq!(manager.list_federated_servers().await.len(), 1);
        
        let info = manager.get_server_info("example.com").await.unwrap();
        assert_eq!(info.hostname, "example.com");
        
        manager.remove_federated_server("example.com").await.unwrap();
        assert_eq!(manager.list_federated_servers().await.len(), 0);
    }
}