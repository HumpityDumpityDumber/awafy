use crate::models::{Lounge, QueueSong, Session};
use anyhow::{Context, Result};
use futures_util::FutureExt;
use rust_socketio::{
    Event, Payload, TransportType,
    asynchronous::{Client, ClientBuilder},
};
use serde_json::json;
use tokio::sync::{mpsc, mpsc::Receiver};
use tracing::{error, info};
use urlencoding::encode;

pub struct LoungeSocket {
    client: Client,
    rx: Receiver<(String, Payload)>,
}

impl LoungeSocket {
    pub async fn connect(session: &Session, room_id: String) -> Result<Self> {
        let socket_url = format!(
            "https://socket-gateway.awa.fm?token={}",
            encode(&session.access_token)
        );

        let (tx, rx) = mpsc::channel::<(String, Payload)>(32);

        let client = ClientBuilder::new(socket_url)
            .on(Event::Connect, move |_, socket| {
                let room_id = room_id.clone();
                async move {
                    {
                        let socket: &Client = &socket;
                        let room_id: &str = &room_id;
                        async move {
                            let payload = json!({ "roomId": room_id });
                            if let Err(e) = socket.emit("send:user:joined", payload).await {
                                error!("Failed to join room: {e}");
                            }
                        }
                    }
                    .await;
                }
                .boxed()
            })
            .on(Event::Close, |_, _socket| {
                async move { info!("socket closed!") }.boxed()
            })
            .on("error", |err, _| {
                async move { error!("Error: {:?}", err) }.boxed()
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

        Ok(Self { client, rx })
    }
    pub async fn recv(&mut self) -> Option<(String, Payload)> {
        self.rx.recv().await
    }
    pub async fn disconnect(self) -> Result<()> {
        self.client
            .disconnect()
            .await
            .context("Failed to disconnect socket")?;
        Ok(())
    }
    pub async fn pause(&self, playing: &QueueSong, device_id: &str) -> Result<()> {
        let payload = json!({"events": [
            {
                "action": 8,
                "mediaTrackId": playing,
                "clientId": device_id,
                "seekPosition": 0,
                "createdBy": {
                    "id": device_id
                }
            }
        ]});

        self.client
            .emit("send:queue:event:register", payload)
            .await
            .context("Failed to send pause message")?;

        Ok(())
    }
}
