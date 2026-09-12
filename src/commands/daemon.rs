use crate::{auth, https::ApiClient};
use anyhow::{Context, Error, Result};
use futures_util::FutureExt;
use rust_socketio::{
    Payload, TransportType,
    asynchronous::{Client, ClientBuilder},
};
use serde_json::json;
use urlencoding::encode;

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;

    let client = ApiClient::new(&session.device_id).context("Failed to build API client")?;
    client.refresh_session(&mut session).await?;

    let mut lounge = client.create_lounge(&session).await?;
    let queue = client.fetch_queue(&lounge, &session).await?;
    lounge.update_queue(queue);

    dbg!(lounge.name);
    dbg!(lounge.id.clone());

    let socket_url = format!(
        "https://socket-gateway.awa.fm?token={}&transport=websocket",
        encode(&session.access_token)
    );

    let room_connect = json!({"roomId": lounge.id.clone()}).to_string();

    let join_room = |p: Payload, s: Client| {
        async move {
            s.emit("send:user:joined", &room_connect)
                .await
                .expect("Failed to join room");
        }
        .boxed()
    };

    let socket = ClientBuilder::new(socket_url)
        .on("receive:user:joined", |_payload, _socket| {
            async move {
                println!("User joined!");
            }
            .boxed()
        })
        .on("receive:queue:event:update", |_payload, _socket| {
            async move {
                println!("Queue updated");
            }
            .boxed()
        })
        .on("open", join_room)
        .transport_type(TransportType::Websocket)
        .connect()
        .await
        .expect("Connection failed");

    tokio::signal::ctrl_c().await?;

    Ok(())
}
