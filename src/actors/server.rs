use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use crate::actors::channel::{ChannelActor, ChannelMessage};
use crate::protocol::{Command, Message, Reply};
use crate::state::ServerState;

pub struct ServerActor {
    server_state: Arc<RwLock<ServerState>>,
    channels: Arc<RwLock<HashMap<String, mpsc::Sender<ChannelMessage>>>>,
    rx: mpsc::Receiver<ServerMessage>,
    tx: mpsc::Sender<ServerMessage>,
}

#[derive(Debug, Clone)]
pub enum ServerMessage {
    ConnectionCommand {
        connection_id: u64,
        command: Command,
    },
    CreateChannel {
        name: String,
        creator_id: u64,
    },
    RemoveChannel {
        name: String,
    },
    BroadcastMessage {
        message: Message,
        exclude: Option<u64>,
    },
    ServerNotice {
        target: String,
        message: String,
    },
}

impl ServerActor {
    pub fn new(server_state: Arc<RwLock<ServerState>>) -> (Self, mpsc::Sender<ServerMessage>) {
        let (tx, rx) = mpsc::channel(1024);
        let channels = Arc::new(RwLock::new(HashMap::new()));
        
        let actor = Self {
            server_state,
            channels,
            rx,
            tx: tx.clone(),
        };
        
        (actor, tx)
    }
    
    pub async fn run(mut self) {
        info!("Server actor started");
        
        while let Some(msg) = self.rx.recv().await {
            match msg {
                ServerMessage::ConnectionCommand { connection_id, command } => {
                    if let Err(e) = self.handle_command(connection_id, command).await {
                        error!("Error handling command: {}", e);
                    }
                }
                ServerMessage::CreateChannel { name, creator_id } => {
                    self.create_channel(name, creator_id).await;
                }
                ServerMessage::RemoveChannel { name } => {
                    self.remove_channel(&name).await;
                }
                ServerMessage::BroadcastMessage { message, exclude } => {
                    self.broadcast_message(message, exclude).await;
                }
                ServerMessage::ServerNotice { target, message } => {
                    self.send_server_notice(target, message).await;
                }
            }
        }
        
        info!("Server actor stopped");
    }
    
    async fn handle_command(
        &self,
        connection_id: u64,
        command: Command,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            Command::Join(channels, keys) => {
                self.handle_join(connection_id, channels, keys).await?;
            }
            Command::Part(channels, message) => {
                self.handle_part(connection_id, channels, message).await?;
            }
            Command::Privmsg { target, message } => {
                self.handle_privmsg(connection_id, target, message).await?;
            }
            Command::Notice { target, message } => {
                self.handle_notice(connection_id, target, message).await?;
            }
            Command::Topic { channel, topic } => {
                self.handle_topic(connection_id, channel, topic).await?;
            }
            Command::Kick { channel, user, reason } => {
                self.handle_kick(connection_id, channel, user, reason).await?;
            }
            Command::Mode { target, modes, params } => {
                self.handle_mode(connection_id, target, modes, params).await?;
            }
            Command::Who(mask) => {
                self.handle_who(connection_id, mask).await?;
            }
            Command::Whois(targets) => {
                self.handle_whois(connection_id, targets).await?;
            }
            Command::List(channels) => {
                self.handle_list(connection_id, channels).await?;
            }
            Command::Names(channels) => {
                self.handle_names(connection_id, channels).await?;
            }
            _ => {
                debug!("Unhandled command: {:?}", command);
            }
        }
        
