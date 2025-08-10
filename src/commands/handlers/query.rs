use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_query(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    target: String,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut responses = Vec::new();
    
    // Get connection info
    let connection_info = {
        let state = server_state.read().await;
        state.connections.get(&connection_id).map(|conn| {
            (conn.nickname.clone().unwrap_or_else(|| "*".to_string()))
        })
    };

    let nick = match connection_info {
        Some(nick) => nick,
        None => return Ok(responses),
    };
    
    // QUERY doesn't have a standard IRC response, but we can check if the target exists
    // and send appropriate feedback to help the client open a DM window
    
    let target_exists = {
        let state = server_state.read().await;
        state.nicknames.contains_key(&target.to_lowercase())
    };
    
    if target_exists {
        // Send a notice to help the client recognize this is a valid DM target
        // This is a non-standard but helpful response that indicates the query is valid
        responses.push(Message::new("NOTICE")
            .with_params(vec![nick.clone(), format!("Query window opened for {}", target)])
            .with_prefix("*.Query".to_string()));
    } else {
        // Target doesn't exist - send an error
        responses.push(Reply::NoSuchNick { 
            nick: nick.clone(), 
            target: target 
        }.to_message("*"));
    }
    
    Ok(responses)
}