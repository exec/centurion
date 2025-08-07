use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::protocol::{Command, Message, Reply};
use crate::state::ServerState;

pub mod handlers;
pub mod standard_replies;

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
            Command::Nick(new_nick) => {
                handlers::nick::handle_nick(self.server_state.clone(), connection_id, new_nick).await
            }
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
                handlers::notice::handle_notice(self.server_state.clone(), connection_id, vec![target, message]).await
            }
            Command::Who(mask) => {
                let params = mask.map_or(vec![], |m| vec![m]);
                handlers::who::handle_who(self.server_state.clone(), connection_id, params).await
            }
            Command::Whois(targets) => {
                handlers::whois::handle_whois(self.server_state.clone(), connection_id, targets).await
            }
            Command::Topic { channel, topic } => {
                let mut params = vec![channel];
                if let Some(t) = topic {
                    params.push(t);
                }
                handlers::topic::handle_topic(self.server_state.clone(), connection_id, params).await
            }
            Command::Mode { target, modes, params } => {
                let mut all_params = vec![target];
                if let Some(m) = modes {
                    all_params.push(m);
                }
                all_params.extend(params);
                handlers::mode::handle_mode(self.server_state.clone(), connection_id, all_params).await
            }
            Command::Kick { channel, user, reason } => {
                let mut params = vec![channel, user];
                if let Some(r) = reason {
                    params.push(r);
                }
                handlers::kick::handle_kick(self.server_state.clone(), connection_id, params).await
            }
            Command::List(channels) => {
                let params = channels.unwrap_or_default();
                handlers::list::handle_list(self.server_state.clone(), connection_id, params).await
            }
            Command::Names(channels) => {
                handlers::names::handle_names(self.server_state.clone(), connection_id, channels).await
            }
            Command::Motd(server) => {
                let params = server.map_or(vec![], |s| vec![s]);
                handlers::motd::handle_motd(self.server_state.clone(), connection_id, params).await
            }
            Command::Oper { name, password } => {
                handlers::oper::handle_oper(self.server_state.clone(), connection_id, vec![name, password]).await
            }
            Command::ChatHistory { subcommand, target, params } => {
                let mut full_params = vec![subcommand, target];
                full_params.extend(params);
                handlers::chathistory::handle_chathistory(self.server_state.clone(), connection_id, full_params).await
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