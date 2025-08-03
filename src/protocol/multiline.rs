use std::collections::HashMap;
use thiserror::Error;

use crate::protocol::{Message, extensions::Batch};

#[derive(Error, Debug)]
pub enum MultilineError {
    #[error("Maximum bytes exceeded")]
    MaxBytesExceeded,
    
    #[error("Maximum lines exceeded")]
    MaxLinesExceeded,
    
    #[error("Invalid target")]
    InvalidTarget,
    
    #[error("Invalid multiline batch")]
    InvalidBatch,
    
    #[error("Multiline not supported")]
    NotSupported,
}

#[derive(Debug, Clone)]
pub struct MultilineCapability {
    pub max_bytes: usize,
    pub max_lines: Option<usize>,
}

impl MultilineCapability {
    pub fn new(max_bytes: usize, max_lines: Option<usize>) -> Self {
        Self { max_bytes, max_lines }
    }
    
    pub fn to_cap_value(&self) -> String {
        let mut value = format!("max-bytes={}", self.max_bytes);
        if let Some(lines) = self.max_lines {
            value.push_str(&format!(",max-lines={}", lines));
        }
        value
    }
    
    pub fn from_cap_value(value: &str) -> Option<Self> {
        let mut max_bytes = None;
        let mut max_lines = None;
        
        for param in value.split(',') {
            if let Some((key, val)) = param.split_once('=') {
                match key {
                    "max-bytes" => {
                        max_bytes = val.parse().ok();
                    }
                    "max-lines" => {
                        max_lines = val.parse().ok();
                    }
                    _ => {}
                }
            }
        }
        
        max_bytes.map(|bytes| Self::new(bytes, max_lines))
    }
}

#[derive(Debug, Clone)]
pub struct MultilineBatch {
    pub reference: String,
    pub target: String,
    pub command: String, // PRIVMSG or NOTICE
    pub lines: Vec<MultilinePart>,
    pub total_bytes: usize,
    pub tags: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
pub struct MultilinePart {
    pub content: String,
    pub concat: bool, // true if has draft/multiline-concat tag
}

impl MultilineBatch {
    pub fn new(reference: String, target: String, command: String) -> Self {
        Self {
            reference,
            target,
            command,
            lines: Vec::new(),
            total_bytes: 0,
            tags: HashMap::new(),
        }
    }
    
    pub fn add_line(&mut self, content: String, concat: bool) -> Result<(), MultilineError> {
        let part = MultilinePart { content, concat };
        self.total_bytes += part.content.len();
        self.lines.push(part);
        Ok(())
    }
    
    pub fn validate(&self, capability: &MultilineCapability) -> Result<(), MultilineError> {
        // Check byte limit
        if self.total_bytes > capability.max_bytes {
            return Err(MultilineError::MaxBytesExceeded);
        }
        
        // Check line limit if specified
        if let Some(max_lines) = capability.max_lines {
            if self.lines.len() > max_lines {
                return Err(MultilineError::MaxLinesExceeded);
            }
        }
        
        // Validate command type
        if !matches!(self.command.as_str(), "PRIVMSG" | "NOTICE") {
            return Err(MultilineError::InvalidBatch);
        }
        
        // Validate target
        if self.target.is_empty() {
            return Err(MultilineError::InvalidTarget);
        }
        
        Ok(())
    }
    
    pub fn compose_message(&self) -> String {
        let mut result = String::new();
        
        for (i, part) in self.lines.iter().enumerate() {
            if part.concat {
                // Direct concatenation without separator
                result.push_str(&part.content);
            } else {
                // Add line feed separator unless it's the first line
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(&part.content);
            }
        }
        
        result
    }
    
    pub fn split_for_fallback(&self, max_line_length: usize) -> Vec<Message> {
        let composed = self.compose_message();
        let lines: Vec<&str> = composed.split('\n').collect();
        let mut messages = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            let mut msg = Message::new(&self.command)
                .add_param(self.target.clone())
                .add_param(line.to_string());
            
            // Add tags to first message only
            if i == 0 {
                for (key, value) in &self.tags {
                    msg = msg.add_tag(key.clone(), value.clone());
                }
            }
            
            messages.push(msg);
        }
        
        messages
    }
}

pub struct MultilineProcessor {
    capability: MultilineCapability,
    active_batches: HashMap<String, MultilineBatch>,
}

impl MultilineProcessor {
    pub fn new(capability: MultilineCapability) -> Self {
        Self {
            capability,
            active_batches: HashMap::new(),
        }
    }
    
