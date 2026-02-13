use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Join { username: String },
    Chat { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    UserJoined { username: String, time: Option<DateTime<Utc>> },
    Chat { username: String, message: String, time: Option<DateTime<Utc>> },
}

