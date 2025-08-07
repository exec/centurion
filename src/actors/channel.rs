use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::protocol::{Message, Reply};
use crate::state::{Channel, ChannelMember, ServerState};

pub struct ChannelActor {
    name: String,
    channel: Arc<RwLock<Channel>>,
    server_state: Arc<RwLock<ServerState>>,
    rx: mpsc::Receiver<ChannelMessage>,
    tx: mpsc::Sender<ChannelMessage>,
}

#[derive(Debug, Clone)]
pub enum ChannelMessage {
    Join {
        connection_id: u64,
        key: Option<String>,
    },
    Part {
        connection_id: u64,
        message: Option<String>,
    },
    Message {
        sender_id: u64,
        message: Message,
    },
    SetTopic {
        connection_id: u64,
        topic: Option<String>,
    },
    Kick {
        kicker_id: u64,
        target_id: u64,
        reason: Option<String>,
    },
    SetMode {
        connection_id: u64,
        modes: String,
        params: Vec<String>,
    },
    Invite {
        inviter_id: u64,
        target_id: u64,
    },
    GetInfo {
        requester_id: u64,
    },
}

impl ChannelActor {
    pub fn new(
        name: String,
        server_state: Arc<RwLock<ServerState>>,
    ) -> (Self, mpsc::Sender<ChannelMessage>) {
        let (tx, rx) = mpsc::channel(256);
        let channel = Arc::new(RwLock::new(Channel::new(name.clone())));
        
        let actor = Self {
            name: name.clone(),
            channel,
            server_state,
            rx,
            tx: tx.clone(),
        };
        
        (actor, tx)
    }
    
    pub async fn run(mut self) {
        info!("Channel actor started for {}", self.name);
        
        while let Some(msg) = self.rx.recv().await {
            match msg {
                ChannelMessage::Join { connection_id, key } => {
                    self.handle_join(connection_id, key).await;
                }
                ChannelMessage::Part { connection_id, message } => {
                    self.handle_part(connection_id, message).await;
                }
                ChannelMessage::Message { sender_id, message } => {
                    self.handle_message(sender_id, message).await;
                }
                ChannelMessage::SetTopic { connection_id, topic } => {
                    self.handle_set_topic(connection_id, topic).await;
                }
                ChannelMessage::Kick { kicker_id, target_id, reason } => {
                    self.handle_kick(kicker_id, target_id, reason).await;
                }
                ChannelMessage::SetMode { connection_id, modes, params } => {
                    self.handle_set_mode(connection_id, modes, params).await;
                }
                ChannelMessage::Invite { inviter_id, target_id } => {
                    self.handle_invite(inviter_id, target_id).await;
                }
                ChannelMessage::GetInfo { requester_id } => {
                    self.handle_get_info(requester_id).await;
                }
            }
        }
        
        info!("Channel actor stopped for {}", self.name);
    }
    
