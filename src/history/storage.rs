use super::{HistoryItem, MessageType, format_timestamp};
use std::collections::{BTreeMap, VecDeque};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Configuration for history storage
#[derive(Debug, Clone)]
pub struct HistoryConfig {
    /// Maximum number of messages to store per target
    pub max_messages_per_target: usize,
    /// Maximum age of messages to keep
    pub max_age: Duration,
    /// Whether to store join/part messages
    pub store_joins: bool,
    /// Whether to store mode changes
    pub store_modes: bool,
    /// Whether to store nick changes
    pub store_nicks: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_messages_per_target: 1000,
            max_age: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            store_joins: true,
            store_modes: true,
            store_nicks: true,
        }
    }
}

/// Ring buffer for storing message history for a specific target
#[derive(Debug)]
struct HistoryBuffer {
    messages: VecDeque<HistoryItem>,
    max_size: usize,
    max_age: Duration,
}

impl HistoryBuffer {
    fn new(config: &HistoryConfig) -> Self {
        Self {
            messages: VecDeque::with_capacity(config.max_messages_per_target),
            max_size: config.max_messages_per_target,
            max_age: config.max_age,
        }
    }

    /// Add a message to the buffer
    fn add_message(&mut self, item: HistoryItem) {
        // Remove old messages first
        self.cleanup_old_messages();

        // Add new message
        self.messages.push_back(item);

        // Enforce size limit
        while self.messages.len() > self.max_size {
            self.messages.pop_front();
        }
    }

