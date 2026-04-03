use crate::{auth, https::build_client, rooms};
use anyhow::{Context, Error, Result};

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;
    let authed_client = build_client(&session.device_id, Some(&session))?;
    let client = build_client(&session.device_id, None)?;

    auth::refresh_session(&mut session, &client).await?;

    let mut lounge = rooms::create_lounge(&authed_client).await?;

    rooms::fetch_queue(&authed_client, &mut lounge).await?;

    println!("{:?}", lounge);

    Ok(())
}
