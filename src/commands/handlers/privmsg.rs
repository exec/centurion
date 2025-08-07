use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;
use crate::utils::generate_message_id;
use crate::history::{HistoryItem, MessageType};

pub async fn handle_privmsg(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    target: String,
    message: String,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    // Get sender info and prepare message  
    let (nick, user, host, has_echo_message, msg_id, privmsg) = {
        let state = server_state.read().await;
        
        let connection = state.connections.get(&connection_id)
            .ok_or("Connection not found")?;
        let nick = connection.nickname.clone()
            .ok_or("No nickname set")?;
        let user = connection.username.clone()
            .unwrap_or(nick.clone());
        let host = connection.hostname.clone();
        let has_echo_message = connection.capabilities.contains(&"echo-message".to_string());
        
        let prefix = format!("{}!{}@{}", nick, user, host);
        
        // Generate message ID for tracking reactions/replies
        let msg_id = generate_message_id();
        
        // Create PRIVMSG message with server-time and msgid
        let mut privmsg = Message::new("PRIVMSG")
            .with_prefix(prefix)
            .with_params(vec![target.clone(), message.clone()]);
            
        // Add msgid tag if client supports message-tags
        if connection.capabilities.contains(&"message-tags".to_string()) {
            privmsg = privmsg.with_tag("msgid".to_string(), Some(msg_id.clone()));
        }
        
        // Add server-time tag if client supports server-time
        if connection.capabilities.contains(&"server-time".to_string()) {
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            privmsg = privmsg.with_tag("time".to_string(), Some(timestamp));
        }

        (nick, user, host, has_echo_message, msg_id, privmsg)
    };

    // Store message in history
    let history_item = HistoryItem::new(
        msg_id.clone(),
        MessageType::Privmsg,
        nick.clone(),
        "*".to_string(), // TODO: Get actual account name
        message.clone(),
        target.clone(),
    );
    
    {
        let state = server_state.read().await;
        state.history.store_message(history_item);
    }
    
    // Send the message
    {
        let state = server_state.read().await;
        
        if target.starts_with('#') || target.starts_with('&') {
            // Channel message
            if let Some(channel) = state.channels.get(&target) {
                // Send to all channel members
                for entry in channel.members.iter() {
                    let member_id = *entry.key();
                    // Echo back to sender only if they have echo-message capability
                    if member_id != connection_id || has_echo_message {
                        if let Some(member_conn) = state.connections.get(&member_id) {
                            let _ = member_conn.tx.send(privmsg.clone()).await;
                        }
                    }
                }
            } else {
                // Channel doesn't exist or user not in channel
                return Ok(vec![Message::from(Reply::NoSuchChannel {
                    nick: nick.clone(),
                    channel: target.clone(),
                })]);
            }
        } else {
            // Private message to user
            let target_nick = target.clone();
            let mut target_connection = None;
            
            // Find target user
            for entry in state.connections.iter() {
                if let Some(ref nickname) = entry.nickname {
                    if nickname == &target_nick {
                        target_connection = Some(entry.clone());
                        break;
                    }
                }
            }
            
            if let Some(target_conn) = target_connection {
                // Send to target user
                let _ = target_conn.tx.send(privmsg.clone()).await;
                
                // Echo back to sender if they have echo-message capability
                if has_echo_message {
                    if let Some(sender_conn) = state.connections.get(&connection_id) {
                        let _ = sender_conn.tx.send(privmsg.clone()).await;
                    }
                }
            } else {
                // User not found
                return Ok(vec![Message::from(Reply::NoSuchNick {
                    nick: nick.clone(),
                    target: target_nick,
                })]);
            }
        }
    }
    
    Ok(vec![])  // No response to sender (unless error)
}
