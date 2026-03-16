use crate::models::{Code, LoginData};
use anyhow::Result;
use keyring::Entry;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

const JSON_TYPE: &str = "application/json";

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

#[derive(Serialize, Deserialize, Debug)]
struct RefreshCred {
    refresh_token: String,
    username: String,
    user_id: String,
    device_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AccessCred {
    access_token: String,
    token_expiry: i64,
}

fn register_credentials(credentials: &LoginData, device_id: String) -> Result<()> {
    const SERVICE: &str = "AWAfy";

    let refresh_cred = RefreshCred {
        refresh_token: credentials.auth_data.refresh.refresh_token.clone(),
        username: credentials.name.clone(),
        user_id: credentials.id.clone(),
        device_id,
    };

    let refresh_entry = Entry::new(SERVICE, &format!("{}:refresh_token", credentials.name))?;
    let refresh_json = serde_json::to_string(&refresh_cred)?;
    refresh_entry.set_password(&refresh_json)?;

    let access_cred = AccessCred {
        access_token: credentials.auth_data.refresh.access_token.clone(),
        token_expiry: credentials.auth_data.refresh.expires_at.clone(),
    };

    let access_entry = Entry::new(SERVICE, &format!("{}:access_token", credentials.name))?;
    let access_json = serde_json::to_string(&access_cred)?;
    access_entry.set_password(&access_json)?;
    Ok(())
}

#[tokio::main]
pub async fn auth() -> Result<(), reqwest::Error> {
    let device_id = Uuid::new_v4().to_string();
    let client = ApiClient::new(&device_id);

    let code = client.get_code().await?;
    println!("{}", code.code);

    let login_data = client.poll_login(&code).await?;
    register_credentials(&login_data, device_id).unwrap();
    println!("logged in as {}!", login_data.name);
    Ok(())
}
