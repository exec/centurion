use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_topic(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut responses = Vec::new();
    
    if params.is_empty() {
        return Err("TOPIC command requires channel parameter".into());
    }
    
    let channel_name = params[0].clone();
    let new_topic = params.get(1).cloned();
    
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
    
    // Find the channel
    let mut channel = match state.channels.get_mut(&channel_name) {
        Some(channel) => channel,
        None => {
            responses.push(Message::from(Reply::NoSuchChannel {
                nick,
                channel: channel_name,
            }));
            return Ok(responses);
        }
    };
    
    // Check if user is in the channel
    if !channel.is_member(connection_id) {
        responses.push(Message::from(Reply::NotOnChannel {
            nick,
            channel: channel_name,
        }));
        return Ok(responses);
    }
    
    match new_topic {
        Some(topic) => {
            // Setting topic
            // Check if channel has +t mode (topic restricted to operators)
            if channel.modes.contains(&'t') && !channel.is_operator(connection_id) {
                responses.push(Message::from(Reply::ChanOpPrivsNeeded {
                    nick,
                    channel: channel_name,
                }));
                return Ok(responses);
            }
            
            // Update the topic
            channel.topic = if topic.is_empty() { None } else { Some(topic.clone()) };
            channel.topic_set_by = Some(prefix.clone());
            channel.topic_set_at = Some(chrono::Utc::now());
            
            // Create TOPIC message
            let topic_msg = Message::new("TOPIC")
                .with_prefix(prefix)
                .with_params(vec![channel_name.clone(), topic]);
            
            // Send TOPIC message to all channel members
            for entry in channel.members.iter() {
                let member_id = *entry.key();
                if let Some(member_conn) = state.connections.get(&member_id) {
                    let _ = member_conn.tx.send(topic_msg.clone()).await;
                }
            }
            
            // Send TOPIC confirmation to the setter
            responses.push(topic_msg);
        },
        None => {
            // Getting topic
            match &channel.topic {
                Some(topic) => {
                    responses.push(Message::from(Reply::Topic {
                        nick: nick.clone(),
                        channel: channel_name.clone(),
                        topic: topic.clone(),
                    }));
                    
                    // TODO: Implement TopicWhoTime (IRC 333) reply when available in legion-protocol
                    // This would show who set the topic and when
                },
                None => {
                    responses.push(Message::from(Reply::NoTopic {
                        nick,
                        channel: channel_name,
                    }));
                }
            }
        }
    }
    
    Ok(responses)
}

#[cfg(test)]
#[path = "topic_test.rs"]
mod tests;
