use std::sync::Arc;
use tokio::sync::RwLock;
use crate::protocol::{Message, Reply};
use crate::state::ServerState;

pub async fn handle_mode(
    server_state: Arc<RwLock<ServerState>>,
    connection_id: u64,
    params: Vec<String>,
) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    let mut responses = Vec::new();
    
    if params.is_empty() {
        return Err("MODE command requires target parameter".into());
    }
    
    let target = params[0].clone();
    let mut state = server_state.write().await;
    
    // Get connection info
    let connection = state.connections.get(&connection_id)
        .ok_or("Connection not found")?
        .clone();
    
    let nick = connection.nickname.clone()
        .ok_or("No nickname set")?;
    
    // Check if target is a channel
    if target.starts_with('#') || target.starts_with('&') || target.starts_with('!') || target.starts_with('+') {
        handle_channel_mode(&mut state, connection_id, &nick, &target, &params[1..], &mut responses).await?;
    } else {
        // User mode - not implemented yet
        return Err("User modes not implemented yet".into());
    }
    
    Ok(responses)
}

async fn handle_channel_mode(
    state: &mut tokio::sync::RwLockWriteGuard<'_, ServerState>,
    connection_id: u64,
    nick: &str,
    channel_name: &str,
    mode_params: &[String],
    responses: &mut Vec<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find the channel
    let mut channel = match state.channels.get_mut(channel_name) {
        Some(channel) => channel,
        None => {
            responses.push(Message::from(Reply::NoSuchChannel {
                nick: nick.to_string(),
                channel: channel_name.to_string(),
            }));
            return Ok(());
        }
    };
    
    // Check if user is in the channel
    if !channel.is_member(connection_id) {
        responses.push(Message::from(Reply::NotOnChannel {
            nick: nick.to_string(),
            channel: channel_name.to_string(),
        }));
        return Ok(());
    }
    
    // If no mode parameters, return current channel modes
    if mode_params.is_empty() {
        let mode_string = format_channel_modes(&channel);
        responses.push(Message::from(Reply::ChannelModeIs {
            nick: nick.to_string(),
            channel: channel_name.to_string(),
            modes: mode_string,
            params: Vec::new(),
        }));
        return Ok(());
    }
    
    // Parse and apply mode changes
    let mode_string = mode_params[0].clone();
    let mut mode_args = mode_params[1..].to_vec();
    let mut arg_index = 0;
    
    let mut adding = true;
    let mut mode_changes = Vec::new();
    let mut param_changes = Vec::new();
    
    for ch in mode_string.chars() {
        match ch {
            '+' => adding = true,
            '-' => adding = false,
            'o' => {
                // Operator status - requires parameter
                if arg_index >= mode_args.len() {
                    responses.push(Message::from(Reply::NeedMoreParams {
                        nick: nick.to_string(),
                        command: "MODE".to_string(),
                    }));
                    return Ok(());
                }
                
                let target_nick = &mode_args[arg_index];
                arg_index += 1;
                
                if let Some(target_id) = state.nicknames.get(target_nick) {
                    let target_id = *target_id;
                    
                    if !channel.is_member(target_id) {
                        responses.push(Message::from(Reply::UserNotInChannel {
                            nick: nick.to_string(),
                            target: target_nick.clone(),
                            channel: channel_name.to_string(),
                        }));
                        continue;
                    }
                    
                    // Check if user has op privileges (only ops can change modes)
                    if !channel.is_operator(connection_id) {
                        responses.push(Message::from(Reply::ChanOpPrivsNeeded {
                            nick: nick.to_string(),
                            channel: channel_name.to_string(),
                        }));
                        return Ok(());
                    }
                    
                    // Apply operator mode change
                    if let Some(mut member) = channel.members.get_mut(&target_id) {
                        if adding {
                            if !member.modes.contains(&'o') {
                                member.modes.push('o');
                                mode_changes.push(format!("+o"));
                                param_changes.push(target_nick.clone());
                            }
                        } else {
                            if let Some(pos) = member.modes.iter().position(|&x| x == 'o') {
                                member.modes.remove(pos);
                                mode_changes.push(format!("-o"));
                                param_changes.push(target_nick.clone());
                            }
                        }
                    }
                } else {
                    responses.push(Message::from(Reply::NoSuchNick {
                        nick: nick.to_string(),
                        target: target_nick.clone(),
                    }));
                }
            },
            'v' => {
                // Voice status - requires parameter
                if arg_index >= mode_args.len() {
                    responses.push(Message::from(Reply::NeedMoreParams {
                        nick: nick.to_string(),
                        command: "MODE".to_string(),
                    }));
                    return Ok(());
                }
                
                let target_nick = &mode_args[arg_index];
                arg_index += 1;
                
                if let Some(target_id) = state.nicknames.get(target_nick) {
                    let target_id = *target_id;
                    
                    if !channel.is_member(target_id) {
                        responses.push(Message::from(Reply::UserNotInChannel {
                            nick: nick.to_string(),
                            target: target_nick.clone(),
                            channel: channel_name.to_string(),
                        }));
                        continue;
                    }
                    
                    // Check if user has op privileges
                    if !channel.is_operator(connection_id) {
                        responses.push(Message::from(Reply::ChanOpPrivsNeeded {
                            nick: nick.to_string(),
                            channel: channel_name.to_string(),
                        }));
                        return Ok(());
                    }
                    
                    // Apply voice mode change
                    if let Some(mut member) = channel.members.get_mut(&target_id) {
                        if adding {
                            if !member.modes.contains(&'v') {
                                member.modes.push('v');
                                mode_changes.push(format!("+v"));
                                param_changes.push(target_nick.clone());
                            }
                        } else {
                            if let Some(pos) = member.modes.iter().position(|&x| x == 'v') {
                                member.modes.remove(pos);
                                mode_changes.push(format!("-v"));
                                param_changes.push(target_nick.clone());
                            }
                        }
                    }
                } else {
                    responses.push(Message::from(Reply::NoSuchNick {
                        nick: nick.to_string(),
                        target: target_nick.clone(),
                    }));
                }
            },
            't' | 'n' | 'm' | 'i' | 's' | 'p' => {
                // Channel modes that don't require parameters
                if !channel.is_operator(connection_id) {
                    responses.push(Message::from(Reply::ChanOpPrivsNeeded {
                        nick: nick.to_string(),
                        channel: channel_name.to_string(),
                    }));
                    return Ok(());
                }
                
                if adding {
                    if !channel.modes.contains(&ch) {
                        channel.modes.push(ch);
                        mode_changes.push(format!("+{}", ch));
                    }
                } else {
                    if let Some(pos) = channel.modes.iter().position(|&x| x == ch) {
                        channel.modes.remove(pos);
                        mode_changes.push(format!("-{}", ch));
                    }
                }
            },
            'k' => {
                // Channel key - requires parameter
                if !channel.is_operator(connection_id) {
                    responses.push(Message::from(Reply::ChanOpPrivsNeeded {
                        nick: nick.to_string(),
                        channel: channel_name.to_string(),
                    }));
                    return Ok(());
                }
                
                if adding {
                    if arg_index >= mode_args.len() {
                        responses.push(Message::from(Reply::NeedMoreParams {
                            nick: nick.to_string(),
                            command: "MODE".to_string(),
                        }));
                        return Ok(());
                    }
                    
                    let key = mode_args[arg_index].clone();
                    arg_index += 1;
                    
                    channel.key = Some(key.clone());
                    if !channel.modes.contains(&'k') {
                        channel.modes.push('k');
                    }
                    mode_changes.push(format!("+k"));
                    param_changes.push(key);
                } else {
                    if arg_index < mode_args.len() {
                        let _old_key = &mode_args[arg_index];
                        arg_index += 1;
                    }
                    
                    channel.key = None;
                    if let Some(pos) = channel.modes.iter().position(|&x| x == 'k') {
                        channel.modes.remove(pos);
                    }
                    mode_changes.push(format!("-k"));
                    param_changes.push("*".to_string());
                }
            },
            'l' => {
                // User limit
                if !channel.is_operator(connection_id) {
                    responses.push(Message::from(Reply::ChanOpPrivsNeeded {
                        nick: nick.to_string(),
                        channel: channel_name.to_string(),
                    }));
                    return Ok(());
                }
                
                if adding {
                    if arg_index >= mode_args.len() {
                        responses.push(Message::from(Reply::NeedMoreParams {
                            nick: nick.to_string(),
                            command: "MODE".to_string(),
                        }));
                        return Ok(());
                    }
                    
                    let limit_str = &mode_args[arg_index];
                    arg_index += 1;
                    
                    if let Ok(limit) = limit_str.parse::<usize>() {
                        channel.limit = Some(limit);
                        if !channel.modes.contains(&'l') {
                            channel.modes.push('l');
                        }
                        mode_changes.push(format!("+l"));
                        param_changes.push(limit_str.clone());
                    }
                } else {
                    channel.limit = None;
                    if let Some(pos) = channel.modes.iter().position(|&x| x == 'l') {
                        channel.modes.remove(pos);
                    }
                    mode_changes.push(format!("-l"));
                }
            },
            _ => {
                // Unknown mode - silently ignore for now
            }
        }
    }
    
    // If we made any changes, broadcast MODE message to channel
    if !mode_changes.is_empty() {
        let connection = state.connections.get(&connection_id).unwrap();
        let user = connection.username.clone().unwrap_or_else(|| nick.to_string());
        let host = connection.hostname.clone();
        let prefix = format!("{}!{}@{}", nick, user, host);
        
        // Build mode change message
        let mode_str = mode_changes.join("");
        let mut params = vec![channel_name.to_string(), mode_str];
        params.extend(param_changes);
        
        let mode_msg = Message::new("MODE")
            .with_prefix(prefix)
            .with_params(params);
        
        // Send to all channel members
        for entry in channel.members.iter() {
            let member_id = *entry.key();
            if let Some(member_conn) = state.connections.get(&member_id) {
                let _ = member_conn.tx.send(mode_msg.clone()).await;
            }
        }
        
        // Also send to the mode setter
        responses.push(mode_msg);
    }
    
    Ok(())
}

fn format_channel_modes(channel: &crate::state::channel::Channel) -> String {
    if channel.modes.is_empty() {
        "+".to_string()
    } else {
        format!("+{}", channel.modes.iter().collect::<String>())
    }
}

#[cfg(test)]
#[path = "mode_test.rs"]
mod tests;
