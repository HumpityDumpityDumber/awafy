use crate::auth;
use crate::https;
use anyhow::{Context, Error, Result};
use uuid::Uuid;

pub async fn login() -> Result<(), Error> {
    let device_id = Uuid::new_v4().to_string();
    let client = https::build_client(&device_id, None).context("Failed to build client")?;

    let code = auth::get_code(&client).await?;
    println!("{}", code.code);

    let login_data = auth::poll_login(&code, &client).await?;
    auth::register_credentials(&login_data, &device_id)?;
    println!("logged in as {}!", login_data["name"].as_str().unwrap());
    Ok(())
}
