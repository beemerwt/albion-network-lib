use crate::albion::ChatChannel;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ChatState {
    channels_by_id: HashMap<i64, ChatChannel>,
}

impl ChatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn join_channel(&mut self, channel_id: i64, chat_index: i64) {
        self.channels_by_id
            .insert(channel_id, ChatChannel::from_chat_index(chat_index));
    }

    pub fn leave_channel(&mut self, channel_id: i64) {
        self.channels_by_id.remove(&channel_id);
    }

    pub fn channel_type(&self, channel_id: i64) -> Option<ChatChannel> {
        self.channels_by_id.get(&channel_id).copied()
    }

    pub fn clear(&mut self) {
        self.channels_by_id.clear();
    }

    pub fn len(&self) -> usize {
        self.channels_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.channels_by_id.is_empty()
    }
}