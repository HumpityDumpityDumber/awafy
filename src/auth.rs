use crate::constants::JSON_TYPE;
use crate::models::{Code, Credentials, LoginData, UserInfo};
use anyhow::Result;
use directories::ProjectDirs;
use keyring::Entry;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};
use std::{fs, path::PathBuf};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    pub fn new(device_id: &str) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON_TYPE));
        headers.insert(ACCEPT, HeaderValue::from_static(JSON_TYPE));
        headers.insert("X-Device-Id-Type", HeaderValue::from_static("4"));

        headers.insert("X-Device-Id", HeaderValue::from_str(device_id).unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build reqwest client");

        Self { client: client }
    }
    pub async fn get_code(&self) -> Result<Code, reqwest::Error> {
        let code_response = self
            .client
            .post("https://api.awa.io/v4/code")
            .send()
            .await?
            .text()
            .await?;

        let code: Code = serde_json::from_str(&code_response).unwrap();
        return Ok(code);
    }
    pub async fn poll_login(&self, code: &Code) -> Result<LoginData, reqwest::Error> {
        loop {
            let auth_response = self
                .client
                .post("https://api.awa.io/v5/login/code")
                .json(code)
                .send()
                .await?;

            let status = auth_response.status();
            let text = auth_response.text().await?;

            match status {
                StatusCode::OK => {
                    let logindata: LoginData = serde_json::from_str(&text).unwrap();
                    return Ok(logindata);
                }
                StatusCode::UNAUTHORIZED => {
                    println!("Code not authorized! Trying again in 3 seconds...");
                    sleep(Duration::from_secs(3)).await;
                }
                _ => {
                    panic!(
                        "Genuinely what went wrong. I got status code {} with body of {}???",
                        status, text
                    )
                }
            }
        }
    }
}

fn register_credentials(login_data: &LoginData, device_id: String) -> Result<()> {
    const SERVICE: &str = "AWAfy";

    let credentials = Credentials {
        access_token: login_data.auth_data.refresh.access_token.clone(),
        token_expiry: login_data.auth_data.refresh.expires_at.clone(),
        refresh_token: login_data.auth_data.refresh.refresh_token.clone(),
    };

    let refresh_entry = Entry::new(SERVICE, &login_data.id)?;
    let refresh_json = serde_json::to_string(&credentials)?;
    refresh_entry.set_password(&refresh_json)?;

    // TODO: also need to save user info file w/ id, name, pfp link
    if let Some(proj_dirs) = ProjectDirs::from("ink", "raurutuchr", "AWAfy") {
        fs::create_dir_all(proj_dirs.config_dir()).unwrap();
        let mut config = PathBuf::from(proj_dirs.config_dir());
        config.push("user.json");
        fs::write(config, "thing").unwrap();
    } else {
        panic!()
    }

    let user_info = UserInfo {
        id: login_data.id.clone(),
        name: login_data.name.clone(),
        device_id: device_id,
    };

    Ok(())
}

#[tokio::main]
pub async fn login() -> Result<()> {
    let device_id = Uuid::new_v4().to_string();
    let client = ApiClient::new(&device_id);

    let code = client.get_code().await?;
    println!("{}", code.code);

    let login_data = client.poll_login(&code).await?;
    register_credentials(&login_data, device_id).unwrap();
    println!("logged in as {}!", login_data.name);
    Ok(())
}
