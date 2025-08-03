use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    // Core IRCv3 capabilities (Ratified)
    MessageTags,
    ServerTime,
    AccountNotify,
    AccountTag,
    AwayNotify,
    Batch,
    CapNotify,
    ChgHost,
    EchoMessage,
    ExtendedJoin,
    InviteNotify,
    LabeledResponse,
    Monitor,
    MultiPrefix,
    Sasl,
    Setname,
    StandardReplies,
    UserhostInNames,
    BotMode,
    UTF8Only,
    StrictTransportSecurity,
    WebIRC,
    Chathistory,
    
    // 2024 Bleeding-edge capabilities
    MessageRedaction,      // April 2024 - Message deletion/redaction
    AccountExtban,         // July 2024 - Account-based bans
    Metadata2,             // September 2024 - User metadata v2
    
    // Draft capabilities (Work in Progress)
    MessageTagsUnlimited,
    Multiline,             // Multi-line messages with batching
    NoImplicitNames,
    PreAway,               // Away status during registration
    ReadMarker,            // Read receipt tracking
    RelayMsg,              // Bot message relaying
    ReplyDrafts,
    TypingClient,          // Typing indicators
    WebSocket,             // WebSocket transport
    ChannelRename,         // Channel renaming
    Persistence,           // Message persistence features
    ServerNameIndication,  // SNI support
    
    // Client-only tags
    ClientTyping,          // +typing client tag
    ClientReply,           // +draft/reply client tag
    ClientReact,           // +draft/react client tag
    
    // Custom/Vendor specific
    Custom(String),
}

impl Capability {
    pub fn from_str(s: &str) -> Self {
        match s {
            // Core IRCv3 capabilities (Ratified)
            "message-tags" => Capability::MessageTags,
            "server-time" => Capability::ServerTime,
            "account-notify" => Capability::AccountNotify,
            "account-tag" => Capability::AccountTag,
            "away-notify" => Capability::AwayNotify,
            "batch" => Capability::Batch,
            "cap-notify" => Capability::CapNotify,
            "chghost" => Capability::ChgHost,
            "echo-message" => Capability::EchoMessage,
            "extended-join" => Capability::ExtendedJoin,
            "invite-notify" => Capability::InviteNotify,
            "labeled-response" => Capability::LabeledResponse,
            "monitor" => Capability::Monitor,
            "multi-prefix" => Capability::MultiPrefix,
            "sasl" => Capability::Sasl,
            "setname" => Capability::Setname,
            "standard-replies" => Capability::StandardReplies,
            "userhost-in-names" => Capability::UserhostInNames,
            "bot" => Capability::BotMode,
            "utf8only" => Capability::UTF8Only,
            "sts" => Capability::StrictTransportSecurity,
            "webirc" => Capability::WebIRC,
            "chathistory" => Capability::Chathistory,
            
            // 2024 Bleeding-edge capabilities
            "draft/message-redaction" => Capability::MessageRedaction,
            "account-extban" => Capability::AccountExtban,
            "draft/metadata-2" => Capability::Metadata2,
            
            // Draft capabilities (Work in Progress)
            "draft/message-tags-unlimited" => Capability::MessageTagsUnlimited,
            "draft/multiline" => Capability::Multiline,
            "draft/no-implicit-names" => Capability::NoImplicitNames,
            "draft/pre-away" => Capability::PreAway,
            "draft/read-marker" => Capability::ReadMarker,
            "draft/relaymsg" => Capability::RelayMsg,
            "draft/reply" => Capability::ReplyDrafts,
            "draft/typing" => Capability::TypingClient,
            "draft/websocket" => Capability::WebSocket,
            "draft/channel-rename" => Capability::ChannelRename,
            "draft/persistence" => Capability::Persistence,
            "draft/sni" => Capability::ServerNameIndication,
            
            // Client-only tags (handled by client-tags capability)
            "+typing" => Capability::ClientTyping,
            "+draft/reply" => Capability::ClientReply,
            "+draft/react" => Capability::ClientReact,
            
            other => Capability::Custom(other.to_string()),
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            // Core IRCv3 capabilities (Ratified)
            Capability::MessageTags => "message-tags",
            Capability::ServerTime => "server-time",
            Capability::AccountNotify => "account-notify",
            Capability::AccountTag => "account-tag",
            Capability::AwayNotify => "away-notify",
            Capability::Batch => "batch",
            Capability::CapNotify => "cap-notify",
            Capability::ChgHost => "chghost",
            Capability::EchoMessage => "echo-message",
            Capability::ExtendedJoin => "extended-join",
            Capability::InviteNotify => "invite-notify",
            Capability::LabeledResponse => "labeled-response",
            Capability::Monitor => "monitor",
            Capability::MultiPrefix => "multi-prefix",
            Capability::Sasl => "sasl",
            Capability::Setname => "setname",
            Capability::StandardReplies => "standard-replies",
            Capability::UserhostInNames => "userhost-in-names",
            Capability::BotMode => "bot",
            Capability::UTF8Only => "utf8only",
            Capability::StrictTransportSecurity => "sts",
            Capability::WebIRC => "webirc",
            Capability::Chathistory => "chathistory",
            
            // 2024 Bleeding-edge capabilities
            Capability::MessageRedaction => "draft/message-redaction",
            Capability::AccountExtban => "account-extban",
            Capability::Metadata2 => "draft/metadata-2",
            
            // Draft capabilities (Work in Progress)
            Capability::MessageTagsUnlimited => "draft/message-tags-unlimited",
            Capability::Multiline => "draft/multiline",
            Capability::NoImplicitNames => "draft/no-implicit-names",
            Capability::PreAway => "draft/pre-away",
            Capability::ReadMarker => "draft/read-marker",
            Capability::RelayMsg => "draft/relaymsg",
            Capability::ReplyDrafts => "draft/reply",
            Capability::TypingClient => "draft/typing",
            Capability::WebSocket => "draft/websocket",
            Capability::ChannelRename => "draft/channel-rename",
            Capability::Persistence => "draft/persistence",
            Capability::ServerNameIndication => "draft/sni",
            
            // Client-only tags
            Capability::ClientTyping => "+typing",
            Capability::ClientReply => "+draft/reply",
            Capability::ClientReact => "+draft/react",
            
            Capability::Custom(s) => s,
        }
    }
}

