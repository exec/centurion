use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use crate::protocol::{Message, Reply};
use crate::state::ServerState;
use crate::commands::standard_replies::common;

/// Handle NICK command - change or set nickname
pub async fn handle_nick(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    new_nick: String,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    debug!("Processing NICK command for connection {}: {}", connection_id, new_nick);
    
    let mut state = server_state.write().await;
    let mut responses = Vec::new();
    
    // Get connection info
    let mut connection = match state.connections.get_mut(&connection_id) {
        Some(conn) => conn,
        None => {
            warn!("NICK: Connection {} not found", connection_id);
            return Ok(responses);
        }
    };
    
    // Validate nickname
    if new_nick.is_empty() || new_nick.len() > 30 {
        responses.push(common::invalid_nickname("NICK", &new_nick).to_message(&state.server_name));
        return Ok(responses);
    }
    
    // Check for invalid characters (basic validation)
    if new_nick.contains(' ') || new_nick.contains('\r') || new_nick.contains('\n') || 
       new_nick.starts_with('#') || new_nick.starts_with('&') || new_nick.starts_with(':') {
        responses.push(common::invalid_nickname("NICK", &new_nick).to_message(&state.server_name));
        return Ok(responses);
    }
    
    // Check if nickname is already in use by another connection
    for entry in state.connections.iter() {
        let other_id = *entry.key();
        let other_conn = entry.value();
        if other_id != connection_id {
            if let Some(ref other_nick) = other_conn.nickname {
                if other_nick.eq_ignore_ascii_case(&new_nick) {
                    responses.push(common::nickname_in_use("NICK", &new_nick).to_message(&state.server_name));
                    return Ok(responses);
                }
            }
        }
    }
    
    let old_nick = connection.nickname.clone();
    let is_new_user = old_nick.is_none();
    
    // Set the new nickname
    connection.nickname = Some(new_nick.clone());
    
    if is_new_user {
        // This is initial nickname setting during registration
        debug!("Set initial nickname for connection {}: {}", connection_id, new_nick);
    } else {
        // This is a nickname change - notify all channels the user is in
        let old_nick = old_nick.unwrap();
        let hostmask = format!("{}!{}@{}", old_nick, 
                              connection.username.as_deref().unwrap_or("unknown"),
                              connection.addr);
        
        let nick_change_message = Message {
            tags: std::collections::HashMap::new(),
            prefix: Some(hostmask),
            command: "NICK".to_string(),
            params: vec![new_nick.clone()],
        };
        
        // Find all channels this user is in
        let mut channels_to_notify = Vec::new();
        for entry in state.channels.iter() {
            let channel = entry.value();
            if channel.members.contains_key(&connection_id) {
                channels_to_notify.push(channel.name.clone());
            }
        }
        
        // Send NICK message to all users in those channels (including the user themselves)
        let mut notified_connections = std::collections::HashSet::new();
        
        for channel_name in channels_to_notify {
            if let Some(channel) = state.channels.get(&channel_name) {
                for entry in channel.members.iter() {
                    let member_id = *entry.key();
                    if !notified_connections.contains(&member_id) {
                        notified_connections.insert(member_id);
                        
                        // Send to this member (including the user who changed nick)
                        responses.push(nick_change_message.clone());
                    }
                }
            }
        }
        
        debug!("Nickname changed from {} to {} for connection {}", old_nick, new_nick, connection_id);
    }
    
    Ok(responses)
}