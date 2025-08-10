use crate::protocol::Message;
use crate::state::ServerState;
use crate::utils::generate_message_id;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Helper function to add server tags to a message
fn add_server_tags(mut message: Message, connection_capabilities: &Vec<String>) -> Message {
    // Add msgid tag if client supports message-tags
    if connection_capabilities.contains(&"message-tags".to_string()) {
        let msg_id = generate_message_id();
        message = message.with_tag("msgid".to_string(), Some(msg_id));
    }

    // Add server-time tag if client supports server-time
    if connection_capabilities.contains(&"server-time".to_string()) {
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        message = message.with_tag("time".to_string(), Some(timestamp));
    }

    message
}

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
                        // Build TAGMSG message for this specific client
                        let mut tagmsg = Message::new("TAGMSG")
                            .with_prefix(sender_prefix.clone())
                            .with_params(vec![target.clone()]);
                        
                        // Add server tags based on client capabilities
                        tagmsg = add_server_tags(tagmsg, &conn.capabilities);
                        
                        // Add all client tags
                        for (key, value) in &tags {
                            tagmsg = tagmsg.with_tag(key.clone(), Some(value.clone()));
                        }
                        
                        let _ = conn.tx.send(tagmsg).await;
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
                // Get target connection for server tags
                let target_capabilities = {
                    let state = server_state.read().await;
                    state.connections.get(&target_id).map(|conn| conn.capabilities.clone())
                };
                
                if let Some(capabilities) = target_capabilities {
                    let mut tagmsg = Message::new("TAGMSG")
                        .with_prefix(sender_prefix.clone())
                        .with_params(vec![target.clone()]);
                    
                    // Add server tags based on target capabilities
                    tagmsg = add_server_tags(tagmsg, &capabilities);
                    
                    // Add all client tags
                    for (key, value) in &tags {
                        tagmsg = tagmsg.with_tag(key.clone(), Some(value.clone()));
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
                    let sender_echo_info = {
                        let state = server_state.read().await;
                        state.connections.get(&sender_id).map(|conn| (
                            conn.capabilities.contains(&"echo-message".to_string()),
                            conn.capabilities.clone(),
                            conn.tx.clone()
                        ))
                    };
                    
                    if let Some((has_echo, sender_caps, sender_tx)) = sender_echo_info {
                        if has_echo {
                            // Build echo message with sender's capabilities
                            let mut echo_tagmsg = Message::new("TAGMSG")
                                .with_prefix(sender_prefix)
                                .with_params(vec![target]);
                            
                            // Add server tags for sender
                            echo_tagmsg = add_server_tags(echo_tagmsg, &sender_caps);
                            
                            // Add all client tags
                            for (key, value) in &tags {
                                echo_tagmsg = echo_tagmsg.with_tag(key.clone(), Some(value.clone()));
                            }
                            
                            let _ = sender_tx.send(echo_tagmsg).await;
                        }
                    }
                }
            }
        }
    }
    
    Ok(responses)
}