    pub fn start_batch(
        &mut self,
        reference: String,
        target: String,
        command: String,
        tags: HashMap<String, Option<String>>,
    ) -> Result<(), MultilineError> {
        // Validate target is single recipient
        if target.contains(',') {
            return Err(MultilineError::InvalidTarget);
        }
        
        let mut batch = MultilineBatch::new(reference.clone(), target, command);
        batch.tags = tags;
        
        self.active_batches.insert(reference, batch);
        Ok(())
    }
    
    pub fn add_batch_line(
        &mut self,
        reference: &str,
        message: Message,
    ) -> Result<(), MultilineError> {
        let batch = self.active_batches.get_mut(reference)
            .ok_or(MultilineError::InvalidBatch)?;
        
        // Extract content from message params
        let content = message.params.last()
            .cloned()
            .unwrap_or_default();
        
        // Check for concat tag
        let concat = message.tags.contains_key("draft/multiline-concat");
        
        batch.add_line(content, concat)?;
        
        // Validate after each addition
        batch.validate(&self.capability)?;
        
        Ok(())
    }
    
    pub fn end_batch(&mut self, reference: &str) -> Result<MultilineBatch, MultilineError> {
        let batch = self.active_batches.remove(reference)
            .ok_or(MultilineError::InvalidBatch)?;
        
        batch.validate(&self.capability)?;
        Ok(batch)
    }
    
    pub fn create_multiline_message(&self, batch: &MultilineBatch) -> Message {
        let content = batch.compose_message();
        
        let mut msg = Message::new(&batch.command)
            .add_param(batch.target.clone())
            .add_param(content);
        
        // Add original tags
        for (key, value) in &batch.tags {
            msg = msg.add_tag(key.clone(), value.clone());
        }
        
        msg
    }
    
    pub fn get_capability(&self) -> &MultilineCapability {
        &self.capability
    }
}

pub fn create_multiline_fail_message(
    error: MultilineError,
    context: &str,
) -> Message {
    let error_code = match error {
        MultilineError::MaxBytesExceeded => "MULTILINE_MAX_BYTES",
        MultilineError::MaxLinesExceeded => "MULTILINE_MAX_LINES",
        MultilineError::InvalidTarget => "MULTILINE_INVALID_TARGET",
        MultilineError::InvalidBatch => "MULTILINE_INVALID",
        MultilineError::NotSupported => "MULTILINE_INVALID",
    };
    
    Message::new("FAIL")
        .add_param("BATCH")
        .add_param(error_code)
        .add_param(context.to_string())
        .add_param(error.to_string())
}

pub fn is_multiline_batch(batch_type: &str) -> bool {
    batch_type == "draft/multiline"
}

pub fn validate_multiline_message(message: &Message) -> Result<(), MultilineError> {
    // Only PRIVMSG and NOTICE are supported in multiline batches
    if !matches!(message.command.as_str(), "PRIVMSG" | "NOTICE") {
        return Err(MultilineError::InvalidBatch);
    }
    
    // Must have at least target and content parameters
    if message.params.len() < 2 {
        return Err(MultilineError::InvalidBatch);
    }
    
    // Only certain tags are allowed in multiline batches
    for key in message.tags.keys() {
        if !matches!(key.as_str(), "draft/multiline-concat" | "batch") {
            return Err(MultilineError::InvalidBatch);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multiline_capability_parsing() {
        let cap = MultilineCapability::from_cap_value("max-bytes=4096,max-lines=100").unwrap();
        assert_eq!(cap.max_bytes, 4096);
        assert_eq!(cap.max_lines, Some(100));
        
        let cap_value = cap.to_cap_value();
        assert!(cap_value.contains("max-bytes=4096"));
        assert!(cap_value.contains("max-lines=100"));
    }
    
    #[test]
    fn test_multiline_batch_composition() {
        let mut batch = MultilineBatch::new(
            "ref123".to_string(),
            "#channel".to_string(),
            "PRIVMSG".to_string(),
        );
        
        batch.add_line("Line 1".to_string(), false).unwrap();
        batch.add_line("Line 2".to_string(), false).unwrap();
        batch.add_line(" continued".to_string(), true).unwrap();
        
        let composed = batch.compose_message();
        assert_eq!(composed, "Line 1\nLine 2 continued");
    }
    
    #[test]
    fn test_multiline_validation() {
        let capability = MultilineCapability::new(100, Some(3));
        let mut batch = MultilineBatch::new(
            "ref123".to_string(),
            "#channel".to_string(),
            "PRIVMSG".to_string(),
        );
        
        // Should pass validation when empty
        batch.validate(&capability).unwrap();
        
        // Add content that exceeds byte limit
        batch.add_line("x".repeat(101), false).unwrap();
        assert!(matches!(batch.validate(&capability), Err(MultilineError::MaxBytesExceeded)));
    }
}