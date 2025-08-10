use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_kick(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut responses = Vec::new();
    
    if params.len() < 2 {
        return Err("KICK command requires channel and nick parameters".into());
    }
    
    let channel_name = params[0].clone();
    let target_nick = params[1].clone();
    let kick_reason = if params.len() > 2 {
        params[2].clone()
    } else {
        "Kicked".to_string()
    };
    
    let mut state = server_state.write().await;
    
    // Get kicker connection info
    let connection = state.connections.get(&connection_id)
        .ok_or("Connection not found")?
        .clone();
    
    let kicker_nick = connection.nickname.clone()
        .ok_or("No nickname set")?;
    let user = connection.username.clone()
        .unwrap_or(kicker_nick.clone());
    let host = connection.hostname.clone();
    
    let kicker_prefix = format!("{}!{}@{}", kicker_nick, user, host);
    
    // Find the channel
    let channel = match state.channels.get_mut(&channel_name) {
        Some(channel) => channel,
        None => {
            responses.push(Message::from(Reply::NoSuchChannel {
                nick: kicker_nick,
                channel: channel_name,
            }));
            return Ok(responses);
        }
    };
    
    // Check if kicker is in the channel
    if !channel.members.contains_key(&connection_id) {
        responses.push(Message::from(Reply::NotOnChannel {
            nick: kicker_nick,
            channel: channel_name,
        }));
        return Ok(responses);
    }
    
    // Check if kicker has operator privileges
    if !channel.is_operator(connection_id) {
        responses.push(Message::from(Reply::ChanOpPrivsNeeded {
            nick: kicker_nick,
            channel: channel_name,
        }));
        return Ok(responses);
    }
    
    // Find target user by nickname
    let target_connection_id = state.connections
        .iter()
        .find_map(|entry| {
            let (id, conn) = entry.pair();
            if conn.nickname.as_ref() == Some(&target_nick) {
                Some(*id)
            } else {
                None
            }
        });
    
    let target_connection_id = match target_connection_id {
        Some(id) => id,
        None => {
            responses.push(Message::from(Reply::NoSuchNick {
                nick: kicker_nick,
                target: target_nick,
            }));
            return Ok(responses);
        }
    };
    
    // Check if target is in the channel
    if !channel.members.contains_key(&target_connection_id) {
        responses.push(Message::from(Reply::UserNotInChannel {
            nick: kicker_nick,
            target: target_nick,
            channel: channel_name,
        }));
        return Ok(responses);
    }
    
    // Remove target from channel
    channel.members.remove(&target_connection_id);
    
    // Create KICK message
    let kick_msg = Message::new("KICK")
        .with_prefix(kicker_prefix)
        .with_params(vec![channel_name.clone(), target_nick.clone(), kick_reason]);
    
    // Send KICK message to all remaining channel members
    for entry in channel.members.iter() {
        let member_id = *entry.key();
        if let Some(member_conn) = state.connections.get(&member_id) {
            let _ = member_conn.tx.send(kick_msg.clone()).await;
        }
    }
    
    // Send KICK message to the kicked user
    if let Some(target_conn) = state.connections.get(&target_connection_id) {
        let _ = target_conn.tx.send(kick_msg.clone()).await;
    }
    
    // Send KICK confirmation to the kicker
    responses.push(kick_msg);
    
    // If channel is empty, remove it
    if channel.members.is_empty() {
        state.channels.remove(&channel_name);
    }
    
    Ok(responses)
}

#[cfg(test)]
#[path = "kick_test.rs"]
mod tests;
