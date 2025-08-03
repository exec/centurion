use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use governor::{Quota, RateLimiter as GovernorRateLimiter, state::direct::NotKeyed};
use parking_lot::RwLock;
use thiserror::Error;

pub mod auth;
pub mod tls;
pub mod validation;

pub use self::auth::{AuthMethod, SaslMechanism, authenticate};
pub use self::validation::{validate_nickname, validate_channel_name, validate_message};

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Connection limit exceeded")]
    ConnectionLimitExceeded,
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Access denied")]
    AccessDenied,
    
    #[error("Banned")]
    Banned,
}

pub struct RateLimiter {
    quota: Quota,
    limiter: GovernorRateLimiter<NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>,
}

impl RateLimiter {
    pub fn new(max_requests: u32, per_duration: Duration) -> Self {
        let quota = Quota::with_period(per_duration)
            .unwrap()
            .allow_burst(std::num::NonZeroU32::new(max_requests).unwrap());
        
        Self {
            quota,
            limiter: GovernorRateLimiter::direct(quota),
        }
    }
    
    pub async fn check(&self) -> bool {
        self.limiter.check().is_ok()
    }
    
    pub async fn check_key(&self, _key: &str) -> bool {
        self.limiter.check().is_ok()
    }
}

pub struct ConnectionLimiter {
    limits: Arc<RwLock<ConnectionLimits>>,
}

struct ConnectionLimits {
    per_ip: HashMap<IpAddr, usize>,
    global_count: usize,
    max_per_ip: usize,
    max_global: usize,
}

impl ConnectionLimiter {
    pub fn new(max_per_ip: usize, max_global: usize) -> Self {
        Self {
            limits: Arc::new(RwLock::new(ConnectionLimits {
                per_ip: HashMap::new(),
                global_count: 0,
                max_per_ip,
                max_global,
            })),
        }
    }
    
    pub fn check_and_add(&self, addr: SocketAddr) -> Result<(), SecurityError> {
        let mut limits = self.limits.write();
        
        if limits.global_count >= limits.max_global {
            return Err(SecurityError::ConnectionLimitExceeded);
        }
        
        let ip = addr.ip();
        let max_per_ip = limits.max_per_ip;
        let count = limits.per_ip.entry(ip).or_insert(0);
        
        if *count >= max_per_ip {
            return Err(SecurityError::ConnectionLimitExceeded);
        }
        
        *count += 1;
        limits.global_count += 1;
        
        Ok(())
    }
    
    pub fn remove(&self, addr: SocketAddr) {
        let mut limits = self.limits.write();
        let ip = addr.ip();
        
        if let Some(count) = limits.per_ip.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                limits.per_ip.remove(&ip);
            }
        }
        
        limits.global_count = limits.global_count.saturating_sub(1);
    }
}

pub struct FloodProtection {
    message_times: Arc<RwLock<HashMap<u64, Vec<Instant>>>>,
    max_messages: usize,
    time_window: Duration,
}

impl FloodProtection {
    pub fn new(max_messages: usize, time_window: Duration) -> Self {
        Self {
            message_times: Arc::new(RwLock::new(HashMap::new())),
            max_messages,
            time_window,
        }
    }
    
    pub fn check_flood(&self, connection_id: u64) -> bool {
        let mut times = self.message_times.write();
        let now = Instant::now();
        
        let messages = times.entry(connection_id).or_insert_with(Vec::new);
        
        // Remove old messages outside the time window
        messages.retain(|&time| now.duration_since(time) < self.time_window);
        
        if messages.len() >= self.max_messages {
            return false;
        }
        
        messages.push(now);
        true
    }
    
    pub fn clear(&self, connection_id: u64) {
        self.message_times.write().remove(&connection_id);
    }
}

pub struct BanManager {
    bans: Arc<RwLock<HashMap<String, Vec<BanEntry>>>>,
}

#[derive(Clone)]
pub struct BanEntry {
    pub mask: String,
    pub reason: Option<String>,
    pub set_by: String,
    pub expires_at: Option<Instant>,
}

impl BanManager {
    pub fn new() -> Self {
        Self {
            bans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn add_ban(&self, channel: &str, ban: BanEntry) {
        self.bans.write()
            .entry(channel.to_string())
            .or_insert_with(Vec::new)
            .push(ban);
    }
    
    pub fn remove_ban(&self, channel: &str, mask: &str) -> bool {
        if let Some(bans) = self.bans.write().get_mut(channel) {
            if let Some(pos) = bans.iter().position(|b| b.mask == mask) {
                bans.remove(pos);
                return true;
            }
        }
        false
    }
    
    pub fn is_banned(&self, channel: &str, user_mask: &str) -> bool {
        let bans = self.bans.read();
        if let Some(channel_bans) = bans.get(channel) {
            let now = Instant::now();
            for ban in channel_bans {
                if let Some(expires) = ban.expires_at {
                    if now > expires {
                        continue;
                    }
                }
                if mask_matches(&ban.mask, user_mask) {
                    return true;
                }
            }
        }
        false
    }
    
    pub fn get_bans(&self, channel: &str) -> Vec<BanEntry> {
        self.bans.read()
            .get(channel)
            .cloned()
            .unwrap_or_default()
    }
}

fn mask_matches(pattern: &str, text: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    
    let mut dp = vec![vec![false; text_chars.len() + 1]; pattern_chars.len() + 1];
    dp[0][0] = true;
    
    for i in 1..=pattern_chars.len() {
        if pattern_chars[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    
    for i in 1..=pattern_chars.len() {
        for j in 1..=text_chars.len() {
            match pattern_chars[i - 1] {
                '*' => {
                    dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
                }
                '?' => {
                    dp[i][j] = dp[i - 1][j - 1];
                }
                c => {
                    dp[i][j] = dp[i - 1][j - 1] && c.to_ascii_lowercase() == text_chars[j - 1].to_ascii_lowercase();
                }
            }
        }
    }
    
    dp[pattern_chars.len()][text_chars.len()]
}