        Ok(())
    }
    
    async fn handle_join(
        &self,
        connection_id: u64,
        channels: Vec<String>,
        keys: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.server_state.read().await;
        let conn = state.connections.get(&connection_id);
        
        if conn.is_none() {
            return Ok(());
        }
        
        let conn = conn.unwrap();
        let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
        drop(state);
        
        for (i, channel_name) in channels.iter().enumerate() {
            let key = keys.get(i).cloned();
            
            // Validate channel name
            if !crate::security::validate_channel_name(channel_name) {
                let state = self.server_state.read().await;
                if let Some(conn) = state.connections.get(&connection_id) {
                    let _ = conn.tx.send(Reply::NoSuchChannel {
                        nick: nick.clone(),
                        channel: channel_name.clone(),
                    }.to_message(&state.server_name)).await;
                }
                continue;
            }
            
            // Get or create channel
            let channel_tx = {
                let mut channels = self.channels.write().await;
                
                if let Some(tx) = channels.get(channel_name) {
                    tx.clone()
                } else {
                    // Create new channel
                    let (actor, tx) = ChannelActor::new(
                        channel_name.clone(),
                        Arc::clone(&self.server_state),
                    );
                    
                    channels.insert(channel_name.clone(), tx.clone());
                    
                    // Spawn channel actor
                    tokio::spawn(actor.run());
                    
                    // Add to server state
                    let mut state = self.server_state.write().await;
                    state.channels.insert(
                        channel_name.clone(),
                        crate::state::Channel::new(channel_name.clone()),
                    );
                    
                    tx
                }
            };
            
            // Send join message to channel
            let _ = channel_tx.send(ChannelMessage::Join {
                connection_id,
                key,
            }).await;
        }
        
        Ok(())
    }
    
    async fn handle_part(
        &self,
        connection_id: u64,
        channels: Vec<String>,
        message: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channels = self.channels.read().await;
        
        for channel_name in channels.iter() {
            if let Some(channel_tx) = channels.get(channel_name) {
                let _ = channel_tx.send(ChannelMessage::Part {
                    connection_id,
                    message: message.clone(),
                }).await;
            }
        }
        
        Ok(())
    }
    
    async fn handle_privmsg(
        &self,
        connection_id: u64,
        target: String,
        message: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if crate::utils::is_channel(&target) {
            // Send to channel
            let channels = self.channels.read().await;
            if let Some(channel_tx) = channels.get(&target) {
                let msg = Message::new("PRIVMSG")
                    .add_param(target)
                    .add_param(message);
                
                let _ = channel_tx.send(ChannelMessage::Message {
                    sender_id: connection_id,
                    message: msg,
                }).await;
            }
        } else {
            // Direct message to user
            let state = self.server_state.read().await;
            
            // Find sender info
            let sender = match state.connections.get(&connection_id) {
                Some(conn) => conn,
                None => return Ok(()),
            };
            
            let sender_mask = sender.full_mask();
            
            // Find target by nickname
            if let Some(&target_id) = state.nicknames.get(&target.to_lowercase()) {
                if let Some(target_conn) = state.connections.get(&target_id) {
                    let msg = Message::new("PRIVMSG")
                        .with_prefix(sender_mask)
                        .add_param(target)
                        .add_param(message);
                    
                    let _ = target_conn.tx.send(msg).await;
                }
            } else {
                // No such nick
                let nick = sender.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = sender.tx.send(Reply::NoSuchNick {
                    nick,
                    target,
                }.to_message(&state.server_name)).await;
            }
        }
        
        Ok(())
    }
    
    async fn handle_notice(
        &self,
        connection_id: u64,
        target: String,
        message: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Similar to PRIVMSG but no error replies
        if crate::utils::is_channel(&target) {
            let channels = self.channels.read().await;
            if let Some(channel_tx) = channels.get(&target) {
                let msg = Message::new("NOTICE")
                    .add_param(target)
                    .add_param(message);
                
                let _ = channel_tx.send(ChannelMessage::Message {
                    sender_id: connection_id,
                    message: msg,
                }).await;
            }
        } else {
            let state = self.server_state.read().await;
            
            let sender = match state.connections.get(&connection_id) {
                Some(conn) => conn,
                None => return Ok(()),
            };
            
            let sender_mask = sender.full_mask();
            
            if let Some(&target_id) = state.nicknames.get(&target.to_lowercase()) {
                if let Some(target_conn) = state.connections.get(&target_id) {
                    let msg = Message::new("NOTICE")
                        .with_prefix(sender_mask)
                        .add_param(target)
                        .add_param(message);
                    
                    let _ = target_conn.tx.send(msg).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_topic(
        &self,
        connection_id: u64,
        channel: String,
        topic: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channels = self.channels.read().await;
        
        if let Some(channel_tx) = channels.get(&channel) {
            let _ = channel_tx.send(ChannelMessage::SetTopic {
                connection_id,
                topic,
            }).await;
        } else {
            let state = self.server_state.read().await;
            if let Some(conn) = state.connections.get(&connection_id) {
                let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                let _ = conn.tx.send(Reply::NoSuchChannel {
                    nick,
                    channel,
                }.to_message(&state.server_name)).await;
            }
        }
        
        Ok(())
    }
    
    async fn handle_kick(
        &self,
        connection_id: u64,
        channel: String,
        user: String,
        reason: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channels = self.channels.read().await;
        
        if let Some(channel_tx) = channels.get(&channel) {
            // Find target user ID
            let state = self.server_state.read().await;
            if let Some(&target_id) = state.nicknames.get(&user.to_lowercase()) {
                let _ = channel_tx.send(ChannelMessage::Kick {
                    kicker_id: connection_id,
                    target_id,
                    reason,
                }).await;
            } else {
                if let Some(conn) = state.connections.get(&connection_id) {
                    let nick = conn.nickname.clone().unwrap_or_else(|| "*".to_string());
                    let _ = conn.tx.send(Reply::NoSuchNick {
                        nick,
                        target: user,
                    }.to_message(&state.server_name)).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn handle_mode(
        &self,
        connection_id: u64,
        target: String,
        modes: Option<String>,
        params: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if crate::utils::is_channel(&target) {
            let channels = self.channels.read().await;
            if let Some(channel_tx) = channels.get(&target) {
                if let Some(mode_str) = modes {
                    let _ = channel_tx.send(ChannelMessage::SetMode {
                        connection_id,
                        modes: mode_str,
                        params,
                    }).await;
                } else {
                    let _ = channel_tx.send(ChannelMessage::GetInfo {
                        requester_id: connection_id,
                    }).await;
                }
            }
        } else {
            // User modes
            // TODO: Implement user mode handling
        }
        
        Ok(())
    }
    
    async fn handle_who(
        &self,
        connection_id: u64,
        mask: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.server_state.read().await;
        let requester = match state.connections.get(&connection_id) {
            Some(conn) => conn,
            None => return Ok(()),
        };
        
        let nick = requester.nickname.clone().unwrap_or_else(|| "*".to_string());
        
        // TODO: Implement WHO mask matching
        // For now, just send end of WHO
        let _ = requester.tx.send(Reply::EndOfWho {
            nick,
            target: mask.unwrap_or_else(|| "*".to_string()),
        }.to_message(&state.server_name)).await;
        
        Ok(())
    }
    
    async fn handle_whois(
        &self,
        connection_id: u64,
        targets: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.server_state.read().await;
        let requester = match state.connections.get(&connection_id) {
            Some(conn) => conn,
            None => return Ok(()),
        };
        
        let nick = requester.nickname.clone().unwrap_or_else(|| "*".to_string());
        
        for target in targets {
            if let Some(&target_id) = state.nicknames.get(&target.to_lowercase()) {
                if let Some(target_conn) = state.connections.get(&target_id) {
                    let target_nick = target_conn.nickname.clone().unwrap();
                    let username = target_conn.username.clone().unwrap_or_else(|| "*".to_string());
                    let realname = target_conn.realname.clone().unwrap_or_else(|| "*".to_string());
                    
                    // Send WHOIS replies
                    let _ = requester.tx.send(Reply::WhoisUser {
                        nick: nick.clone(),
                        target: target_nick.clone(),
                        username,
                        host: target_conn.hostname.clone(),
                        realname,
                    }.to_message(&state.server_name)).await;
                    
                    let _ = requester.tx.send(Reply::WhoisServer {
                        nick: nick.clone(),
                        target: target_nick.clone(),
                        server: state.server_name.clone(),
                        info: "IronChat IRC Server".to_string(),
                    }.to_message(&state.server_name)).await;
                    
                    // TODO: Add channels, idle time, etc.
                    
                    let _ = requester.tx.send(Reply::EndOfWhois {
                        nick: nick.clone(),
                        target: target_nick,
                    }.to_message(&state.server_name)).await;
                }
            } else {
                let _ = requester.tx.send(Reply::NoSuchNick {
                    nick: nick.clone(),
                    target: target.clone(),
                }.to_message(&state.server_name)).await;
            }
        }
        
        Ok(())
    }
    
    async fn handle_list(
        &self,
        connection_id: u64,
        channels: Option<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.server_state.read().await;
        let requester = match state.connections.get(&connection_id) {
            Some(conn) => conn,
            None => return Ok(()),
        };
        
        let nick = requester.nickname.clone().unwrap_or_else(|| "*".to_string());
        
        // Send list start
        let _ = requester.tx.send(Reply::ListStart {
            nick: nick.clone(),
        }.to_message(&state.server_name)).await;
        
        // List channels
        let channel_list: Vec<String> = if let Some(specific) = channels {
            specific
        } else {
            state.channels.iter().map(|entry| entry.key().clone()).collect()
        };
        
        let channel_actors = self.channels.read().await;
        
        for channel_name in channel_list {
            if let Some(channel_tx) = channel_actors.get(&channel_name) {
                let _ = channel_tx.send(ChannelMessage::GetInfo {
                    requester_id: connection_id,
                }).await;
            }
        }
        
        // Send list end
        let _ = requester.tx.send(Reply::ListEnd {
            nick,
        }.to_message(&state.server_name)).await;
        
        Ok(())
    }
    
    async fn handle_names(
        &self,
        connection_id: u64,
        channels: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channel_actors = self.channels.read().await;
        
        for channel_name in channels {
            if let Some(channel_tx) = channel_actors.get(&channel_name) {
                // Channel actor will send names directly to the requester
                let _ = channel_tx.send(ChannelMessage::GetInfo {
                    requester_id: connection_id,
                }).await;
            }
        }
        
        Ok(())
    }
    
    async fn create_channel(&self, name: String, creator_id: u64) {
        let mut channels = self.channels.write().await;
        
        if channels.contains_key(&name) {
            return;
        }
        
        let (actor, tx) = ChannelActor::new(
            name.clone(),
            Arc::clone(&self.server_state),
        );
        
        channels.insert(name.clone(), tx);
        
        // Spawn channel actor
        tokio::spawn(actor.run());
        
        // Add to server state
        let mut state = self.server_state.write().await;
        state.channels.insert(
            name.clone(),
            crate::state::Channel::new(name),
        );
        
        info!("Created channel: {}", name);
    }
    
    async fn remove_channel(&self, name: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(name);
        
        let mut state = self.server_state.write().await;
        state.channels.remove(name);
        
        info!("Removed channel: {}", name);
    }
    
    async fn broadcast_message(&self, message: Message, exclude: Option<u64>) {
        let state = self.server_state.read().await;
        
        for conn in state.connections.iter() {
            if Some(*conn.key()) == exclude {
                continue;
            }
            
            let _ = conn.value().tx.send(message.clone()).await;
        }
    }
    
    async fn send_server_notice(&self, target: String, message: String) {
        let state = self.server_state.read().await;
        let server_name = state.server_name.clone();
        
        let notice = Message::new("NOTICE")
            .with_prefix(server_name)
            .add_param(target.clone())
            .add_param(message);
        
        if crate::utils::is_channel(&target) {
            // Send to channel
            drop(state);
            let channels = self.channels.read().await;
            if let Some(channel_tx) = channels.get(&target) {
                let _ = channel_tx.send(ChannelMessage::Message {
                    sender_id: 0, // Server ID
                    message: notice,
                }).await;
            }
        } else if target == "*" {
            // Broadcast to all
            for conn in state.connections.iter() {
                let _ = conn.value().tx.send(notice.clone()).await;
            }
        } else {
            // Send to specific user
            if let Some(&target_id) = state.nicknames.get(&target.to_lowercase()) {
                if let Some(conn) = state.connections.get(&target_id) {
                    let _ = conn.tx.send(notice).await;
                }
            }
        }
    }
}