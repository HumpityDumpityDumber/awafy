use crate::models::Lounge;
use anyhow::{Error, Result};
use reqwest::Client;

pub async fn create_lounge(client: &Client) -> Result<Lounge, Error> {
    let lounge_body = r#"{"name": "test", "description": "", "topicText": "", "allowGifting": false, "allowLiveAudio": false, "disabledAutoFillTrack": false, "coOwnerUsers": [], "thumbType": 2, "backgroundType": 3}"#;

    let lounge: Lounge = client
        .post("https://api.awa.io/v6/room")
        .body(lounge_body)
        .send()
        .await?
        .json()
        .await?;

    Ok(lounge)
}

// pub async fn fetch_queue(client: &Client, lounge: &mut Lounge) -> Result<(), Error> {
//     let queue: Queue = client
//         .get(format!("https://api.awa.io/v6/room/{}/queue", lounge.id))
//         .send()
//         .await?
//         .json()
//         .await?;

//     lounge.update_queue(queue);
//     Ok(())
// }
