use super::{HistoryItem, parse_timestamp};
use std::time::SystemTime;

/// Selector for history queries (timestamp or msgid)
#[derive(Debug, Clone)]
pub struct HistorySelector {
    pub msgid: Option<String>,
    pub timestamp: Option<SystemTime>,
}

impl HistorySelector {
    pub fn new() -> Self {
        Self {
            msgid: None,
            timestamp: None,
        }
    }

    pub fn with_msgid(msgid: String) -> Self {
        Self {
            msgid: Some(msgid),
            timestamp: None,
        }
    }

    pub fn with_timestamp(timestamp: SystemTime) -> Self {
        Self {
            msgid: None,
            timestamp: Some(timestamp),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.msgid.is_none() && self.timestamp.is_none()
    }
}

/// Different types of history queries
#[derive(Debug, Clone)]
pub enum HistoryQuery {
    /// Get messages before a selector
    Before {
        target: String,
        selector: HistorySelector,
        limit: usize,
    },
    /// Get messages after a selector  
    After {
        target: String,
        selector: HistorySelector,
        limit: usize,
    },
    /// Get latest messages
    Latest {
        target: String,
        selector: Option<HistorySelector>, // Optional end point
        limit: usize,
    },
    /// Get messages around a selector
    Around {
        target: String,
        selector: HistorySelector,
        limit: usize,
    },
    /// Get messages between two selectors
    Between {
        target: String,
        start: HistorySelector,
        end: HistorySelector,
        limit: usize,
    },
    /// List conversation targets
    Targets {
        start: Option<HistorySelector>,
        end: Option<HistorySelector>,
        limit: usize,
    },
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub messages: Vec<HistoryItem>,
    pub targets: Vec<(String, SystemTime)>,
    pub is_target_list: bool,
}

impl QueryResult {
    pub fn messages(messages: Vec<HistoryItem>) -> Self {
        Self {
            messages,
            targets: Vec::new(),
            is_target_list: false,
        }
    }

    pub fn targets(targets: Vec<(String, SystemTime)>) -> Self {
        Self {
            messages: Vec::new(),
            targets,
            is_target_list: true,
        }
    }
}

impl HistoryQuery {
    /// Parse a CHATHISTORY command into a query
    pub fn parse_chathistory_command(params: &[String]) -> Result<Self, String> {
        if params.len() < 2 {
            return Err("Not enough parameters".to_string());
        }

        let subcommand = params[0].to_lowercase();
        let target = params[1].clone();

        match subcommand.as_str() {
            "before" => {
                if params.len() < 3 {
                    return Err("BEFORE requires a selector".to_string());
                }
                let selector = parse_selector(&params[2])?;
                let limit = parse_limit(params.get(3), 100);
                
                Ok(HistoryQuery::Before {
                    target,
                    selector,
                    limit,
                })
            },
            "after" => {
                if params.len() < 3 {
                    return Err("AFTER requires a selector".to_string());
                }
                let selector = parse_selector(&params[2])?;
                let limit = parse_limit(params.get(3), 100);
                
                Ok(HistoryQuery::After {
                    target,
                    selector,
                    limit,
                })
            },
            "latest" => {
                let selector = if params.len() >= 3 && params[2] != "*" {
                    Some(parse_selector(&params[2])?)
                } else {
                    None
                };
                let limit = parse_limit(params.get(3), 100);
                
                Ok(HistoryQuery::Latest {
                    target,
                    selector,
                    limit,
                })
            },
            "around" => {
                if params.len() < 3 {
                    return Err("AROUND requires a selector".to_string());
                }
                let selector = parse_selector(&params[2])?;
                let limit = parse_limit(params.get(3), 100);
                
                Ok(HistoryQuery::Around {
                    target,
                    selector,
                    limit,
                })
            },
            "between" => {
                if params.len() < 4 {
                    return Err("BETWEEN requires two selectors".to_string());
                }
                let start = parse_selector(&params[2])?;
                let end = parse_selector(&params[3])?;
                let limit = parse_limit(params.get(4), 100);
                
                Ok(HistoryQuery::Between {
                    target,
                    start,
                    end,
                    limit,
                })
            },
            "targets" => {
                // For TARGETS, the target is actually the first selector
                let start = if target != "*" {
                    Some(parse_selector(&target)?)
                } else {
                    None
                };
                
                let end = if params.len() >= 3 && params[2] != "*" {
                    Some(parse_selector(&params[2])?)
                } else {
                    None
                };
                
                let limit = parse_limit(params.get(3), 100);
                
                Ok(HistoryQuery::Targets {
                    start,
                    end,
                    limit,
                })
            },
            _ => Err(format!("Unknown CHATHISTORY subcommand: {}", subcommand)),
        }
    }
}

/// Parse a selector parameter (timestamp=... or msgid=...)
fn parse_selector(param: &str) -> Result<HistorySelector, String> {
    if param == "*" {
        return Ok(HistorySelector::new());
    }

    if let Some(equals_pos) = param.find('=') {
        let (key, value) = param.split_at(equals_pos);
        let value = &value[1..]; // Skip the '=' character
        
        match key {
            "timestamp" => {
                let timestamp = parse_timestamp(value)
                    .map_err(|e| format!("Invalid timestamp: {}", e))?;
                Ok(HistorySelector::with_timestamp(timestamp))
            },
            "msgid" => {
                if value.is_empty() {
                    return Err("Empty msgid".to_string());
                }
                Ok(HistorySelector::with_msgid(value.to_string()))
            },
            _ => Err(format!("Unknown selector type: {}", key)),
        }
    } else {
        Err("Invalid selector format".to_string())
    }
}

/// Parse limit parameter
fn parse_limit(param: Option<&String>, default: usize) -> usize {
    param
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
        .min(500) // Cap at reasonable maximum
}