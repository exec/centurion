use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_who(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let state = server_state.read().await;
    let mut messages = Vec::new();
    
    // Get the requesting connection
    let requesting_connection = state.connections.get(&connection_id);
    if requesting_connection.is_none() {
        return Ok(messages);
    }
    let req_conn = requesting_connection.unwrap();
    let requester_nick = req_conn.nickname.as_deref().unwrap_or("*");
    
    // Default to "*" (all visible users) if no parameter given
    let target = params.get(0).cloned().unwrap_or_else(|| "*".to_string());
    
    if target.starts_with('#') || target.starts_with('&') || target.starts_with('!') {
        // WHO for a specific channel
        if let Some(channel) = state.channels.get(&target) {
            // Check if requesting user is in channel or channel is not secret/private
            let can_see_channel = channel.is_member(connection_id) || 
                                !channel.modes.contains(&'s') && !channel.modes.contains(&'p');
            
            if can_see_channel {
                // Return all visible members of this channel
                for member_entry in channel.members.iter() {
                    let member_id = *member_entry.key();
                    if let Some(member_conn) = state.connections.get(&member_id) {
                        if let Some(member_nick) = &member_conn.nickname {
                            // Determine status flags (H = here, G = gone/away, @ = op, + = voice)
                            // TODO: Implement AWAY support in Connection struct
                            let mut flags = "H".to_string(); // Always "here" for now
                            
                            // Add channel status (highest status first)
                            if member_entry.value().modes.contains(&'o') {
                                flags.push('@');
                            } else if member_entry.value().modes.contains(&'v') {
                                flags.push('+');
                            }
                            
                            // Create a basic WHO reply using available reply types
                            // Since WHO reply (352) isn't available, we'll document this limitation
                            // TODO: Implement proper RPL_WHOREPLY (352) when available in legion-protocol
                            // For now, we'll skip individual WHO replies and just send end of WHO
                        }
                    }
                }
            }
        }
    } else if target == "*" {
        // WHO for all visible users - very limited implementation
        // In a real implementation, this would show all visible users not in channels
        // For now, we'll just acknowledge the command
    } else {
        // WHO for a specific user
        if let Some(target_id_ref) = state.nicknames.get(&target.to_lowercase()) {
            let target_id = *target_id_ref;
            if let Some(target_conn) = state.connections.get(&target_id) {
                // TODO: Implement proper RPL_WHOREPLY (352) when available in legion-protocol
                // For now, we'll just acknowledge the user exists by not sending NoSuchNick
            } else {
                messages.push(Message::from(Reply::NoSuchNick {
                    nick: requester_nick.to_string(),
                    target: target.clone(),
                }));
            }
        } else {
            messages.push(Message::from(Reply::NoSuchNick {
                nick: requester_nick.to_string(),
                target: target.clone(),
            }));
        }
    }
    
    // Always send end of WHO
    // TODO: Replace with proper RPL_ENDOFWHO (315) when available in legion-protocol
    // For now, we'll create a basic message indicating the command completed
    let end_who_msg = Message::new("315")
        .with_params(vec![
            requester_nick.to_string(),
            target.clone(),
            "End of WHO list".to_string(),
        ]);
    messages.push(end_who_msg);
    
    Ok(messages)
}

#[cfg(test)]
#[path = "who_test.rs"]
mod tests;
