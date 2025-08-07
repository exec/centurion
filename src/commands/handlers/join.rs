use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::{ServerState, Channel, ChannelMember};
use chrono::Utc;

pub async fn handle_join(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    channels: Vec<String>,
    _keys: Vec<String>,
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
        // Validate channel name
        if !channel_name.starts_with('#') && !channel_name.starts_with('&') {
            responses.push(Message::from(
                Reply::NoSuchChannel {
                    nick: nick.clone(),
                    channel: channel_name.clone(),
                }
            ));
            continue;
        }
        
        // Get or create channel
        let channel = state.channels.entry(channel_name.clone())
            .or_insert_with(|| Channel {
                name: channel_name.clone(),
                topic: None,
                topic_set_by: None,
                topic_set_at: None,
                modes: Vec::new(),
                members: dashmap::DashMap::new(),
                created_at: Utc::now(),
                key: None,
                limit: None,
            });
        
        // Check if already in channel
        if channel.members.contains_key(&connection_id) {
            continue; // Already in channel, skip
        }
        
        // Add member to channel
        channel.members.insert(connection_id, ChannelMember {
            connection_id,
            modes: Vec::new(),
            joined_at: Utc::now(),
        });
        
        // Send JOIN message to all channel members (including joiner)
        let join_msg = Message::new("JOIN")
            .with_prefix(prefix.clone())
            .with_params(vec![channel_name.clone()]);
        
        // Send to all OTHER members immediately (they need to see the join)
        for entry in channel.members.iter() {
            let member_id = *entry.key();
            if member_id != connection_id {  // Don't send to joiner yet
                if let Some(member_conn) = state.connections.get(&member_id) {
                    let _ = member_conn.tx.send(join_msg.clone()).await;
                }
            }
        }
        
        // Send JOIN message to joiner first (so client creates channel)
        responses.push(join_msg);
        
        // Send channel topic if it exists (to joiner only)
        if let Some(topic) = &channel.topic {
            responses.push(Message::from(
                Reply::Topic {
                    nick: nick.clone(),
                    channel: channel_name.clone(),
                    topic: topic.clone(),
                }
            ));
        } else {
            responses.push(Message::from(
                Reply::NoTopic {
                    nick: nick.clone(),
                    channel: channel_name.clone(),
                }
            ));
        }
        
        // Send NAMES list (to joiner only)
        let mut names = Vec::new();
        for entry in channel.members.iter() {
            let member_id = *entry.key();
            if let Some(member_conn) = state.connections.get(&member_id) {
                if let Some(member_nick) = &member_conn.nickname {
                    names.push(member_nick.clone());
                }
            }
        }
        
        if !names.is_empty() {
            responses.push(Message::from(
                Reply::NamReply {
                    nick: nick.clone(),
                    symbol: '=', // '=' for public channel
                    channel: channel_name.clone(),
                    names,
                }
            ));
        }
        
        responses.push(Message::from(
            Reply::EndOfNames {
                nick: nick.clone(),
                channel: channel_name.clone(),
            }
        ));
    }
    
    Ok(responses)
}