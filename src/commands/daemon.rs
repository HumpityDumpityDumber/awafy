use crate::https::ApiClient;
use crate::models::Lounge;
use crate::player::Player;
use crate::socket::LoungeSocket;
use anyhow::{Context, Result};
use rust_socketio::Payload;
use serde_json::Value;
use tracing::{debug, info};

pub async fn main() -> Result<()> {
    let mut api = ApiClient::new().context("Failed to build API client")?;
    api.refresh_session().await?;

    let mut lounge: Lounge = if let Some(lounge) = api.existing_lounge().await? {
        lounge
    } else {
        api.create_lounge().await?
    };

    info!("Lounge created: {}", serde_json::to_string_pretty(&lounge)?);

    let mut socket =
        LoungeSocket::connect(&api.session.lock().await.clone(), lounge.id.clone()).await?;
    let queue = api.fetch_queue(&lounge.id).await?;
    lounge.update_queue(queue);

    let mut player = Player::new(api.clone());

    // Start player task

    loop {
        tokio::select! {
            Some((event, payload)) = socket.recv() => {
                match event.as_str() {
                    "receive:user:joined" => {
                        if let Payload::Text(values) = payload {
                            let info = values.first().unwrap().clone();
                            let parsed: Value = serde_json::from_str(info.as_str().unwrap()).unwrap();
                            info!("User joined: {}", serde_json::to_string_pretty(&parsed).unwrap());
                        }
                    }
                    "message" => {
                        if let Payload::Text(values) = payload {
                            if let Some(msg) = values.first() {
                                match msg.as_str() {
                                    Some("receive:queue:event:update") => {
                                        let events = api.fetch_queue_events(&lounge).await?;
                                        lounge.update_queue_events(events);
                                        if lounge.is_playing == true {
                                            socket.pause(&lounge.get_playing().await.unwrap(), &api.session.lock().await.clone().device_id).await?;
                                        }
                                    }
                                    Some("receive:user:force_leave") => {
                                        break
                                    }
                                    unrecognized => {
                                        debug!("Unrecognized message string: {:?}", unrecognized);
                                    }
                                }
                            }
                        }
                    }
                    unrecognized => {
                        debug!("Unrecognized event: {} with payload: {:?}", unrecognized, &payload);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                api.finish_lounge(&lounge).await?;
                break;
            }
        }
    }
    info!("Shutting down daemon...");
    socket.disconnect().await?;

    Ok(())
}
