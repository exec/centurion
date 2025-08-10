use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_names(
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
    
    // Parse parameters - NAMES [channels] [target]
    let channels_to_list = if params.is_empty() {
        // If no parameters, list names for all channels the user is in
        let mut user_channels = Vec::new();
        for channel_entry in state.channels.iter() {
            let (channel_name, channel) = channel_entry.pair();
            if channel.is_member(connection_id) {
                user_channels.push(channel_name.clone());
            }
        }
        user_channels
    } else {
        // Split comma-separated channel list
        params[0].split(',').map(|s| s.to_string()).collect()
    };
    
    // Generate NAMES reply for each channel
    for channel_name in &channels_to_list {
        if let Some(channel) = state.channels.get(channel_name) {
            // Check if user can see channel names
            let can_see_names = channel.is_member(connection_id) || 
                              (!channel.modes.contains(&'s') && !channel.modes.contains(&'p'));
            
            if can_see_names {
                let mut names = Vec::new();
                
                // Collect member names with status prefixes
                for member_entry in channel.members.iter() {
                    let member_id = *member_entry.key();
                    if let Some(member_conn) = state.connections.get(&member_id) {
                        if let Some(member_nick) = &member_conn.nickname {
                            let mut name_with_prefix = String::new();
                            
                            // Add status prefixes (highest status first)
                            if member_entry.value().modes.contains(&'o') {
                                name_with_prefix.push('@');
                            } else if member_entry.value().modes.contains(&'v') {
                                name_with_prefix.push('+');
                            }
                            
                            name_with_prefix.push_str(member_nick);
                            names.push(name_with_prefix);
                        }
                    }
                }
                
                // Determine channel symbol (= public, @ secret, * private)
                let symbol = if channel.modes.contains(&'s') {
                    '@'  // Secret channel
                } else if channel.modes.contains(&'p') {
                    '*'  // Private channel
                } else {
                    '='  // Public channel
                };
                
                // Send NAMES reply
                messages.push(Message::from(Reply::NamReply {
                    nick: requester_nick.to_string(),
                    symbol,
                    channel: channel_name.clone(),
                    names,
                }));
            }
        }
        
        // Always send END OF NAMES, even for non-existent channels
        messages.push(Message::from(Reply::EndOfNames {
            nick: requester_nick.to_string(),
            channel: channel_name.clone(),
        }));
    }
    
    // If no channels were specified and user is not in any channels,
    // still need to send end of names for the implicit query
    if params.is_empty() && channels_to_list.is_empty() {
        messages.push(Message::from(Reply::EndOfNames {
            nick: requester_nick.to_string(),
            channel: "*".to_string(),
        }));
    }
    
    Ok(messages)
}
