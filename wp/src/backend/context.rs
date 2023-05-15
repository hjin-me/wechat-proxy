use std::collections::HashMap;

pub type Chat = (i64, String, String);

#[derive(Debug, Clone)]
pub struct ChatContext {
    pub user_id: String,
    pub conversations: Vec<Chat>,
}

impl ChatContext {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            conversations: vec![],
        }
    }
    pub fn history(&self) -> Vec<Vec<String>> {
        let mut r = vec![];
        let before = time::OffsetDateTime::now_utc().unix_timestamp() - 60 * 30;

        // conversations for loop
        for c in &self.conversations {
            if c.0 < before {
                continue;
            }
            r.push(vec![c.1.clone(), c.2.clone()]);
        }

        r
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatMgr {
    pub chats: HashMap<String, ChatContext>,
}

impl ChatMgr {
    pub fn add(&mut self, user_id: &str, q: &str, a: &str, ts: i64) {
        let c = self
            .chats
            .entry(user_id.to_string())
            .or_insert(ChatContext {
                user_id: user_id.to_string(),
                conversations: vec![],
            });
        c.conversations.push((ts, q.to_string(), a.to_string()));
    }
    pub fn get(&self, user_id: &str) -> Option<&ChatContext> {
        self.chats.get(user_id)
    }
    pub fn clear(&mut self, user_id: &str) {
        let c = self
            .chats
            .entry(user_id.to_string())
            .or_insert(ChatContext {
                user_id: user_id.to_string(),
                conversations: vec![],
            });
        c.conversations.clear();
    }
}
