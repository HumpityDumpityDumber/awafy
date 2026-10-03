use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct Code {
    pub code: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Session {
    pub access_token: String,
    pub token_expiry: i64,
    pub refresh_token: String,
    pub device_id: String,
    pub device_name: String,
}

impl Session {
    pub fn from_login_data(login_data: &Value, device_id: &str, device_name: &str) -> Self {
        return Session {
            access_token: login_data["authData"]["refresh"]["accessToken"]
                .as_str()
                .expect("Missing access token")
                .to_owned(),
            token_expiry: login_data["authData"]["refresh"]["expiresAt"]
                .as_i64()
                .expect("Missing token expiry"),
            refresh_token: login_data["authData"]["refresh"]["refreshToken"]
                .as_str()
                .expect("Missing refresh token")
                .to_owned(),
            device_id: device_id.to_owned(),
            device_name: device_name.to_owned(),
        };
    }
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct UserInfo {
//     pub id: String,
//     pub name: String,
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueueSong {
    pub name: Option<String>,
    pub track_id: String,
    pub local_id: String,
    pub album: Option<String>,
    pub album_art: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum QueueAction {
    Unknown,
    Reset,
    Add(Vec<QueueSong>),
    Remove(String),
    Play(String),
    Pause(String),
    Move { id: String, dest_id: String },
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Lounge {
    pub name: String,
    pub id: String,
    #[serde(skip)]
    pub cursor: String,
    #[serde(skip)]
    pub is_playing: bool,
    #[serde(rename = "mediaQueueId")]
    pub queue_id: String,
    #[serde(skip)]
    queue: Mutex<Vec<QueueSong>>,
    #[serde(skip)]
    pub pos_index: Option<usize>,
    #[serde(skip)]
    pub last_id: String,
}

impl Lounge {
    pub async fn get_playing(&self) -> Option<QueueSong> {
        let queue = self.queue.lock().unwrap();

        Some(queue[self.pos_index?].clone())
    }
    pub fn update_queue(&mut self, queue: Queue) {
        self.queue = Mutex::new(queue.tracks);
        self.cursor = queue.cursor;
        self.last_id = queue.last_id;
        self.is_playing = queue.is_playing;
        self.pos_index = queue.pos_index;

        info!("queue updated: {:#?}", self);
    }
    pub fn update_queue_events(&mut self, events: QueueEvents) {
        self.cursor = events.cursor;
        self.last_id = events.last_id;

        for a in events.events {
            // 0 - Unkown
            // 1 - Reset
            // 2 - Add
            // 3 - Remove
            // 4 - Play
            // 5 - Pause
            // 7 - Move
            match a {
                QueueAction::Unknown => (),
                QueueAction::Reset => self.queue.lock().unwrap().clear(),
                QueueAction::Add(mut tracks) => {
                    let queue = &mut self.queue.lock().unwrap();
                    queue.append(&mut tracks);
                }
                QueueAction::Remove(track_id) => {
                    let queue = &mut self.queue.lock().unwrap();
                    let pos = queue.iter().position(|t| track_id == t.local_id).unwrap();

                    queue.remove(pos);
                }
                QueueAction::Play(track) => {
                    let queue = &mut self.queue.lock().unwrap();

                    self.is_playing = true;
                    self.pos_index = Some(queue.iter().position(|t| track == t.local_id).unwrap());
                }
                QueueAction::Pause(track) => {
                    let queue = &mut self.queue.lock().unwrap();

                    self.is_playing = false;
                    self.pos_index = Some(queue.iter().position(|t| track == t.local_id).unwrap());
                }
                QueueAction::Move { id, dest_id } => {
                    let queue = &mut self.queue.lock().unwrap();
                    let from = queue.iter().position(|t| id == t.local_id).unwrap();
                    let to = queue.iter().position(|t| dest_id == t.local_id).unwrap();

                    if from > to {
                        queue[to..=from].rotate_right(1);
                    } else {
                        queue[from..=to].rotate_left(1);
                    }
                }
            }
        }
        info!("queue updated: {:#?}", self);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Queue {
    pub tracks: Vec<QueueSong>,
    pub cursor: String,
    pub last_id: String,
    pub is_playing: bool,
    pub pos_index: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueueEvents {
    pub events: Vec<QueueAction>,
    pub cursor: String,
    pub last_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueueEvent {
    action: QueueAction,
    target_contents: Value,
}

pub struct PlayerState {
    playing: bool,
    position: Duration,
    song: QueueSong,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StreamingToken {
    #[serde(rename = "accessToken")]
    token: String,
    #[serde(rename = "expiredAt")]
    expiry: i64,
    #[serde(rename = "expireTime")]
    expire_time: u32,
}
