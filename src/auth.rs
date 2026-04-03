use crate::constants::SERVICE;
use crate::models::Code;
use crate::models::{Session, UserInfo};
use anyhow::{Context, Error, Result};
use directories::ProjectDirs;
use keyring::Entry;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tokio::time::{Duration, sleep};

pub async fn get_code(client: &Client) -> Result<Code, Error> {
    let code: Code = client
        .post("https://api.awa.io/v4/code")
        .send()
        .await?
        .json()
        .await?;

    Ok(code)
}

pub async fn poll_login(code: &Code, client: &Client) -> Result<Value, Error> {
    loop {
        let auth_response = client
            .post("https://api.awa.io/v5/login/code")
            .json(code)
            .send()
            .await?;

        let status = auth_response.status();
        let text = auth_response.text().await?;

        match status {
            StatusCode::OK => {
                let login_data: Value = serde_json::from_str(&text)?;
                return Ok(login_data);
            }
            StatusCode::UNAUTHORIZED => {
                println!("Code not authorized! Trying again in 3 seconds...");
                sleep(Duration::from_secs(3)).await;
            }
            _ => {
                panic!("Unexpected status code {} with body {}", status, text)
            }
        }
    }
}

pub fn get_user_info() -> Result<UserInfo, Error> {
    let proj_dirs = ProjectDirs::from("ink", "raurutuchr", "AWAfy").unwrap();
    let mut config = PathBuf::from(proj_dirs.config_dir());
    config.push("user.json");

    let user_info: UserInfo = serde_json::from_str(&fs::read_to_string(config)?)?;
    return Ok(user_info);
}

pub fn new_session() -> Result<Session, Error> {
    let user_info = get_user_info()?;
    let entry = Entry::new(SERVICE, &user_info.id).unwrap();

    let session: Session = serde_json::from_str(&entry.get_password()?)?;
    return Ok(session);
}

pub fn register_credentials(login_data: &Value, device_id: &str) -> Result<()> {
    let credentials = Session::from_login_data(login_data, &device_id);
    let refresh_entry = Entry::new(SERVICE, login_data["id"].as_str().unwrap())?;
    let refresh_json = serde_json::to_string(&credentials)?;
    refresh_entry.set_password(&refresh_json)?;

    let proj_dirs =
        ProjectDirs::from("ink", "raurutuchr", "AWAfy").context("Failed to find dirs")?;
    fs::create_dir_all(proj_dirs.config_dir()).context("Failed to create config dir")?;
    let mut conf_path = PathBuf::from(proj_dirs.config_dir());
    conf_path.push("user.json");

    let user_info = UserInfo {
        id: login_data["id"].as_str().unwrap().to_owned(),
        name: login_data["name"].as_str().unwrap().to_owned(),
    };

    fs::write(conf_path, serde_json::to_string(&user_info)?)
        .context("Failed to write user file")?;

    Ok(())
}

pub async fn refresh_session(session: &mut Session, client: &Client) -> Result<(), Error> {
    let login_data: Value = client
        .post("https://api.awa.io/v5/authorize")
        .body(json!({"refreshToken": session.refresh_token}).to_string())
        .send()
        .await?
        .json()
        .await?;

    register_credentials(&login_data, &session.device_id)?;

    let new_session = Session::from_login_data(&login_data, &session.device_id);
    *session = new_session;
    Ok(())
}
