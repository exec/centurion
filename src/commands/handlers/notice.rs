use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::Message;
use crate::state::ServerState;

pub async fn handle_notice(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    // TODO: Implement notice command
    Ok(vec![])
}
