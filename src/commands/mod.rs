use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::protocol::{Command, Message, Reply};
use crate::state::ServerState;

pub mod handlers;

pub struct CommandProcessor {
    server_state: Arc<RwLock<ServerState>>,
}

impl CommandProcessor {
    pub fn new(server_state: Arc<RwLock<ServerState>>) -> Self {
        Self { server_state }
    }
    
    pub async fn process(
        &self,
        connection_id: u64,
        command: Command,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        debug!("Processing command for connection {}: {:?}", connection_id, command);
        
        match command {
            Command::Join(channels, keys) => {
                handlers::join::handle_join(self.server_state.clone(), connection_id, channels, keys).await
            }
            Command::Part(channels, message) => {
                handlers::part::handle_part(self.server_state.clone(), connection_id, channels, message).await
            }
            Command::Privmsg { target, message } => {
                handlers::privmsg::handle_privmsg(self.server_state.clone(), connection_id, target, message).await
            }
            Command::Notice { target, message } => {
                handlers::notice::handle_notice(self.server_state.clone(), connection_id, target, message).await
            }
            Command::Who(mask) => {
                handlers::who::handle_who(self.server_state.clone(), connection_id, mask).await
            }
            Command::Whois(targets) => {
                handlers::whois::handle_whois(self.server_state.clone(), connection_id, targets).await
            }
            Command::Topic { channel, topic } => {
                handlers::topic::handle_topic(self.server_state.clone(), connection_id, channel, topic).await
            }
            Command::Mode { target, modes, params } => {
                handlers::mode::handle_mode(self.server_state.clone(), connection_id, target, modes, params).await
            }
            Command::Kick { channel, user, reason } => {
                handlers::kick::handle_kick(self.server_state.clone(), connection_id, channel, user, reason).await
            }
            Command::List(channels) => {
                handlers::list::handle_list(self.server_state.clone(), connection_id, channels).await
            }
            Command::Names(channels) => {
                handlers::names::handle_names(self.server_state.clone(), connection_id, channels).await
            }
            Command::Motd(server) => {
                handlers::motd::handle_motd(self.server_state.clone(), connection_id, server).await
            }
            Command::Oper { name, password } => {
                handlers::oper::handle_oper(self.server_state.clone(), connection_id, name, password).await
            }
            _ => {
                let state = self.server_state.read().await;
                let nick = state.connections.get(&connection_id)
                    .and_then(|c| c.nickname.clone())
                    .unwrap_or_else(|| "*".to_string());
                
                Ok(vec![Reply::UnknownCommand {
                    nick,
                    command: format!("{:?}", command),
                }.to_message(&state.server_name)])
            }
        }
    }
}