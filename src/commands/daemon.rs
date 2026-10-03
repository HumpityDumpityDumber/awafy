use crate::models::Lounge;
use crate::socket::LoungeSocket;
use crate::{auth, https::ApiClient};
use anyhow::{Context, Result};
use rust_socketio::Payload;
use serde_json::Value;
use tracing::{debug, info};

pub async fn main() -> Result<()> {
    let mut session = auth::new_session().context("Failed to get credentials")?;

    let client = ApiClient::new(&session.device_id).context("Failed to build API client")?;
    client.refresh_session(&mut session).await?;

    let mut lounge: Lounge = if let Some(lounge) = client.existing_lounge(&session).await? {
        lounge
    } else {
        client.create_lounge(&session).await?
    };

    info!("Lounge created: {}", serde_json::to_string_pretty(&lounge)?);

    let mut socket = LoungeSocket::connect(&session, lounge.id.clone()).await?;
    let queue = client.fetch_queue(&lounge.id, &session).await?;
    lounge.update_queue(queue);

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
                                        let events = client.fetch_queue_events(&lounge, &session).await?;
                                        lounge.update_queue_events(events);
                                        if lounge.is_playing == true {
                                            socket.pause(&lounge.get_playing().await.unwrap(), &session.device_id).await?;
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
