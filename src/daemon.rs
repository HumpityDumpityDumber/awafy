use crate::constants::JSON_TYPE;
use anyhow::Result;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Url};
use std::sync::Arc;
use tokio::sync::RwLock;

struct ApiClient {
    client: Client,
}

impl ApiClient {
    pub fn new(device_id: &str) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON_TYPE));
        headers.insert(ACCEPT, HeaderValue::from_static(JSON_TYPE));
        headers.insert("X-Device-Id-Type", HeaderValue::from_static("4"));

        headers.insert("X-Device-Id", HeaderValue::from_str(device_id).unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build reqwest client");

        Self { client: client }
    }
    pub fn create_lounge() {
        todo!()
    }
}

struct Song {
    name: String,
    id: String,
    album: String,
    album_art: Url,
}

struct Lounge {
    name: String,
    id: String,
    state: String,
    queue: Arc<RwLock<Vec<Song>>>,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    todo!()
}
