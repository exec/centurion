use bytes::Bytes;
use legion_protocol::{IrcMessage, Command as ProtocolCommand, IronError};
use std::collections::HashMap;
use thiserror::Error;

pub mod codec;
pub mod commands;
// pub mod replies; // Now using legion-protocol
// pub mod capabilities; // Now using legion-protocol
pub mod extensions;

// 2024 Bleeding-edge IRCv3 capabilities - now using legion-protocol::bleeding_edge
// pub mod redaction; 
// pub mod multiline;
// pub mod read_marker;
// pub mod typing;

pub use self::codec::IrcCodec;
pub use self::commands::Command;

// Re-export legion-protocol types
pub use legion_protocol::{
    Capability, CapabilitySet, CapabilityHandler,
    constants, utils, Reply
};

pub use legion_protocol::sasl::{SaslAuth, SaslMechanism};

#[cfg(feature = "bleeding-edge")]
pub use legion_protocol::bleeding_edge;

// Use legion-protocol's IrcMessage directly
pub type Message = IrcMessage;

// Use legion-protocol's error types
pub type ProtocolError = IronError;

// Helper extension trait for Message conversion
pub trait MessageExt {
    fn to_bytes(&self) -> Bytes;
}

impl MessageExt for Message {
    fn to_bytes(&self) -> Bytes {
        Bytes::from(self.to_string())
    }
}

// From<Reply> for Message is implemented in legion-protocol