pub struct CapabilitySet {
    capabilities: HashSet<Capability>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        let mut capabilities = HashSet::new();
        
        // Core IRCv3 capabilities (Ratified)
        capabilities.insert(Capability::MessageTags);
        capabilities.insert(Capability::ServerTime);
        capabilities.insert(Capability::AccountNotify);
        capabilities.insert(Capability::AccountTag);
        capabilities.insert(Capability::AwayNotify);
        capabilities.insert(Capability::Batch);
        capabilities.insert(Capability::CapNotify);
        capabilities.insert(Capability::ChgHost);
        capabilities.insert(Capability::EchoMessage);
        capabilities.insert(Capability::ExtendedJoin);
        capabilities.insert(Capability::InviteNotify);
        capabilities.insert(Capability::LabeledResponse);
        capabilities.insert(Capability::Monitor);
        capabilities.insert(Capability::MultiPrefix);
        capabilities.insert(Capability::Sasl);
        capabilities.insert(Capability::Setname);
        capabilities.insert(Capability::StandardReplies);
        capabilities.insert(Capability::UserhostInNames);
        capabilities.insert(Capability::BotMode);
        capabilities.insert(Capability::UTF8Only);
        capabilities.insert(Capability::StrictTransportSecurity);
        capabilities.insert(Capability::Chathistory);
        
        // 2024 Bleeding-edge capabilities
        capabilities.insert(Capability::MessageRedaction);
        capabilities.insert(Capability::AccountExtban);
        capabilities.insert(Capability::Metadata2);
        
        // Draft capabilities (Work in Progress) - Enable bleeding-edge features
        capabilities.insert(Capability::Multiline);
        capabilities.insert(Capability::ReadMarker);
        capabilities.insert(Capability::RelayMsg);
        capabilities.insert(Capability::TypingClient);
        capabilities.insert(Capability::PreAway);
        
        // Client-only tags support
        capabilities.insert(Capability::ClientTyping);
        capabilities.insert(Capability::ClientReply);
        capabilities.insert(Capability::ClientReact);
        
        Self { capabilities }
    }
    
    /// Create a capability set with only stable/ratified capabilities
    pub fn stable_only() -> Self {
        let mut capabilities = HashSet::new();
        
        // Only include ratified capabilities
        capabilities.insert(Capability::MessageTags);
        capabilities.insert(Capability::ServerTime);
        capabilities.insert(Capability::AccountNotify);
        capabilities.insert(Capability::AccountTag);
        capabilities.insert(Capability::AwayNotify);
        capabilities.insert(Capability::Batch);
        capabilities.insert(Capability::CapNotify);
        capabilities.insert(Capability::ChgHost);
        capabilities.insert(Capability::EchoMessage);
        capabilities.insert(Capability::ExtendedJoin);
        capabilities.insert(Capability::InviteNotify);
        capabilities.insert(Capability::LabeledResponse);
        capabilities.insert(Capability::Monitor);
        capabilities.insert(Capability::MultiPrefix);
        capabilities.insert(Capability::Sasl);
        capabilities.insert(Capability::Setname);
        capabilities.insert(Capability::StandardReplies);
        capabilities.insert(Capability::UserhostInNames);
        capabilities.insert(Capability::BotMode);
        capabilities.insert(Capability::UTF8Only);
        capabilities.insert(Capability::StrictTransportSecurity);
        capabilities.insert(Capability::Chathistory);
        
        Self { capabilities }
    }
    
    /// Create a bleeding-edge capability set with all 2024-2025 features
    pub fn bleeding_edge() -> Self {
        let mut set = Self::new();
        
        // Add experimental capabilities
        set.add(Capability::MessageTagsUnlimited);
        set.add(Capability::NoImplicitNames);
        set.add(Capability::ReplyDrafts);
        set.add(Capability::WebSocket);
        set.add(Capability::ChannelRename);
        set.add(Capability::Persistence);
        set.add(Capability::ServerNameIndication);
        
        set
    }
    
    pub fn supports(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
    
    pub fn add(&mut self, cap: Capability) {
        self.capabilities.insert(cap);
    }
    
    pub fn remove(&mut self, cap: &Capability) -> bool {
        self.capabilities.remove(cap)
    }
    
    pub fn to_string_list(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|cap| cap.as_str().to_string())
            .collect()
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}