use crate::{auth, https::ApiClient};
use anyhow::{Context, Error, Result};
use futures_util::FutureExt;
use rust_socketio::{
    Event, Payload, TransportType,
    asynchronous::{Client, ClientBuilder},
};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use urlencoding::encode;

async fn join_room(socket: &Client, room_id: &str) {
    let payload = json!({ "roomId": room_id });
    if let Err(e) = socket.emit("send:user:joined", payload).await {
        eprintln!("Failed to join room: {e}");
    }
}

async fn queue_update_handler() {
    todo!();
}

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;

    let client = ApiClient::new(&session.device_id).context("Failed to build API client")?;
    client.refresh_session(&mut session).await?;

    let mut lounge = client.create_lounge(&session).await?;
    let queue = client.fetch_queue(&lounge, &session).await?;
    lounge.update_queue(queue);

    dbg!(&lounge.name);
    dbg!(&lounge.id);

    let socket_url = format!(
        "https://socket-gateway.awa.fm?token={}&transport=websocket",
        encode(&session.access_token)
    );

    let room_id = lounge.id.clone();

    let (tx, mut rx) = mpsc::channel::<(String, Payload)>(32);

    let socket = ClientBuilder::new(socket_url)
        .on("open", move |_, socket| {
            let room_id = room_id.clone();
            async move {
                join_room(&socket, &room_id).await;
            }
            .boxed()
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
                            println!("User joined: {}", serde_json::to_string_pretty(&parsed).unwrap());
                        }
                    }
                    "message" => {
                        if let Payload::Text(values) = payload {
                            if let Some(msg) = values.first() {
                                match msg.as_str() {
                                    Some("receive:queue:event:update") => {
                                        let queue = client.fetch_queue(&lounge, &session).await?;
                                        lounge.update_queue(queue);

                                        dbg!(&lounge);
                                    }
                                    other => {
                                        dbg!("Unrecognized message string:", other);
                                    }
                                }
                            }
                        }
                    }
                    unrecognized => {
                        dbg!("Unrecognized event:", unrecognized, &payload);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Shutting down daemon...");
                socket.disconnect().await?;
                client.finish_lounge(&lounge, &session).await?;
                break;
            }
        }
    }

    Ok(())
}
