//! Общий протокол обмена между клиентом и сервером «Искры».
//! Транспорт: TCP, по одному JSON-объекту на строку.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const APP_NAME: &str = "Искра";
pub const DEFAULT_PORT: u16 = 8787;

/// Профиль пользователя, как его видят остальные.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: String,
    /// Индекс цвета аватарки.
    pub color: u8,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    pub text: String,
    pub ts: i64,
    /// Прочитано ли сообщение получателем (актуально для своих сообщений).
    pub read: bool,
}

/// Короткая карточка диалога для списка чатов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub user: User,
    pub last_text: String,
    pub last_ts: i64,
    pub last_from: u64,
    pub unread: u32,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum ClientMsg {
    Register {
        username: String,
        name: String,
        password: String,
    },
    Login {
        username: String,
        password: String,
    },
    SendMessage {
        to: u64,
        text: String,
    },
    History {
        with: u64,
        before_id: Option<u64>,
        limit: u32,
    },
    Typing {
        to: u64,
    },
    MarkRead {
        with: u64,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum ServerMsg {
    AuthOk {
        me: User,
        chats: Vec<ChatSummary>,
    },
    Chats {
        chats: Vec<ChatSummary>,
    },
    Message {
        msg: Message,
    },
    History {
        with: u64,
        messages: Vec<Message>,
        has_more: bool,
    },
    Typing {
        from: u64,
    },
    Read {
        by: u64,
        up_to: i64,
    },
    Presence {
        user_id: u64,
        online: bool,
        last_seen: i64,
    },
    Error {
        message: String,
        /// true — повторять подключение бессмысленно (например, неверный пароль).
        fatal: bool,
    },
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Куда писать данные приложения.
pub fn data_dir() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("ISKRA_DATA") {
        if !p.trim().is_empty() {
            return std::path::PathBuf::from(p);
        }
    }
    let base = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("Iskra")
}
