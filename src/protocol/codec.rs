use bytes::{Buf, BufMut, BytesMut};
use legion_protocol::IrcMessage;
use std::io;
use tokio_util::codec::{Decoder, Encoder};
use tracing::debug;

use super::{Message, MessageExt};

const MAX_MESSAGE_LENGTH: usize = 8191;

pub struct IrcCodec {
    max_length: usize,
}

impl IrcCodec {
    pub fn new() -> Self {
        Self {
            max_length: MAX_MESSAGE_LENGTH,
        }
    }
}

impl Default for IrcCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for IrcCodec {
    type Item = Message;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(idx) = buf.iter().position(|&b| b == b'\n') {
            let line = buf.split_to(idx + 1);
            
            let line = if line.ends_with(b"\r\n") {
                &line[..line.len() - 2]
            } else if line.ends_with(b"\n") {
                &line[..line.len() - 1]
            } else {
                &line[..]
            };

            let line_str = std::str::from_utf8(line)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"))?;

            debug!("Received: {}", line_str);

            match line_str.parse::<IrcMessage>() {
                Ok(msg) => Ok(Some(msg.into())),
                Err(e) => {
                    debug!("Failed to parse message: {}", e);
                    Err(io::Error::new(io::ErrorKind::InvalidData, e))
                }
            }
        } else if buf.len() > self.max_length {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message too long",
            ))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<Message> for IrcCodec {
    type Error = io::Error;

    fn encode(&mut self, msg: Message, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = msg.to_bytes();
        
        if bytes.len() > self.max_length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Message too long",
            ));
        }

        debug!("Sending: {}", String::from_utf8_lossy(&bytes).trim_end());
        buf.put(bytes);
        Ok(())
    }
}