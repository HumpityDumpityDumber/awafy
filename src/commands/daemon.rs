use crate::models::Lounge;
use crate::socket::LoungeSocket;
use crate::{auth, https::ApiClient};
use anyhow::{Context, Error, Result};
use rust_socketio::Payload;
use serde_json::Value;
use tracing::{debug, info};

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;

    let client = ApiClient::new(&session.device_id).context("Failed to build API client")?;
    client.refresh_session(&mut session).await?;

    let mut lounge: Lounge = client.create_lounge(&session).await?;
    client.fetch_queue(&lounge.id, &session).await?;

    info!("Lounge created: {}", serde_json::to_string_pretty(&lounge)?);

    let room_id = lounge.id.clone();

    let mut socket = LoungeSocket::connect(&session, room_id).await?;

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
                                        let events = client.fetch_queue_events(&lounge, &session, None).await?;
                                        lounge.update_queue_events(events);
                                        debug!("{}", serde_json::to_string_pretty(&lounge)?);
                                        if lounge.is_playing == true {
                                            todo!();
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
                client.finish_lounge(&lounge, &session).await?;
                break;
            }
        }
    }
    info!("Shutting down daemon...");
    socket.disconnect().await?;

    Ok(())
}
