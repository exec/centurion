use crate::protocol::Message;
use crate::state::ServerState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle_tagmsg(
    server_state: Arc<RwLock<ServerState>>,
    sender_id: u64,
    target: String,
    tags: HashMap<String, String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let responses = Vec::new();
    
    // Get sender info
    let sender_info = {
        let state = server_state.read().await;
        state.connections.get(&sender_id).map(|conn| (
            conn.nickname.clone().unwrap_or_else(|| "*".to_string()),
            conn.username.clone().unwrap_or_else(|| "~unknown".to_string()),
            conn.hostname.clone(),
        ))
    };

    let (sender_nick, sender_user, sender_host) = match sender_info {
        Some(info) => info,
        None => return Ok(responses),
    };
    
    let sender_prefix = format!("{}!{}@{}", sender_nick, sender_user, sender_host);
    
    // Check if target is a channel or user
    if target.starts_with('#') || target.starts_with('&') {
        // Handle channel TAGMSG
        let channel_member_ids = {
            let state = server_state.read().await;
            state.channels.get(&target).map(|channel| {
                channel.members.iter().map(|member| *member.key()).collect::<Vec<u64>>()
            })
        };
        
        if let Some(member_ids) = channel_member_ids {
            // Build TAGMSG message
            let mut tagmsg = Message::new("TAGMSG")
                .with_prefix(sender_prefix.clone())
                .with_params(vec![target.clone()]);
            
            // Add all tags
            for (key, value) in tags {
                tagmsg = tagmsg.with_tag(key, Some(value));
            }
            
            // Check if sender has echo-message capability
            let sender_has_echo = {
                let state = server_state.read().await;
                state.connections.get(&sender_id)
                    .map(|conn| conn.capabilities.contains(&"echo-message".to_string()))
                    .unwrap_or(false)
            };
            
            // Send to all channel members
            let state = server_state.read().await;
            for member_id in member_ids {
                // Echo back to sender only if they have echo-message capability
                if member_id == sender_id && !sender_has_echo {
                    continue;
                }
                
                if let Some(conn) = state.connections.get(&member_id) {
                    // Send TAGMSG to all clients that support message-tags
                    if conn.capabilities.contains(&"message-tags".to_string()) {
                        let _ = conn.tx.send(tagmsg.clone()).await;
                    }
                }
            }
        }
    } else {
        // Handle private TAGMSG
        let target_id = {
            let state = server_state.read().await;
            state.nicknames.get(&target.to_lowercase()).map(|entry| *entry.value())
        };
        
        if let Some(target_id) = target_id {
            // Check if target has message-tags capability
            let has_capability = {
                let state = server_state.read().await;
                state.connections.get(&target_id)
                    .map(|conn| conn.capabilities.contains(&"message-tags".to_string()))
                    .unwrap_or(false)
            };
            
            if has_capability {
                let mut tagmsg = Message::new("TAGMSG")
                    .with_prefix(sender_prefix)
                    .with_params(vec![target.clone()]);
                
                // Add all tags
                for (key, value) in tags {
                    tagmsg = tagmsg.with_tag(key, Some(value));
                }
                
                // Send the message
                let tx = {
                    let state = server_state.read().await;
                    state.connections.get(&target_id).map(|conn| conn.tx.clone())
                };
                
                if let Some(tx) = tx {
                    let _ = tx.send(tagmsg.clone()).await;
                }
                
                // Echo back to sender if they have echo-message capability
                let sender_has_echo = {
                    let state = server_state.read().await;
                    state.connections.get(&sender_id)
                        .map(|conn| conn.capabilities.contains(&"echo-message".to_string()))
                        .unwrap_or(false)
                };
                
                if sender_has_echo {
                    let sender_tx = {
                        let state = server_state.read().await;
                        state.connections.get(&sender_id).map(|conn| conn.tx.clone())
                    };
                    
                    if let Some(tx) = sender_tx {
                        let _ = tx.send(tagmsg).await;
                    }
                }
            }
        }
    }
    
    Ok(responses)
}