use crate::{auth, https::build_client, rooms};
use anyhow::{Context, Error, Result};
// use std::sync::Arc;
// use tokio::sync::RwLock;

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;
    let client = build_client(&session.device_id, Some(&session))?;

    auth::refresh_session(&mut session, &client).await?;

    let lounge = rooms::create_lounge(&client).await?;

    // rooms::fetch_queue(&mut lounge).await?;

    println!("{:?}", lounge);

    Ok(())
}
