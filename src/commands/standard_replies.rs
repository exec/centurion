/// IRCv3 Standard Replies implementation
/// Provides FAIL, WARN, and NOTE message types with standardized error codes

use crate::protocol::Message;

/// Standard reply types as defined by IRCv3
#[derive(Debug, Clone, PartialEq)]
pub enum StandardReplyType {
    /// FAIL - Indicates complete failure to process a command
    Fail,
    /// WARN - Indicates non-fatal feedback  
    Warn,
    /// NOTE - Provides informational messages
    Note,
}

impl StandardReplyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StandardReplyType::Fail => "FAIL",
            StandardReplyType::Warn => "WARN", 
            StandardReplyType::Note => "NOTE",
        }
    }
}

/// Standard reply codes as defined by IRCv3 specification
#[derive(Debug, Clone, PartialEq)]
pub enum StandardReplyCode {
    // General error codes
    /// Account is required to connect to the server
    AccountRequiredToConnect,
    /// Invalid parameters provided to command
    InvalidParams,
    /// Invalid target specified
    InvalidTarget,
    /// Need more parameters to execute command
    NeedMoreParams,
    /// Message could not be retrieved
    MessageError,
    /// Unknown error occurred
    UnknownError,
    
    // Channel-related error codes
    /// Cannot send to channel (banned, not joined, etc.)
    CannotSendToChan,
    /// No such channel exists
    NoSuchChannel,
    /// Not on channel
    NotOnChannel,
    /// Channel is invite-only
    InviteOnlyChan,
    /// Channel key required
    BadChannelKey,
    /// Channel is full
    ChannelIsFull,
    /// Banned from channel
    BannedFromChan,
    
    // User-related error codes
    /// No such nick/user exists
    NoSuchNick,
    /// Nickname in use
    NicknameInUse,
    /// Invalid nickname
    InvalidNickname,
    
    // Authentication-related error codes
    /// SASL authentication failed
    SaslFail,
    /// Invalid credentials provided
    InvalidCredentials,
    /// Registration failed
    RegFail,
    /// Account registration callback invalid
    RegInvalidCallback,
    /// Account already registered
    AlreadyRegistered,
    
    // Command-related error codes
    /// Unknown command
    UnknownCommand,
    /// Command disabled
    CommandDisabled,
    
    // Rate limiting and resource errors
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Resource limit exceeded
    ResourceLimit,
    
    // Custom error code
    Custom(String),
}

impl StandardReplyCode {
    pub fn as_str(&self) -> &str {
        match self {
            StandardReplyCode::AccountRequiredToConnect => "ACCOUNT_REQUIRED_TO_CONNECT",
            StandardReplyCode::InvalidParams => "INVALID_PARAMS",
            StandardReplyCode::InvalidTarget => "INVALID_TARGET",
            StandardReplyCode::NeedMoreParams => "NEED_MORE_PARAMS",
            StandardReplyCode::MessageError => "MESSAGE_ERROR",
            StandardReplyCode::UnknownError => "UNKNOWN_ERROR",
            
            StandardReplyCode::CannotSendToChan => "CANNOT_SEND_TO_CHAN",
            StandardReplyCode::NoSuchChannel => "NO_SUCH_CHANNEL",
            StandardReplyCode::NotOnChannel => "NOT_ON_CHANNEL",
            StandardReplyCode::InviteOnlyChan => "INVITE_ONLY_CHAN",
            StandardReplyCode::BadChannelKey => "BAD_CHANNEL_KEY",
            StandardReplyCode::ChannelIsFull => "CHANNEL_IS_FULL",
            StandardReplyCode::BannedFromChan => "BANNED_FROM_CHAN",
            
            StandardReplyCode::NoSuchNick => "NO_SUCH_NICK",
            StandardReplyCode::NicknameInUse => "NICKNAME_IN_USE",
            StandardReplyCode::InvalidNickname => "INVALID_NICKNAME",
            
            StandardReplyCode::SaslFail => "SASL_FAIL",
            StandardReplyCode::InvalidCredentials => "INVALID_CREDENTIALS",
            StandardReplyCode::RegFail => "REG_FAIL",
            StandardReplyCode::RegInvalidCallback => "REG_INVALID_CALLBACK",
            StandardReplyCode::AlreadyRegistered => "ALREADY_REGISTERED",
            
            StandardReplyCode::UnknownCommand => "UNKNOWN_COMMAND",
            StandardReplyCode::CommandDisabled => "COMMAND_DISABLED",
            
            StandardReplyCode::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            StandardReplyCode::ResourceLimit => "RESOURCE_LIMIT",
            
            StandardReplyCode::Custom(code) => code,
        }
    }
}

/// Builder for creating standard replies
pub struct StandardReply {
    reply_type: StandardReplyType,
    command: String,
    code: StandardReplyCode,
    context: Vec<String>,
    description: String,
}

