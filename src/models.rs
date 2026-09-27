use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;

// #[derive(Serialize, Deserialize, Debug)]
// enum QueueActions {
//     Unknown: 0,
//     Reset: 1,
//     Add: 2,
//     Remove: 3,
//     Play: 4,
//     Pause: 5,
//     Move: 7,
// }

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

#[derive(Serialize, Deserialize, Debug)]
pub struct Song {
    pub name: String,
    pub id: String,
    pub album: String,
    pub album_art: String,
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
    pub queue: Option<Mutex<Vec<Song>>>,
    #[serde(skip)]
    pub last_id: String,
}

impl Lounge {
    pub fn update_queue(&mut self, queue: Queue) -> () {
        self.queue = Some(Mutex::new(queue.tracks));
        self.cursor = queue.cursor;
        self.last_id = queue.last_id;
        self.is_playing = queue.is_playing;
    }
    pub fn update_queue_events(&mut self, events: QueueEvents) -> () {
        // for a in events.events {
        //     match a {
        //         QueueActions::Play => {}
        //         QueueActions::Pause => {}
        //         _ => {}
        //     }
        // }
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Queue {
    pub tracks: Vec<Song>,
    pub cursor: String,
    pub last_id: String,
    pub is_playing: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueueEvents {
    events: Vec<String>,
    cursor: String,
    last_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueueUpdate {
    queue: Queue,
    state: bool,
}

pub struct PlayerState {
    playing: bool,
    position: Duration,
    song: Song,
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
