use regex::Regex;
use once_cell::sync::Lazy;

static NICKNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z\[\]\\`_^{|}][a-zA-Z0-9\[\]\\`_^{|}-]{0,29}$").unwrap()
});

static CHANNEL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[#&][^\x00\x07\x0a\x0d ,:]{1,49}$").unwrap()
});

static VALID_USER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[^\x00\x0a\x0d @]+$").unwrap()
});

pub fn validate_nickname(nick: &str) -> bool {
    if nick.is_empty() || nick.len() > 30 {
        return false;
    }
    
    NICKNAME_REGEX.is_match(nick)
}

pub fn validate_channel_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 50 {
        return false;
    }
    
    CHANNEL_REGEX.is_match(name)
}

pub fn validate_username(username: &str) -> bool {
    if username.is_empty() || username.len() > 10 {
        return false;
    }
    
    VALID_USER_REGEX.is_match(username)
}

pub fn validate_realname(realname: &str) -> bool {
    if realname.is_empty() || realname.len() > 50 {
        return false;
    }
    
    // Realname can contain spaces but not control characters
    !realname.chars().any(|c| c.is_control() && c != ' ')
}

pub fn validate_message(message: &str) -> bool {
    if message.is_empty() || message.len() > 512 {
        return false;
    }
    
    // Messages cannot contain NULL, CR, or LF
    !message.chars().any(|c| matches!(c, '\x00' | '\r' | '\n'))
}

pub fn validate_topic(topic: &str) -> bool {
    if topic.len() > 390 {
        return false;
    }
    
    // Topics cannot contain NULL, CR, or LF
    !topic.chars().any(|c| matches!(c, '\x00' | '\r' | '\n'))
}

pub fn validate_away_message(message: &str) -> bool {
    if message.len() > 255 {
        return false;
    }
    
    // Away messages cannot contain NULL, CR, or LF
    !message.chars().any(|c| matches!(c, '\x00' | '\r' | '\n'))
}

pub fn validate_kick_reason(reason: &str) -> bool {
    if reason.len() > 255 {
        return false;
    }
    
    // Kick reasons cannot contain NULL, CR, or LF
    !reason.chars().any(|c| matches!(c, '\x00' | '\r' | '\n'))
}

pub fn validate_quit_message(message: &str) -> bool {
    if message.len() > 255 {
        return false;
    }
    
    // Quit messages cannot contain NULL, CR, or LF
    !message.chars().any(|c| matches!(c, '\x00' | '\r' | '\n'))
}

pub fn validate_channel_key(key: &str) -> bool {
    if key.is_empty() || key.len() > 23 {
        return false;
    }
    
    // Keys cannot contain spaces or control characters
    !key.chars().any(|c| c.is_whitespace() || c.is_control())
}

pub fn sanitize_message(message: &str) -> String {
    message
        .chars()
        .filter(|&c| !matches!(c, '\x00' | '\r' | '\n'))
        .take(512)
        .collect()
}

pub fn is_valid_host_mask(mask: &str) -> bool {
    // Basic validation for nick!user@host format
    let parts: Vec<&str> = mask.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    
    let user_parts: Vec<&str> = parts[0].split('!').collect();
    if user_parts.len() != 2 {
        return false;
    }
    
    let nick = user_parts[0];
    let user = user_parts[1];
    let host = parts[1];
    
    // Allow wildcards in mask
    let is_valid_mask_char = |c: char| {
        c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '*' | '?' | '[' | ']' | '{' | '}' | '\\' | '|')
    };
    
    nick.chars().all(is_valid_mask_char) &&
    user.chars().all(is_valid_mask_char) &&
    host.chars().all(is_valid_mask_char)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_nickname() {
        assert!(validate_nickname("Alice"));
        assert!(validate_nickname("Bob123"));
        assert!(validate_nickname("[Bot]"));
        assert!(validate_nickname("Test-Nick"));
        
        assert!(!validate_nickname(""));
        assert!(!validate_nickname("123Start"));
        assert!(!validate_nickname("Nick With Spaces"));
        assert!(!validate_nickname("Very_Long_Nickname_That_Exceeds_Limit"));
    }
    
    #[test]
    fn test_validate_channel_name() {
        assert!(validate_channel_name("#general"));
        assert!(validate_channel_name("&local"));
        assert!(validate_channel_name("#test-channel"));
        
        assert!(!validate_channel_name(""));
        assert!(!validate_channel_name("general"));
        assert!(!validate_channel_name("#channel with spaces"));
        assert!(!validate_channel_name("#very_long_channel_name_that_exceeds_the_fifty_character_limit"));
    }
    
    #[test]
    fn test_validate_message() {
        assert!(validate_message("Hello, world!"));
        assert!(validate_message("This is a test message"));
        
        assert!(!validate_message(""));
        assert!(!validate_message("Message with\nnewline"));
        assert!(!validate_message("Message with\rcarriage return"));
    }
}