impl StandardReply {
    /// Create a new FAIL reply
    pub fn fail(command: &str, code: StandardReplyCode, description: &str) -> Self {
        Self {
            reply_type: StandardReplyType::Fail,
            command: command.to_string(),
            code,
            context: Vec::new(),
            description: description.to_string(),
        }
    }
    
    /// Create a new WARN reply
    pub fn warn(command: &str, code: StandardReplyCode, description: &str) -> Self {
        Self {
            reply_type: StandardReplyType::Warn,
            command: command.to_string(),
            code,
            context: Vec::new(),
            description: description.to_string(),
        }
    }
    
    /// Create a new NOTE reply
    pub fn note(command: &str, code: StandardReplyCode, description: &str) -> Self {
        Self {
            reply_type: StandardReplyType::Note,
            command: command.to_string(),
            code,
            context: Vec::new(),
            description: description.to_string(),
        }
    }
    
    /// Add context parameters to the reply
    pub fn with_context(mut self, context: Vec<String>) -> Self {
        self.context = context;
        self
    }
    
    /// Add a single context parameter
    pub fn add_context(mut self, context: String) -> Self {
        self.context.push(context);
        self
    }
    
    /// Convert to an IRC Message
    pub fn to_message(&self, server_name: &str) -> Message {
        let mut params = vec![
            self.command.clone(),
            self.code.as_str().to_string(),
        ];
        
        // Add context parameters
        params.extend(self.context.clone());
        
        // Add description as the final parameter
        params.push(self.description.clone());
        
        Message::new(self.reply_type.as_str())
            .with_prefix(server_name.to_string())
            .with_params(params)
    }
}

/// Convenience functions for common standard replies
pub mod common {
    use super::*;
    
    /// Create a FAIL reply for invalid parameters
    pub fn invalid_params(command: &str, description: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::InvalidParams, description)
    }
    
    /// Create a FAIL reply for invalid target
    pub fn invalid_target(command: &str, target: &str, description: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::InvalidTarget, description)
            .add_context(target.to_string())
    }
    
    /// Create a FAIL reply for no such channel
    pub fn no_such_channel(command: &str, channel: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::NoSuchChannel, "No such channel")
            .add_context(channel.to_string())
    }
    
    /// Create a FAIL reply for no such nick
    pub fn no_such_nick(command: &str, nick: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::NoSuchNick, "No such nick/channel")
            .add_context(nick.to_string())
    }
    
    /// Create a FAIL reply for not on channel
    pub fn not_on_channel(command: &str, channel: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::NotOnChannel, "You're not on that channel")
            .add_context(channel.to_string())
    }
    
    /// Create a FAIL reply for cannot send to channel
    pub fn cannot_send_to_chan(command: &str, channel: &str, reason: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::CannotSendToChan, reason)
            .add_context(channel.to_string())
    }
    
    /// Create a FAIL reply for nickname in use
    pub fn nickname_in_use(command: &str, nick: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::NicknameInUse, "Nickname is already in use")
            .add_context(nick.to_string())
    }
    
    /// Create a FAIL reply for invalid nickname
    pub fn invalid_nickname(command: &str, nick: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::InvalidNickname, "Invalid nickname")
            .add_context(nick.to_string())
    }
    
    /// Create a FAIL reply for unknown command
    pub fn unknown_command(command: &str) -> StandardReply {
        StandardReply::fail("*", StandardReplyCode::UnknownCommand, "Unknown command")
            .add_context(command.to_string())
    }
    
    /// Create a FAIL reply for rate limit exceeded
    pub fn rate_limit_exceeded(command: &str) -> StandardReply {
        StandardReply::fail(command, StandardReplyCode::RateLimitExceeded, "Rate limit exceeded")
    }
    
    /// Create a WARN reply for deprecated command
    pub fn deprecated_command(command: &str, alternative: &str) -> StandardReply {
        StandardReply::warn(command, StandardReplyCode::Custom("DEPRECATED".to_string()), 
                          &format!("Command deprecated, use {} instead", alternative))
    }
    
    /// Create a NOTE reply for informational message
    pub fn info_note(command: &str, message: &str) -> StandardReply {
        StandardReply::note(command, StandardReplyCode::Custom("INFO".to_string()), message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_standard_reply_creation() {
        let reply = StandardReply::fail("JOIN", StandardReplyCode::NoSuchChannel, "No such channel")
            .add_context("#nonexistent".to_string());
        
        let message = reply.to_message("test.server");
        
        assert_eq!(message.command, "FAIL");
        assert_eq!(message.params, vec![
            "JOIN".to_string(),
            "NO_SUCH_CHANNEL".to_string(),
            "#nonexistent".to_string(),
            "No such channel".to_string()
        ]);
    }
    
    #[test]
    fn test_common_replies() {
        let reply = common::invalid_params("PRIVMSG", "Not enough parameters");
        let message = reply.to_message("test.server");
        
        assert_eq!(message.command, "FAIL");
        assert!(message.params.contains(&"INVALID_PARAMS".to_string()));
        assert!(message.params.contains(&"Not enough parameters".to_string()));
    }
}