    async fn handle_join(&self, connection_id: u64, key: Option<String>) {
        let channel = self.channel.read().await;
        let state = self.server_state.read().await;
        
        // Check if already a member
        if channel.is_member(connection_id) {
            return;
        }
        
        // Get connection info
        let conn = match state.connections.get(&connection_id) {
            Some(conn) => conn.clone(),
            None => return,
        };
        
        let nick = match &conn.nickname {
            Some(n) => n.clone(),
            None => return,
        };
        
        // Check channel modes
        if channel.modes.contains(&'k') {
            if channel.key.as_ref() != key.as_ref() {
                let _ = conn.tx.send(Reply::BadChannelKey {
                    nick: nick.clone(),
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
                return;
            }
        }
        
        if channel.modes.contains(&'l') {
            if let Some(limit) = channel.limit {
                if channel.member_count() >= limit {
                    let _ = conn.tx.send(Reply::ChannelIsFull {
                        nick: nick.clone(),
                        channel: self.name.clone(),
                    }.to_message(&state.server_name)).await;
                    return;
                }
            }
        }
        
        // Check if banned
        // TODO: Implement ban checking
        
        // Add member
        let is_first = channel.member_count() == 0;
        drop(channel);
        
        self.channel.write().await.add_member(connection_id, is_first);
        
        // Send join message to all members
        let join_msg = Message::new("JOIN")
            .with_prefix(conn.full_mask())
            .with_params(vec![self.name.clone()]);
        
        self.broadcast_message(join_msg.clone(), None).await;
        
        // Send channel info to joiner
        let channel = self.channel.read().await;
        
        // Send topic if set
        if let Some(topic) = &channel.topic {
            let _ = conn.tx.send(Reply::Topic {
                nick: nick.clone(),
                channel: self.name.clone(),
                topic: topic.clone(),
            }.to_message(&state.server_name)).await;
        } else {
            let _ = conn.tx.send(Reply::NoTopic {
                nick: nick.clone(),
                channel: self.name.clone(),
            }.to_message(&state.server_name)).await;
        }
        
        // Send names list
        self.send_names_list(connection_id).await;
    }
    
    async fn handle_part(&self, connection_id: u64, message: Option<String>) {
        let state = self.server_state.read().await;
        let conn = match state.connections.get(&connection_id) {
            Some(conn) => conn.clone(),
            None => return,
        };
        
        let channel = self.channel.write().await;
        if !channel.remove_member(connection_id) {
            return;
        }
        
        drop(channel);
        drop(state);
        
        // Send part message to all members (including the parting user)
        let mut params = vec![self.name.clone()];
        if let Some(msg) = message {
            params.push(msg);
        }
        
        let part_msg = Message::new("PART")
            .with_prefix(conn.full_mask())
            .with_params(params);
        
        self.broadcast_message(part_msg, None).await;
        
        // Check if channel is empty and should be removed
        let channel = self.channel.read().await;
        if channel.member_count() == 0 {
            // TODO: Remove channel from server state
        }
    }
    
    async fn handle_message(&self, sender_id: u64, mut message: Message) {
        let channel = self.channel.read().await;
        
        // Check if sender is a member
        if !channel.is_member(sender_id) {
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&sender_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = conn.tx.send(Reply::NotOnChannel {
                    nick,
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
            }
            return;
        }
        
        // Check channel modes
        if channel.modes.contains(&'n') && !channel.is_member(sender_id) {
            // No external messages
            return;
        }
        
        if channel.modes.contains(&'m') && !channel.is_operator(sender_id) {
            // Moderated channel - only ops can speak
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&sender_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = conn.tx.send(Reply::CannotSendToChan {
                    nick,
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
            }
            return;
        }
        
        drop(channel);
        
        // Add sender prefix if not present
        if message.prefix.is_none() {
            let full_mask = {
                let state = self.server_state.read().await;
                state.connections.get(&sender_id).map(|conn| conn.full_mask())
            };
            if let Some(mask) = full_mask {
                message.prefix = Some(mask);
            }
        }
        
        // Broadcast to all members except sender
        self.broadcast_message(message, Some(sender_id)).await;
    }
    
    async fn handle_set_topic(&self, connection_id: u64, topic: Option<String>) {
        let channel = self.channel.read().await;
        
        // Check if user is a member
        if !channel.is_member(connection_id) {
            return;
        }
        
        // Check if topic is locked to ops
        if channel.modes.contains(&'t') && !channel.is_operator(connection_id) {
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&connection_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = conn.tx.send(Reply::ChanOpPrivsNeeded {
                    nick,
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
            }
            return;
        }
        
        drop(channel);
        
        // Get setter info
        let state = self.server_state.read().await;
        let setter_info = state.connections.get(&connection_id)
            .and_then(|conn| conn.nickname.clone())
            .unwrap_or_else(|| "*".to_string());
        let setter_mask = state.connections.get(&connection_id)
            .map(|conn| conn.full_mask())
            .unwrap_or_else(|| "*!*@*".to_string());
        drop(state);
        
        // Update topic
        let mut channel = self.channel.write().await;
        channel.topic = topic.clone();
        channel.topic_set_by = Some(setter_info);
        channel.topic_set_at = Some(Utc::now());
        drop(channel);
        
        // Broadcast topic change
        let topic_msg = Message::new("TOPIC")
            .with_prefix(setter_mask)
            .with_params(vec![self.name.clone(), topic.unwrap_or_default()]);
        
        self.broadcast_message(topic_msg, None).await;
    }
    
    async fn handle_kick(&self, kicker_id: u64, target_id: u64, reason: Option<String>) {
        let channel = self.channel.read().await;
        
        // Check if kicker is an operator
        if !channel.is_operator(kicker_id) {
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&kicker_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = conn.tx.send(Reply::ChanOpPrivsNeeded {
                    nick,
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
            }
            return;
        }
        
        // Check if target is a member
        if !channel.is_member(target_id) {
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&kicker_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let target_nick = state.connections.get(&target_id)
                    .and_then(|c| c.nickname.clone())
                    .unwrap_or_else(|| "*".to_string());
                
                let _ = conn.tx.send(Reply::UserNotInChannel {
                    nick,
                    target: target_nick,
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
            }
            return;
        }
        
        drop(channel);
        
        // Get kicker and target info
        let state = self.server_state.read().await;
        let kicker_mask = state.connections.get(&kicker_id)
            .map(|conn| conn.full_mask())
            .unwrap_or_else(|| "*!*@*".to_string());
        let target_nick = state.connections.get(&target_id)
            .and_then(|conn| conn.nickname.clone())
            .unwrap_or_else(|| "*".to_string());
        drop(state);
        
        // Remove target from channel
        self.channel.write().await.remove_member(target_id);
        
        // Send kick message
        let mut params = vec![self.name.clone(), target_nick];
        if let Some(r) = reason {
            params.push(r);
        }
        
        let kick_msg = Message::new("KICK")
            .with_prefix(kicker_mask)
            .with_params(params);
        
        self.broadcast_message(kick_msg, None).await;
    }
    
    async fn handle_set_mode(&self, _connection_id: u64, modes: String, params: Vec<String>) {
        // TODO: Implement mode parsing and setting
        debug!("Mode change requested: {} {}", modes, params.join(" "));
    }
    
    async fn handle_invite(&self, inviter_id: u64, _target_id: u64) {
        let channel = self.channel.read().await;
        
        // Check if inviter is a member
        if !channel.is_member(inviter_id) {
            return;
        }
        
        // Check if channel is invite-only
        if channel.modes.contains(&'i') && !channel.is_operator(inviter_id) {
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&inviter_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = conn.tx.send(Reply::ChanOpPrivsNeeded {
                    nick,
                    channel: self.name.clone(),
                }.to_message(&state.server_name)).await;
            }
            return;
        }
        
        // TODO: Add invite to invite list
        // TODO: Send invite notification to target
    }
    
    async fn handle_get_info(&self, requester_id: u64) {
        // Send channel information to requester
        let conn_info = {
            let state = self.server_state.read().await;
            state.connections.get(&requester_id).map(|conn| {
                (conn.nickname.clone().unwrap_or_else(|| "*".to_string()),
                 conn.tx.clone(),
                 state.server_name.clone())
            })
        };
        
        let (nick, tx, server_name) = match conn_info {
            Some(info) => info,
            None => return,
        };
        
        let (modes, topic, member_count, channel_name) = {
            let channel = self.channel.read().await;
            let modes = format!("+{}", channel.modes.iter().collect::<String>());
            let topic = channel.topic.clone().unwrap_or_default();
            let member_count = channel.member_count();
            let channel_name = self.name.clone();
            (modes, topic, member_count, channel_name)
        };
        
        // Send channel mode
        let _ = tx.send(Reply::ChannelModeIs {
            nick: nick.clone(),
            channel: channel_name.clone(),
            modes,
            params: vec![],
        }.to_message(&server_name)).await;
        
        // Send member count in LIST format
        let _ = tx.send(Reply::List {
            nick,
            channel: channel_name,
            visible: member_count,
            topic,
        }.to_message(&server_name)).await;
    }
    
    async fn broadcast_message(&self, message: Message, exclude: Option<u64>) {
        let channel = self.channel.read().await;
        let state = self.server_state.read().await;
        
        for member in channel.members.iter() {
            let member_id = *member.key();
            
            if Some(member_id) == exclude {
                continue;
            }
            
            if let Some(conn) = state.connections.get(&member_id) {
                let _ = conn.tx.send(message.clone()).await;
            }
        }
    }
    
    async fn send_names_list(&self, connection_id: u64) {
        let channel = self.channel.read().await;
        let state = self.server_state.read().await;
        
        let conn = match state.connections.get(&connection_id) {
            Some(conn) => conn,
            None => return,
        };
        
        let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
        let mut names = Vec::new();
        
        for member in channel.members.iter() {
            if let Some(member_conn) = state.connections.get(member.key()) {
                if let Some(member_nick) = &member_conn.nickname {
                    let prefix = if member.value().modes.contains(&'o') {
                        "@"
                    } else if member.value().modes.contains(&'v') {
                        "+"
                    } else {
                        ""
                    };
                    names.push(format!("{}{}", prefix, member_nick));
                }
            }
        }
        
        // Send names in batches
        for chunk in names.chunks(10) {
            let _ = conn.tx.send(Reply::NamReply {
                nick: nick.clone(),
                symbol: '=',
                channel: self.name.clone(),
                names: chunk.to_vec(),
            }.to_message(&state.server_name)).await;
        }
        
        // Send end of names
        let _ = conn.tx.send(Reply::EndOfNames {
            nick,
            channel: self.name.clone(),
        }.to_message(&state.server_name)).await;
    }
}