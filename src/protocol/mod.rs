use bytes::Bytes;
use iron_protocol::{IrcMessage, Command as ProtocolCommand, IronError};
use std::collections::HashMap;
use thiserror::Error;

pub mod codec;
pub mod commands;
// pub mod replies; // Now using iron-protocol
// pub mod capabilities; // Now using iron-protocol
pub mod extensions;

// 2024 Bleeding-edge IRCv3 capabilities - now using iron-protocol::bleeding_edge
// pub mod redaction; 
// pub mod multiline;
// pub mod read_marker;
// pub mod typing;

pub use self::codec::IrcCodec;
pub use self::commands::Command;

// Re-export iron-protocol types
pub use iron_protocol::{
    Capability, CapabilitySet, CapabilityHandler,
    constants, utils, Reply
};

pub use iron_protocol::sasl::{SaslAuth, SaslMechanism};

#[cfg(feature = "bleeding-edge")]
pub use iron_protocol::bleeding_edge;

// Use iron-protocol's IrcMessage directly
pub type Message = IrcMessage;

// Use iron-protocol's error types
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

// From<Reply> for Message is implemented in iron-protocol