use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_list(
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
    
    // Parse parameters - LIST [channels] [target]
    let channel_filter = params.get(0).cloned();
    
    // Create the list start message
    // TODO: Replace with proper RPL_LISTSTART when available in legion-protocol
    let list_start_msg = Message::new("321")
        .with_params(vec![
            requester_nick.to_string(),
            "Channel".to_string(),
            "Users Name".to_string(),
        ]);
    messages.push(list_start_msg);
    
    // List channels based on filter
    if let Some(ref filter) = channel_filter {
        // List specific channel(s)
        let channel_names: Vec<&str> = filter.split(',').collect();
        for channel_name in channel_names {
            if let Some(channel) = state.channels.get(channel_name) {
                // Check if channel is visible (not secret/private or user is a member)
                let can_see = !channel.modes.contains(&'s') && !channel.modes.contains(&'p') 
                            || channel.is_member(connection_id);
                
                if can_see {
                    let topic = channel.topic.as_deref().unwrap_or("");
                    let member_count = channel.member_count();
                    
                    // TODO: Replace with proper RPL_LIST when available in legion-protocol
                    let list_msg = Message::new("322")
                        .with_params(vec![
                            requester_nick.to_string(),
                            channel_name.to_string(),
                            member_count.to_string(),
                            topic.to_string(),
                        ]);
                    messages.push(list_msg);
                }
            }
        }
    } else {
        // List all visible channels
        for channel_entry in state.channels.iter() {
            let (channel_name, channel) = channel_entry.pair();
            
            // Check if channel is visible
            let can_see = !channel.modes.contains(&'s') && !channel.modes.contains(&'p') 
                        || channel.is_member(connection_id);
            
            if can_see {
                let topic = channel.topic.as_deref().unwrap_or("");
                let member_count = channel.member_count();
                
                // TODO: Replace with proper RPL_LIST when available in legion-protocol
                let list_msg = Message::new("322")
                    .with_params(vec![
                        requester_nick.to_string(),
                        channel_name.clone(),
                        member_count.to_string(),
                        topic.to_string(),
                    ]);
                messages.push(list_msg);
            }
        }
    }
    
    // End of list
    // TODO: Replace with proper RPL_LISTEND when available in legion-protocol
    let list_end_msg = Message::new("323")
        .with_params(vec![
            requester_nick.to_string(),
            "End of LIST".to_string(),
        ]);
    messages.push(list_end_msg);
    
    Ok(messages)
}

#[cfg(test)]
#[path = "list_test.rs"]
mod tests;
