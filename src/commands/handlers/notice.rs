use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;
use crate::utils::generate_message_id;
use crate::history::{HistoryItem, MessageType};

pub async fn handle_notice(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    if params.len() < 2 {
        return Ok(vec![]); // NOTICE doesn't send error responses
    }
    
    let target = params[0].clone();
    let message = params[1].clone();
    
    // Get sender info and prepare message  
    let (nick, user, host, has_echo_message, msg_id, notice_msg) = {
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
        
        // Generate message ID for tracking
        let msg_id = generate_message_id();
        
        // Create NOTICE message
        let notice_msg = Message::new("NOTICE")
            .with_prefix(prefix)
            .with_params(vec![target.clone(), message.clone()])
            .with_tag("msgid".to_string(), Some(msg_id.clone()));

        (nick, user, host, has_echo_message, msg_id, notice_msg)
    };

    // Store message in history
    let history_item = HistoryItem::new(
        msg_id.clone(),
        MessageType::Notice,
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
            // Channel notice
            if let Some(channel) = state.channels.get(&target) {
                // Send to all channel members
                for entry in channel.members.iter() {
                    let member_id = *entry.key();
                    // Echo back to sender only if they have echo-message capability
                    if member_id != connection_id || has_echo_message {
                        if let Some(member_conn) = state.connections.get(&member_id) {
                            let _ = member_conn.tx.send(notice_msg.clone()).await;
                        }
                    }
                }
            }
            // Don't send error for nonexistent channels with NOTICE
        } else {
            // Private notice to user
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
                let _ = target_conn.tx.send(notice_msg.clone()).await;
                
                // Echo back to sender if they have echo-message capability
                if has_echo_message {
                    if let Some(sender_conn) = state.connections.get(&connection_id) {
                        let _ = sender_conn.tx.send(notice_msg.clone()).await;
                    }
                }
            }
            // Don't send error for nonexistent users with NOTICE
        }
    }
    
    Ok(vec![])  // NOTICE never sends responses
}
