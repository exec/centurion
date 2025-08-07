use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;
use std::fs;
use std::path::Path;

pub async fn handle_motd(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    _params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut responses = Vec::new();
    let state = server_state.read().await;
    
    // Get connection nick
    let connection = state.connections.get(&connection_id)
        .ok_or("Connection not found")?;
    let nick = connection.nickname.clone()
        .unwrap_or_else(|| "*".to_string());
    
    // Try to read MOTD file
    let motd_path = Path::new("motd.txt");
    let server_name = "ironchatd.local";
    
    if motd_path.exists() {
        match fs::read_to_string(motd_path) {
            Ok(motd_content) => {
                // Send MOTD start
                responses.push(Message::from(Reply::MotdStart {
                    nick: nick.clone(),
                    server: server_name.to_string(),
                }));
                
                // Send each line of MOTD
                for line in motd_content.lines() {
                    responses.push(Message::from(Reply::Motd {
                        nick: nick.clone(),
                        line: line.to_string(),
                    }));
                }
                
                // Send MOTD end
                responses.push(Message::from(Reply::EndOfMotd {
                    nick: nick.clone(),
                }));
            }
            Err(_) => {
                // Error reading file, send no MOTD
                responses.push(Message::from(Reply::NoMotd {
                    nick: nick.clone(),
                }));
            }
        }
    } else {
        // No MOTD file
        responses.push(Message::from(Reply::NoMotd {
            nick: nick.clone(),
        }));
    }
    
    Ok(responses)
}

pub async fn send_motd(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    handle_motd(server_state, connection_id, vec![]).await
}
