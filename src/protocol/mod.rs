use bytes::Bytes;
use irc_proto::message::Message as IrcMessage;
use std::collections::HashMap;
use thiserror::Error;

pub mod codec;
pub mod commands;
pub mod replies;
pub mod capabilities;
pub mod extensions;

// 2024 Bleeding-edge IRCv3 capabilities
pub mod redaction;
pub mod multiline;
pub mod read_marker;
pub mod typing;

pub use self::codec::IrcCodec;
pub use self::commands::Command;
pub use self::replies::Reply;

#[derive(Debug, Clone)]
pub struct Message {
    pub tags: HashMap<String, Option<String>>,
    pub prefix: Option<String>,
    pub command: String,
    pub params: Vec<String>,
}

impl Message {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            tags: HashMap::new(),
            prefix: None,
            command: command.into(),
            params: Vec::new(),
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn with_params(mut self, params: Vec<String>) -> Self {
        self.params = params;
        self
    }

    pub fn add_param(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self
    }

    pub fn add_tag(mut self, key: impl Into<String>, value: Option<String>) -> Self {
        self.tags.insert(key.into(), value);
        self
    }

    pub fn to_bytes(&self) -> Bytes {
        let mut output = String::new();

        if !self.tags.is_empty() {
            output.push('@');
            let tags: Vec<String> = self.tags
                .iter()
                .map(|(k, v)| match v {
                    Some(val) => format!("{}={}", k, escape_tag_value(val)),
                    None => k.clone(),
                })
                .collect();
            output.push_str(&tags.join(";"));
            output.push(' ');
        }

        if let Some(ref prefix) = self.prefix {
            output.push(':');
            output.push_str(prefix);
            output.push(' ');
        }

        output.push_str(&self.command);

        for (i, param) in self.params.iter().enumerate() {
            output.push(' ');
            if i == self.params.len() - 1 && (param.contains(' ') || param.starts_with(':')) {
                output.push(':');
            }
            output.push_str(param);
        }

        output.push_str("\r\n");
        Bytes::from(output)
    }
}

impl From<IrcMessage> for Message {
    fn from(msg: IrcMessage) -> Self {
        // Simplified conversion for now
        Self {
            tags: HashMap::new(), // Skip tags for now
            prefix: msg.prefix.map(|p| format!("{}", p)),
            command: format!("{:?}", msg.command), // Use Debug formatting for now
            params: vec![], // Will be populated from command arguments
        }
    }
}

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid message format")]
    InvalidFormat,
    
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    
    #[error("Invalid parameters")]
    InvalidParameters,
    
    #[error("Message too long")]
    MessageTooLong,
    
    #[error("Invalid encoding")]
    InvalidEncoding,
}

fn escape_tag_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(';', "\\:")
        .replace(' ', "\\s")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}