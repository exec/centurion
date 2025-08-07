use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;
use crate::history::{HistoryItem, HistoryQuery, QueryResult, MessageType};
use crate::commands::standard_replies::{StandardReply, common};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn handle_chathistory(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let state = server_state.read().await;
    
    // Get connection info
    let connection = state.connections.get(&connection_id)
        .ok_or("Connection not found")?;
    let nick = connection.nickname.clone()
        .ok_or("No nickname set")?;

    // Check if client has chathistory capability
    if !connection.capabilities.contains(&"chathistory".to_string()) {
        return Ok(vec![Message::from(Reply::UnknownCommand {
            nick,
            command: "CHATHISTORY".to_string(),
        })]);
    }

    // Parse query
    let query = match HistoryQuery::parse_chathistory_command(&params) {
        Ok(query) => query,
        Err(err) => {
            let state = server_state.read().await;
            return Ok(vec![
                common::invalid_params("CHATHISTORY", &err).to_message(&state.server_name)
            ]);
        }
    };

    // Check access permissions for target
    if let Some(target) = get_query_target(&query) {
        if !can_access_history(&state, connection_id, target).await {
            return Ok(vec![
                common::invalid_target("CHATHISTORY", target, "Messages could not be retrieved")
                    .to_message(&state.server_name)
            ]);
        }
    }

    // Execute query
    let result = execute_history_query(&state, &query).await?;
    
    // Build response messages
    let mut messages = Vec::new();
    
    if result.is_target_list {
        // TARGETS response
        messages.push(Message::new("BATCH")
            .with_prefix(state.server_name.clone())
            .with_params(vec!["+chathistory-targets".to_string(), "draft/chathistory-targets".to_string()]));
        
        for (target, timestamp) in result.targets {
            let timestamp_str = format_timestamp(timestamp);
            messages.push(Message::new("CHATHISTORY")
                .with_prefix(state.server_name.clone())
                .with_params(vec!["TARGETS".to_string(), target, timestamp_str]));
        }
        
        messages.push(Message::new("BATCH")
            .with_prefix(state.server_name.clone())
            .with_params(vec!["-chathistory-targets".to_string()]));
    } else {
        // Message history response
        if !result.messages.is_empty() {
            messages.push(Message::new("BATCH")
                .with_prefix(state.server_name.clone())
                .with_params(vec!["+history".to_string(), "chathistory".to_string(), get_query_target(&query).unwrap_or("*").to_string()]));
            
            for item in result.messages {
                let irc_msg = item.to_irc_message(&state.server_name);
                messages.push(irc_msg);
            }
            
            messages.push(Message::new("BATCH")
                .with_prefix(state.server_name.clone())
                .with_params(vec!["-history".to_string()]));
        }
    }

    Ok(messages)
}

async fn execute_history_query(
    state: &ServerState,
    query: &HistoryQuery,
) -> Result<QueryResult, Box<dyn std::error::Error>> {
    match query {
        HistoryQuery::Before { target, selector, limit } => {
            let end_time = get_time_from_selector(state, target, selector).await;
            let messages = state.history.get_messages_between(
                target, None, end_time, *limit, false
            );
            Ok(QueryResult::messages(messages))
        },
        HistoryQuery::After { target, selector, limit } => {
            let start_time = get_time_from_selector(state, target, selector).await;
            let messages = state.history.get_messages_between(
                target, start_time, None, *limit, true
            );
            Ok(QueryResult::messages(messages))
        },
        HistoryQuery::Latest { target, selector, limit } => {
            let end_time = if let Some(sel) = selector {
                get_time_from_selector(state, target, sel).await
            } else {
                None
            };
            let messages = state.history.get_messages_between(
                target, None, end_time, *limit, false
            );
            Ok(QueryResult::messages(messages))
        },
        HistoryQuery::Around { target, selector, limit } => {
            let center_time = get_time_from_selector(state, target, selector).await
                .unwrap_or_else(SystemTime::now);
            let center_msgid = selector.msgid.as_deref();
            let messages = state.history.get_messages_around(
                target, center_time, center_msgid, *limit
            );
            Ok(QueryResult::messages(messages))
        },
        HistoryQuery::Between { target, start, end, limit } => {
            let start_time = get_time_from_selector(state, target, start).await;
            let end_time = get_time_from_selector(state, target, end).await;
            
            // Determine if we're going forwards or backwards
            let ascending = match (start_time, end_time) {
                (Some(s), Some(e)) => s < e,
                (Some(_), None) => true,  // AFTER
                (None, Some(_)) => false, // BEFORE
                (None, None) => false,    // LATEST
            };
            
            let messages = state.history.get_messages_between(
                target, start_time, end_time, *limit, ascending
            );
            Ok(QueryResult::messages(messages))
        },
        HistoryQuery::Targets { start: _, end: _, limit } => {
            // For now, return all targets for the user
            // TODO: Implement proper filtering by start/end selectors
            let targets = state.history.get_targets_for_user("*");
            let limited_targets: Vec<_> = targets.into_iter().take(*limit).collect();
            Ok(QueryResult::targets(limited_targets))
        },
    }
}

async fn get_time_from_selector(
    state: &ServerState,
    target: &str,
    selector: &crate::history::queries::HistorySelector,
) -> Option<SystemTime> {
    if let Some(timestamp) = selector.timestamp {
        Some(timestamp)
    } else if let Some(msgid) = &selector.msgid {
        // Look up message by ID to get its timestamp
        if let Some(item) = state.history.find_message_by_id(target, msgid) {
            Some(item.timestamp)
        } else {
            None
        }
    } else {
        None
    }
}

async fn can_access_history(
    state: &ServerState,
    connection_id: u64,
    target: &str,
) -> bool {
    if target.starts_with('#') || target.starts_with('&') {
        // Channel history - check if user is in channel
        if let Some(channel) = state.channels.get(target) {
            channel.members.contains_key(&connection_id)
        } else {
            false
        }
    } else {
        // Private message history - always allowed for now
        // TODO: Add proper access control for DM history
        true
    }
}

fn get_query_target(query: &HistoryQuery) -> Option<&str> {
    match query {
        HistoryQuery::Before { target, .. } => Some(target),
        HistoryQuery::After { target, .. } => Some(target),
        HistoryQuery::Latest { target, .. } => Some(target),
        HistoryQuery::Around { target, .. } => Some(target),
        HistoryQuery::Between { target, .. } => Some(target),
        HistoryQuery::Targets { .. } => None,
    }
}


fn format_timestamp(timestamp: SystemTime) -> String {
    let since_epoch = timestamp.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since_epoch.as_secs();
    let nanos = since_epoch.subsec_nanos();
    
    // Convert to RFC3339 format with milliseconds
    let datetime = chrono::DateTime::from_timestamp(secs as i64, nanos).unwrap();
    format!("timestamp={}", datetime.format("%Y-%m-%dT%H:%M:%S.%3fZ"))
}