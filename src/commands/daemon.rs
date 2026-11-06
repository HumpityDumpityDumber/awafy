use crate::models::Lounge;
use crate::player::Player;
use crate::socket::LoungeSocket;
use crate::{https::ApiClient, player::PlayerEvent};
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

    // Create player channels
    let (player, player_handle, mut playerevent_rx) = Player::new(api.clone()).await?;

    // Run player in the background
    tokio::spawn(player.run());

    if let Some(song) = lounge.get_queue_song(0) {
        player_handle.start(song.song.track_id).await?;
    }

    loop {
        tokio::select! {
            Some(event) = playerevent_rx.recv() => {
                match event {
                    PlayerEvent::SongFinished => {
                        if let Some(song) = lounge.get_queue_song(1) {
                            socket.pause(&song, &api.session.lock().await.clone().device_id).await?;
                        }
                    }
                }
            }
            Some((event, payload)) = socket.recv() => {
                match event.as_str() {
                    "receive:user:joined" => {
                        if let Payload::Text(values) = payload {
                            let info = values.first().unwrap().clone();
                            let parsed: Value = serde_json::from_str(info.as_str().unwrap()).unwrap();
                            info!("User joined: {}", serde_json::to_string_pretty(&parsed).unwrap());
                        }
                    }
                    "receive:message" => {
                        if  let Payload::Text(values) = payload {
                            let info =  values.first().unwrap().clone();
                            let parsed: Value = serde_json::from_str(info.as_str().unwrap()).unwrap();

                            if parsed["message"] == "PLAY" {
                                player_handle.play().await?;
                            }
                            if parsed["message"] == "PAUSE" {
                                player_handle.pause().await?;
                            }
                            // To skip forward and back we pause the
                            // song before or after the current one
                            if parsed["message"] == "SKIP_NEXT" {
                                if let Some(song) = lounge.get_queue_song(1) {
                                    socket.pause(&song, &api.session.lock().await.clone().device_id).await?;
                                }
                            }
                            if parsed["message"] == "SKIP_BACK" {
                                if let Some(song) = lounge.get_queue_song(-1) {
                                    socket.pause(&song, &api.session.lock().await.clone().device_id).await?;
                                }
                            }
                            if parsed["message"] == "FINISH" {
                                break
                            }
                        }
                    }
                    "message" => {
                        if let Payload::Text(values) = payload {
                            if let Some(msg) = values.first() {
                                match msg.as_str() {
                                    Some("receive:queue:event:update") => {
                                        let previous_pos = lounge.pos_index;

                                        let events = api.fetch_queue_events(&lounge).await?;
                                        lounge.update_queue_events(events);

                                        if lounge.pos_index != previous_pos {
                                            if let Some(song) = lounge.get_queue_song(0) {
                                                player_handle.start(song.song.track_id).await?;
                                            }
                                        }

                                        if lounge.is_playing == true {
                                            socket.pause(&lounge.get_queue_song(0).unwrap(), &api.session.lock().await.clone().device_id).await?;
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
