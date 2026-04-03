use crate::{auth, https::ApiClient};
use anyhow::{Context, Error, Result};

pub async fn main() -> Result<(), Error> {
    let mut session = auth::new_session().context("Failed to get credentials")?;

    let client = ApiClient::new(&session.device_id).context("Failed to build API client")?;
    let login_data = client.refresh_session(&mut session).await?;
    auth::register_credentials(&login_data, &session.device_id)?;

    let mut lounge = client.create_lounge(&session).await?;
    let queue = client.fetch_queue(&lounge, &session).await?;
    lounge.update_queue(queue);

    println!("{:?}", lounge);

    Ok(())
}
