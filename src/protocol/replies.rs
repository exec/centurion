use super::Message;

#[derive(Debug, Clone)]
pub enum Reply {
    // Command responses
    Welcome { nick: String, network: String },
    YourHost { nick: String, servername: String, version: String },
    Created { nick: String, date: String },
    MyInfo { nick: String, servername: String, version: String, usermodes: String, chanmodes: String },
    ISupport { nick: String, tokens: Vec<String> },
    
    // Errors
    NoSuchNick { nick: String, target: String },
    NoSuchServer { nick: String, server: String },
    NoSuchChannel { nick: String, channel: String },
    CannotSendToChan { nick: String, channel: String },
    TooManyChannels { nick: String, channel: String },
    WasNoSuchNick { nick: String, target: String },
    TooManyTargets { nick: String, target: String },
    NoOrigin { nick: String },
    NoRecipient { nick: String, command: String },
    NoTextToSend { nick: String },
    NoTopLevel { nick: String, mask: String },
    WildTopLevel { nick: String, mask: String },
    UnknownCommand { nick: String, command: String },
    NoMotd { nick: String },
    NoAdminInfo { nick: String },
    FileError { nick: String, operation: String, file: String },
    NoNicknameGiven { nick: String },
    ErroneousNickname { nick: String, attempted: String },
    NicknameInUse { nick: String, attempted: String },
    NickCollision { nick: String, attempted: String },
    UserNotInChannel { nick: String, target: String, channel: String },
    NotOnChannel { nick: String, channel: String },
    UserOnChannel { nick: String, target: String, channel: String },
    NoLogin { nick: String },
    SummonDisabled { nick: String },
    UsersDisabled { nick: String },
    NotRegistered { nick: String },
    NeedMoreParams { nick: String, command: String },
    AlreadyRegistered { nick: String },
    NoPermForHost { nick: String },
    PasswdMismatch { nick: String },
    YoureBannedCreep { nick: String },
    KeySet { nick: String, channel: String },
    ChannelIsFull { nick: String, channel: String },
    UnknownMode { nick: String, char: char },
    InviteOnlyChan { nick: String, channel: String },
    BannedFromChan { nick: String, channel: String },
    BadChannelKey { nick: String, channel: String },
    NoPrivileges { nick: String },
    ChanOpPrivsNeeded { nick: String, channel: String },
    CantKillServer { nick: String },
    NoOperHost { nick: String },
    UmodeUnknownFlag { nick: String },
    UsersDontMatch { nick: String },
    
    // Numeric replies
    None { nick: String },
    UserHost { nick: String, replies: Vec<String> },
    Ison { nick: String, nicks: Vec<String> },
    Away { nick: String, target: String, message: String },
    Unaway { nick: String },
    NowAway { nick: String },
    WhoisUser { nick: String, target: String, username: String, host: String, realname: String },
    WhoisServer { nick: String, target: String, server: String, info: String },
    WhoisOperator { nick: String, target: String },
    WhoisIdle { nick: String, target: String, idle: u64, signon: u64 },
    EndOfWhois { nick: String, target: String },
    WhoisChannels { nick: String, target: String, channels: Vec<String> },
    WhoReply { nick: String, channel: String, username: String, host: String, server: String, target: String, flags: String, realname: String },
    EndOfWho { nick: String, target: String },
    ListStart { nick: String },
    List { nick: String, channel: String, visible: usize, topic: String },
    ListEnd { nick: String },
    ChannelModeIs { nick: String, channel: String, modes: String, params: Vec<String> },
    Topic { nick: String, channel: String, topic: String },
    NoTopic { nick: String, channel: String },
    Inviting { nick: String, target: String, channel: String },
    Version { nick: String, version: String, server: String, comments: String },
    NamReply { nick: String, symbol: char, channel: String, names: Vec<String> },
    EndOfNames { nick: String, channel: String },
    MotdStart { nick: String, server: String },
    Motd { nick: String, line: String },
    EndOfMotd { nick: String },
}

impl Reply {
    pub fn to_message(&self, server_name: &str) -> Message {
        match self {
            Reply::Welcome { nick, network } => {
                Message::new("001")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        format!("Welcome to the {} IRC Network, {}", network, nick),
                    ])
            }
            Reply::YourHost { nick, servername, version } => {
                Message::new("002")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        format!("Your host is {}, running version {}", servername, version),
                    ])
            }
            Reply::Created { nick, date } => {
                Message::new("003")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        format!("This server was created {}", date),
                    ])
            }
            Reply::MyInfo { nick, servername, version, usermodes, chanmodes } => {
                Message::new("004")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        servername.clone(),
                        version.clone(),
                        usermodes.clone(),
                        chanmodes.clone(),
                    ])
            }
            Reply::ISupport { nick, tokens } => {
                let mut params = vec![nick.clone()];
                params.extend(tokens.clone());
                params.push("are supported by this server".to_string());
                Message::new("005")
                    .with_prefix(server_name)
                    .with_params(params)
            }
            Reply::NoSuchNick { nick, target } => {
                Message::new("401")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        target.clone(),
                        "No such nick/channel".to_string(),
                    ])
            }
            Reply::NicknameInUse { nick, attempted } => {
                Message::new("433")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        attempted.clone(),
                        "Nickname is already in use".to_string(),
                    ])
            }
            Reply::NeedMoreParams { nick, command } => {
                Message::new("461")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        command.clone(),
                        "Not enough parameters".to_string(),
                    ])
            }
            Reply::NoSuchChannel { nick, channel } => {
                Message::new("403")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        channel.clone(),
                        "No such channel".to_string(),
                    ])
            }
            Reply::Topic { nick, channel, topic } => {
                Message::new("332")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        channel.clone(),
                        topic.clone(),
                    ])
            }
            Reply::NoTopic { nick, channel } => {
                Message::new("331")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        channel.clone(),
                        "No topic is set".to_string(),
                    ])
            }
            Reply::NamReply { nick, symbol, channel, names } => {
                let mut params = vec![
                    nick.clone(),
                    symbol.to_string(),
                    channel.clone(),
                ];
                // Join all names into a single string
                let names_str = names.join(" ");
                params.push(names_str);
                Message::new("353")
                    .with_prefix(server_name)
                    .with_params(params)
            }
            Reply::EndOfNames { nick, channel } => {
                Message::new("366")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        channel.clone(),
                        "End of /NAMES list".to_string(),
                    ])
            }
            Reply::MotdStart { nick, server } => {
                Message::new("375")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        format!("- {} Message of the day -", server),
                    ])
            }
            Reply::Motd { nick, line } => {
                Message::new("372")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        format!("- {}", line),
                    ])
            }
            Reply::EndOfMotd { nick } => {
                Message::new("376")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        "End of /MOTD command".to_string(),
                    ])
            }
            Reply::NoMotd { nick } => {
                Message::new("422")
                    .with_prefix(server_name)
                    .with_params(vec![
                        nick.clone(),
                        "MOTD File is missing".to_string(),
                    ])
            }
            _ => {
                Message::new("NOTICE")
                    .with_prefix(server_name)
                    .with_params(vec!["*".to_string(), "Reply not implemented".to_string()])
            }
        }
    }
}