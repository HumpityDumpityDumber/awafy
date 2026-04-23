use crate::https::ApiClient;
use crate::models::Session;
use anyhow::{Error, Result};

pub async fn login() -> Result<(), Error> {
    // TODO
    let device_id = "token generated from machine id";
    let api = ApiClient::new(&device_id)?;

    let code = api.get_code().await?;
    println!("{}", code.code);

    let login_data = api.poll_login(&code).await?;
    let credentials = Session::from_login_data(&login_data, &device_id);

    println!(
        "logged in as user {}!",
        login_data["name"].as_str().unwrap()
    );
    println!(
        "This is your refresh token: {}
Set the environment variable AWAFY_TOKEN to this value before running awafy in daemon mode.",
        credentials.refresh_token
    );

    Ok(())
}
