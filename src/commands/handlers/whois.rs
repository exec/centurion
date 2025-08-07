use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_whois(
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
    
    // Handle each target nickname
    for target_nick in &params {
        if let Some(target_id_ref) = state.nicknames.get(&target_nick.to_lowercase()) {
            let target_id = *target_id_ref;
            if let Some(target_conn) = state.connections.get(&target_id) {
                // Send WHOIS user information (311)
                messages.push(Message::from(Reply::WhoisUser {
                    nick: requester_nick.to_string(),
                    target: target_nick.clone(),
                    username: target_conn.username.as_deref().unwrap_or("*").to_string(),
                    host: target_conn.hostname.clone(),
                    realname: target_conn.realname.as_deref().unwrap_or("Unknown").to_string(),
                }));
                
                // Send server information (312)  
                messages.push(Message::from(Reply::WhoisServer {
                    nick: requester_nick.to_string(),
                    target: target_nick.clone(),
                    server: state.server_name.clone(),
                    info: "IronChat IRC Server".to_string(),
                }));
            } else {
                // User not found
                messages.push(Message::from(Reply::NoSuchNick {
                    nick: requester_nick.to_string(),
                    target: target_nick.clone(),
                }));
            }
        } else {
            // User not found  
            messages.push(Message::from(Reply::NoSuchNick {
                nick: requester_nick.to_string(),
                target: target_nick.clone(),
            }));
        }
        
        // End of WHOIS for this target (318)
        messages.push(Message::from(Reply::EndOfWhois {
            nick: requester_nick.to_string(),
            target: target_nick.clone(),
        }));
    }
    
    Ok(messages)
}
