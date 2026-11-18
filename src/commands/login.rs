use crate::auth;
use crate::https::ApiClient;
use crate::models::Session;
use anyhow::{Error, Result};
use tracing::{debug, info};

pub async fn login(abema: bool) -> Result<(), Error> {
    if abema {
        info!("Making requests to abema music api")
    };
    let device_id = auth::get_device_id()?;
    let device_name = auth::get_device_name(&device_id)?;
    debug!("device id: {}\n device name: {}", &device_id, &device_name);
    let api = ApiClient::new(abema)?;

    let code = api.get_code().await?;
    info!("CODE: {}", code.code);

    let login_data = api.poll_login(&code).await?;
    let credentials = Session::from_login_data(&login_data, &device_id, &device_name);

    info!(
        "logged in as user {}!",
        login_data["name"].as_str().unwrap()
    );
    info!(
        "This is your refresh token: {}
Set the environment variable AWAFY_TOKEN to this value before running awafy in daemon mode.",
        credentials.refresh_token
    );

    Ok(())
}
