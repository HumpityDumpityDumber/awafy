use crate::models::{Session, UserInfo};
use anyhow::{Context, Error, Result};
use directories::ProjectDirs;
use keyring::Entry;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const SERVICE: &str = "awafy";

pub fn get_user_info() -> Result<UserInfo, Error> {
    let proj_dirs = ProjectDirs::from("ink", "raurutuchr", "AWAfy")
        .context("Failed to determine project directories")?;
    let mut config = PathBuf::from(proj_dirs.config_dir());
    config.push("user.json");

    let user_info: UserInfo = serde_json::from_str(&fs::read_to_string(config)?)?;
    Ok(user_info)
}

pub fn new_session() -> Result<Session, Error> {
    let user_info = get_user_info()?;
    let entry = Entry::new(SERVICE, &user_info.id)
        .context("Failed to open keyring entry for current user")?;

    let session: Session = serde_json::from_str(&entry.get_password()?)?;
    Ok(session)
}

pub fn register_credentials(login_data: &Value, device_id: &str) -> Result<()> {
    let credentials = Session::from_login_data(login_data, device_id);

    let user_id = login_data["id"]
        .as_str()
        .context("Missing user id in login payload")?;
    let user_name = login_data["name"]
        .as_str()
        .context("Missing user name in login payload")?;

    let refresh_entry = Entry::new(SERVICE, user_id)?;
    let refresh_json = serde_json::to_string(&credentials)?;
    refresh_entry.set_password(&refresh_json)?;

    let proj_dirs =
        ProjectDirs::from("ink", "raurutuchr", "AWAfy").context("Failed to find dirs")?;
    fs::create_dir_all(proj_dirs.config_dir()).context("Failed to create config dir")?;

    let mut conf_path = PathBuf::from(proj_dirs.config_dir());
    conf_path.push("user.json");

    let user_info = UserInfo {
        id: user_id.to_owned(),
        name: user_name.to_owned(),
    };

    fs::write(conf_path, serde_json::to_string(&user_info)?)
        .context("Failed to write user file")?;

    Ok(())
}
