use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;

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
}

impl Session {
    pub fn from_login_data(login_data: &Value, device_id: &str) -> Self {
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

#[derive(Deserialize, Debug)]
pub struct Lounge {
    pub name: String,
    pub id: String,
    #[serde(skip)]
    pub state: Option<String>,
    #[serde(rename = "mediaQueueId")]
    pub queue_id: String,
    #[serde(skip)]
    queue: Option<Mutex<Queue>>,
}

impl Lounge {
    pub fn update_queue(&mut self, queue: Queue) -> () {
        self.queue = Some(Mutex::new(queue));
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Queue(pub Vec<Song>);

pub struct PlayerState {
    playing: bool,
    position: Duration,
    song: Song,
}
