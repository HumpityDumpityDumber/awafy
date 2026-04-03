use crate::auth;
use crate::https::ApiClient;
use anyhow::{Error, Result};
use uuid::Uuid;

pub async fn login() -> Result<(), Error> {
    let device_id = Uuid::new_v4().to_string();
    let api = ApiClient::new(&device_id)?;

    let code = api.get_code().await?;
    println!("{}", code.code);

    let login_data = api.poll_login(&code).await?;
    auth::register_credentials(&login_data, &device_id)?;
    println!("logged in as {}!", login_data["name"].as_str().unwrap());

    Ok(())
}
