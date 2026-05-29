use std::fmt;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[repr(u8)]
pub enum ChatChannel {
    Say = 27,
    Guild = 24,
    Faction = 29,
}

impl ChatChannel {
    pub fn from_chat_index(value: i64) -> Self {
        match value {
            27 => ChatChannel::Say,
            24 => ChatChannel::Guild,
            29 => ChatChannel::Faction,
            _ => ChatChannel::Say,
        }
    }

    pub fn from_i64(value: i64) -> Self {
        match value {
            0 => ChatChannel::Say,
            3517 => ChatChannel::Guild,
            1868 => ChatChannel::Faction, // Thetford
            1856 => ChatChannel::Faction, // Martlock
            _ => ChatChannel::Say,        // Default to Say for unknown channels
        }
    }
}

impl fmt::Display for ChatChannel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ChatChannel::Say => write!(f, "Local"),
            ChatChannel::Faction => write!(f, "Faction"),
            ChatChannel::Guild => write!(f, "Guild"),
        }
    }
}
