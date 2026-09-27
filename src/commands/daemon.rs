use crate::models::Lounge;
use crate::{auth, https::ApiClient};
use anyhow::{Context, Error, Result};
use futures_util::FutureExt;
use rust_socketio::{
    Event, Payload, TransportType,
    asynchronous::{Client, ClientBuilder},
};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use urlencoding::encode;

async fn join_room(socket: &Client, room_id: &str) {
    let payload = json!({ "roomId": room_id });
    if let Err(e) = socket.emit("send:user:joined", payload).await {
        error!("Failed to join room: {e}");
    }
}

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;

    let client = ApiClient::new(&session.device_id).context("Failed to build API client")?;
    client.refresh_session(&mut session).await?;

    let mut lounge: Lounge = client.create_lounge(&session).await?;
    client.fetch_queue(&lounge.id, &session).await?;

    info!("Lounge created: {}", serde_json::to_string_pretty(&lounge)?);

    let socket_url = format!(
        "https://socket-gateway.awa.fm?token={}&transport=websocket",
        encode(&session.access_token)
    );

    let room_id = lounge.id.clone();

    let (tx, mut rx) = mpsc::channel::<(String, Payload)>(32);

    let socket = ClientBuilder::new(socket_url)
        .on(Event::Connect, move |_, socket| {
            let room_id = room_id.clone();
            async move {
                join_room(&socket, &room_id).await;
            }
            .boxed()
        })
        .on(Event::Close, |_, _socket| {
            async move { println!("socket closed!") }.boxed()
        })
        .on("error", |err, _| {
            async move { eprintln!("Error: {:#?}", err) }.boxed()
        })
        .on_any(move |e: Event, p: Payload, _| {
            let tx = tx.clone();
            async move {
                let _ = tx.send((e.to_string(), p)).await;
            }
            .boxed()
        })
        .transport_type(TransportType::Websocket)
        .connect()
        .await
        .expect("Connection failed");

    loop {
        tokio::select! {
                Some((event, payload)) = rx.recv() => {
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
                                        }
                                        Some("receive:user:force_leave") => {
                                            break
                                        }
                                        other => {
                                            debug!("Unrecognized message string: {:?}", other);
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
