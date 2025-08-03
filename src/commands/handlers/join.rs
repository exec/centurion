use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::Message;
use crate::state::ServerState;

pub async fn handle_join(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    channels: Vec<String>,
    keys: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    // TODO: Implement JOIN command
    Ok(vec![])
}