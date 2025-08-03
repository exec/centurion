use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

pub mod config;

pub fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn mask_ip(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            format!("{}.{}.{}.x", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            format!("{:x}:{:x}:{:x}:x:x:x:x:x", segments[0], segments[1], segments[2])
        }
    }
}

pub fn generate_message_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let timestamp = get_timestamp();
    let random: u32 = rng.gen();
    format!("{:x}{:08x}", timestamp, random)
}

pub fn parse_mode_string(modes: &str) -> (Vec<char>, Vec<char>) {
    let mut adding = true;
    let mut add_modes = Vec::new();
    let mut remove_modes = Vec::new();
    
    for ch in modes.chars() {
        match ch {
            '+' => adding = true,
            '-' => adding = false,
            mode => {
                if adding {
                    add_modes.push(mode);
                } else {
                    remove_modes.push(mode);
                }
            }
        }
    }
    
    (add_modes, remove_modes)
}

pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

pub fn is_channel(target: &str) -> bool {
    target.starts_with('#') || target.starts_with('&')
}

pub fn normalize_channel_name(name: &str) -> String {
    name.to_lowercase()
}

pub fn normalize_nickname(nick: &str) -> String {
    nick.to_lowercase()
}