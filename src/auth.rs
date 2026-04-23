use crate::models::Session;
use anyhow::{Error, Result};
use std::{env, fs, io};

pub fn get_device_id() -> Result<String, io::Error> {
    return fs::read_to_string("/etc/machine-id")
        .or_else(|_| fs::read_to_string("/var/lib/dbus/machine-id"))
        .map(|s| s.trim().to_string());
}

pub fn new_session() -> Result<Session, Error> {
    let session = Session {
        access_token: String::new(),
        token_expiry: 0,
        refresh_token: env::var("AWAFY_TOKEN")?,
        device_id: get_device_id()?,
    };
    Ok(session)
}