    /// Remove messages older than max_age
    fn cleanup_old_messages(&mut self) {
        let cutoff = SystemTime::now()
            .checked_sub(self.max_age)
            .unwrap_or(UNIX_EPOCH);

        while let Some(front) = self.messages.front() {
            if front.timestamp < cutoff {
                self.messages.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get messages between two timestamps
    fn get_messages_between(
        &self,
        start: Option<SystemTime>,
        end: Option<SystemTime>,
        limit: usize,
        ascending: bool,
    ) -> Vec<HistoryItem> {
        let mut results = Vec::new();
        
        for item in &self.messages {
            // Check time bounds
            if let Some(start_time) = start {
                if item.timestamp <= start_time {
                    continue;
                }
            }
            if let Some(end_time) = end {
                if item.timestamp >= end_time {
                    continue;
                }
            }
            
            results.push(item.clone());
        }

        // Sort by timestamp
        if ascending {
            results.sort_by_key(|item| item.timestamp);
        } else {
            results.sort_by_key(|item| std::cmp::Reverse(item.timestamp));
        }

        // Apply limit
        results.truncate(limit);
        results
    }

    /// Get messages around a specific timestamp or msgid
    fn get_messages_around(
        &self,
        center_time: SystemTime,
        center_msgid: Option<&str>,
        limit: usize,
    ) -> Vec<HistoryItem> {
        let mut before = Vec::new();
        let mut after = Vec::new();

        for item in &self.messages {
            // If looking for specific msgid, skip the center message
            if let Some(msgid) = center_msgid {
                if item.msgid == msgid {
                    continue;
                }
            } else if item.timestamp == center_time {
                continue;
            }

            if item.timestamp < center_time {
                before.push(item.clone());
            } else if item.timestamp > center_time {
                after.push(item.clone());
            }
        }

        // Sort before messages (newest first) and after messages (oldest first)
        before.sort_by_key(|item| std::cmp::Reverse(item.timestamp));
        after.sort_by_key(|item| item.timestamp);

        // Take half the limit from each side
        let half_limit = limit / 2;
        before.truncate(half_limit);
        after.truncate(limit - before.len());

        // Combine and sort by timestamp
        let mut result = before;
        result.extend(after);
        result.sort_by_key(|item| item.timestamp);

        result
    }

    /// Find message by msgid
    fn find_by_msgid(&self, msgid: &str) -> Option<&HistoryItem> {
        self.messages.iter().find(|item| item.msgid == msgid)
    }

    /// Get all message targets (for TARGETS subcommand)
    fn get_targets(&self) -> Vec<(String, SystemTime)> {
        let mut targets = BTreeMap::new();
        
        for item in &self.messages {
            // For DMs, use correspondent; for channels, use target
            let target_name = item.correspondent.as_ref().unwrap_or(&item.target).clone();
            
            // Keep the latest timestamp for each target
            targets.entry(target_name)
                .and_modify(|timestamp| {
                    if item.timestamp > *timestamp {
                        *timestamp = item.timestamp;
                    }
                })
                .or_insert(item.timestamp);
        }

        targets.into_iter().collect()
    }
}

/// Main history storage manager
pub struct HistoryStorage {
    /// Buffers for each target (channel or user)
    buffers: RwLock<BTreeMap<String, HistoryBuffer>>,
    config: HistoryConfig,
}

impl HistoryStorage {
    pub fn new(config: HistoryConfig) -> Self {
        Self {
            buffers: RwLock::new(BTreeMap::new()),
            config,
        }
    }

    /// Store a message in history
    pub fn store_message(&self, item: HistoryItem) {
        // Skip certain message types if configured
        match item.message_type {
            MessageType::Join | MessageType::Part | MessageType::Quit if !self.config.store_joins => return,
            MessageType::Mode if !self.config.store_modes => return,
            MessageType::Nick if !self.config.store_nicks => return,
            _ => {}
        }

        let target = item.target.clone();
        
        let mut buffers = self.buffers.write().unwrap();
        let buffer = buffers.entry(target).or_insert_with(|| HistoryBuffer::new(&self.config));
        buffer.add_message(item);
    }

    /// Get messages for a target between two points
    pub fn get_messages_between(
        &self,
        target: &str,
        start: Option<SystemTime>,
        end: Option<SystemTime>,
        limit: usize,
        ascending: bool,
    ) -> Vec<HistoryItem> {
        let buffers = self.buffers.read().unwrap();
        
        if let Some(buffer) = buffers.get(target) {
            buffer.get_messages_between(start, end, limit, ascending)
        } else {
            Vec::new()
        }
    }

    /// Get messages around a specific point
    pub fn get_messages_around(
        &self,
        target: &str,
        center_time: SystemTime,
        center_msgid: Option<&str>,
        limit: usize,
    ) -> Vec<HistoryItem> {
        let buffers = self.buffers.read().unwrap();
        
        if let Some(buffer) = buffers.get(target) {
            buffer.get_messages_around(center_time, center_msgid, limit)
        } else {
            Vec::new()
        }
    }

    /// Find message by ID
    pub fn find_message_by_id(&self, target: &str, msgid: &str) -> Option<HistoryItem> {
        let buffers = self.buffers.read().unwrap();
        
        if let Some(buffer) = buffers.get(target) {
            buffer.find_by_msgid(msgid).cloned()
        } else {
            None
        }
    }

    /// Get conversation targets for a user (for TARGETS subcommand)
    pub fn get_targets_for_user(&self, _user: &str) -> Vec<(String, SystemTime)> {
        let buffers = self.buffers.read().unwrap();
        let mut all_targets = Vec::new();
        
        for buffer in buffers.values() {
            all_targets.extend(buffer.get_targets());
        }

        // Sort by most recent activity
        all_targets.sort_by_key(|(_, timestamp)| std::cmp::Reverse(*timestamp));
        
        // Remove duplicates (keep most recent)
        let mut seen = std::collections::HashSet::new();
        all_targets.retain(|(name, _)| seen.insert(name.clone()));
        
        all_targets
    }

    /// Clean up old messages across all buffers
    pub fn cleanup_old_messages(&self) {
        let mut buffers = self.buffers.write().unwrap();
        for buffer in buffers.values_mut() {
            buffer.cleanup_old_messages();
        }
    }
}

impl Default for HistoryStorage {
    fn default() -> Self {
        Self::new(HistoryConfig::default())
    }
}