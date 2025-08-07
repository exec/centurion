use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Connection registration
    Nick(String),
    User { username: String, realname: String },
    Pass(String),
    Quit(Option<String>),
    Ping(String),
    Pong(String),
    
    // Channel operations
    Join(Vec<String>, Vec<String>), // channels, keys
    Part(Vec<String>, Option<String>), // channels, message
    Topic { channel: String, topic: Option<String> },
    Names(Vec<String>),
    List(Option<Vec<String>>),
    
    // Messaging
    Privmsg { target: String, message: String },
    Notice { target: String, message: String },
    
    // User queries
    Who(Option<String>),
    Whois(Vec<String>),
    Whowas(String, Option<i32>),
    
    // Channel management
    Kick { channel: String, user: String, reason: Option<String> },
    Mode { target: String, modes: Option<String>, params: Vec<String> },
    Invite { nick: String, channel: String },
    
    // Server queries
    Motd(Option<String>),
    Version(Option<String>),
    Stats(Option<String>, Option<String>),
    Time(Option<String>),
    Info(Option<String>),
    
    // IRCv3 commands
    Cap { subcommand: String, params: Vec<String> },
    Authenticate(String),
    Account(String),
    Monitor { subcommand: String, targets: Vec<String> },
    Metadata { target: String, subcommand: String, params: Vec<String> },
    TagMsg { target: String },
    Batch { reference: String, batch_type: Option<String>, params: Vec<String> },
    
    // 2024 Bleeding-edge IRCv3 commands
    Redact { target: String, msgid: String, reason: Option<String> },
    MarkRead { target: String, timestamp: Option<String> },
    SetName { realname: String },
    ChatHistory { subcommand: String, target: String, params: Vec<String> },
    
    // Operator commands
    Oper { name: String, password: String },
    Kill { nick: String, reason: String },
    Rehash,
    Restart,
    Die,
    
    // CTCP
    CtcpRequest { target: String, command: String, params: String },
    CtcpResponse { target: String, command: String, params: String },
    
    // Other
    Unknown(String, Vec<String>),
}

impl Command {
    pub fn parse(command: &str, params: Vec<String>) -> Self {
        match command.to_uppercase().as_str() {
            "NICK" => {
                if let Some(nick) = params.first() {
                    Command::Nick(nick.clone())
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "USER" => {
                if params.len() >= 4 {
                    Command::User {
                        username: params[0].clone(),
                        realname: params[3].clone(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "PASS" => {
                if let Some(pass) = params.first() {
                    Command::Pass(pass.clone())
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "QUIT" => Command::Quit(params.first().cloned()),
            "PING" => {
                if let Some(token) = params.first() {
                    Command::Ping(token.clone())
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "PONG" => {
                if let Some(token) = params.first() {
                    Command::Pong(token.clone())
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "JOIN" => {
                if let Some(channels) = params.first() {
                    let channels: Vec<String> = channels.split(',').map(|s| s.to_string()).collect();
                    let keys: Vec<String> = params.get(1)
                        .map(|k| k.split(',').map(|s| s.to_string()).collect())
                        .unwrap_or_default();
                    Command::Join(channels, keys)
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "PART" => {
                if let Some(channels) = params.first() {
                    let channels: Vec<String> = channels.split(',').map(|s| s.to_string()).collect();
                    let message = params.get(1).cloned();
                    Command::Part(channels, message)
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "PRIVMSG" => {
                if params.len() >= 2 {
                    Command::Privmsg {
                        target: params[0].clone(),
                        message: params[1].clone(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "NOTICE" => {
                if params.len() >= 2 {
                    Command::Notice {
                        target: params[0].clone(),
                        message: params[1].clone(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "TAGMSG" => {
                if let Some(target) = params.first() {
                    Command::TagMsg {
                        target: target.clone(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "CAP" => {
                if let Some(subcommand) = params.first() {
                    Command::Cap {
                        subcommand: subcommand.clone(),
                        params: params[1..].to_vec(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "AUTHENTICATE" => {
                if let Some(data) = params.first() {
                    Command::Authenticate(data.clone())
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "WHOIS" => {
                if !params.is_empty() {
                    Command::Whois(params)
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "REDACT" => {
                if params.len() >= 2 {
                    Command::Redact {
                        target: params[0].clone(),
                        msgid: params[1].clone(),
                        reason: params.get(2).cloned(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "MARKREAD" => {
                if !params.is_empty() {
                    Command::MarkRead {
                        target: params[0].clone(),
                        timestamp: params.get(1).cloned(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "SETNAME" => {
                if let Some(realname) = params.first() {
                    Command::SetName {
                        realname: realname.clone(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            "CHATHISTORY" => {
                if params.len() >= 2 {
                    Command::ChatHistory {
                        subcommand: params[0].clone(),
                        target: params[1].clone(),
                        params: params[2..].to_vec(),
                    }
                } else {
                    Command::Unknown(command.to_string(), params)
                }
            }
            _ => Command::Unknown(command.to_string(), params),
        }
    }
}