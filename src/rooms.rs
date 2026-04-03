use crate::models::{Lounge, Queue, Song};
use anyhow::{Error, Result};
use random_word::Lang;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

pub async fn create_lounge(client: &Client) -> Result<Lounge, Error> {
    let name = format!(
        "{} {}",
        random_word::get(Lang::En),
        random_word::get(Lang::Ja)
    );
    let lounge_body = json!({"name": name, "description": "", "topicText": "", "allowGifting": false, "allowLiveAudio": false, "disabledAutoFillTrack": false, "coOwnerUsers": [], "thumbType": 2, "backgroundType": 3}).to_string();

    let lounge: Lounge = client
        .post("https://api.awa.io/v6/room")
        .body(lounge_body)
        .send()
        .await?
        .json()
        .await?;

    Ok(lounge)
}

// FIXME
pub async fn fetch_queue(client: &Client, lounge: &mut Lounge) -> Result<Queue, Error> {
    let response: Value = client
        .get(format!("https://api.awa.io/v6/room/{}/queue", lounge.id))
        .send()
        .await?
        .json()
        .await?;

    println!("{:?}", response);

    if response["mediaQueue"]["mediaPlaylist"]
        .get("mediaTracks")
        .is_some()
    {
        let queue: Queue = Queue(
            response["mediaQueue"]["mediaPlaylist"]["mediaTracks"]
                .as_array()
                .iter()
                .map(|song| Song {
                    name: "placeholder name".to_string(),
                    id: song["trackId"].as_str().to_owned().to_string(),
                    album: "placeholder album".to_string(),
                    album_art: "placeholder album art".to_string(),
                })
                .collect(),
        );
        Ok(queue)
    } else {
        Ok(Queue(Vec::new()))
    }
}
