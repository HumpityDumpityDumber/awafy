use std::{sync::Arc, vec};

use anyhow::Result;
use reqwest::Url;
use tokio::sync::RwLock;

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
