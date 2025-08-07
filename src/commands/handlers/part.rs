use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_part(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    channels: Vec<String>,
    message: Option<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut responses = Vec::new();
    let mut state = server_state.write().await;
    
    // Get connection info
    let connection = state.connections.get(&connection_id)
        .ok_or("Connection not found")?
        .clone();
    
    let nick = connection.nickname.clone()
        .ok_or("No nickname set")?;
    let user = connection.username.clone()
        .unwrap_or(nick.clone());
    let host = connection.hostname.clone();
    
    let prefix = format!("{}!{}@{}", nick, user, host);
    
    for channel_name in channels {
        // Check if channel exists and user is in it
        if let Some(channel) = state.channels.get_mut(&channel_name) {
            if channel.members.contains_key(&connection_id) {
                // Remove user from channel
                channel.members.remove(&connection_id);
                
                // Create PART message
                let mut part_params = vec![channel_name.clone()];
                if let Some(msg) = &message {
                    part_params.push(msg.clone());
                }
                
                let part_msg = Message::new("PART")
                    .with_prefix(prefix.clone())
                    .with_params(part_params);
                
                // Send PART message to all remaining members
                for entry in channel.members.iter() {
                    let member_id = *entry.key();
                    if let Some(member_conn) = state.connections.get(&member_id) {
                        let _ = member_conn.tx.send(part_msg.clone()).await;
                    }
                }
                
                // Send PART confirmation to the user who left
                responses.push(part_msg);
                
                // If channel is empty, remove it
                if channel.members.is_empty() {
                    state.channels.remove(&channel_name);
                }
            } else {
                // User not in channel
                responses.push(Message::from(Reply::NotOnChannel {
                    nick: nick.clone(),
                    channel: channel_name.clone(),
                }));
            }
        } else {
            // Channel doesn't exist
            responses.push(Message::from(Reply::NoSuchChannel {
                nick: nick.clone(),
                channel: channel_name.clone(),
            }));
        }
    }
    
    Ok(responses)
}
