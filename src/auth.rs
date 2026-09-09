use crate::models::Session;
use anyhow::{Context, Error, Result};
use std::{env, fs};
use winreg::HKLM;

pub fn get_device_id() -> Result<String> {
    match std::env::consts::OS {
        "linux" => {
            let mID = fs::read_to_string("/etc/machine-id")
                .or_else(|_| fs::read_to_string("/var/lib/dbus/machine-id"))
                .context("Failed to read Linux machine-id from standard locations")?
                .trim()
                .to_string();

            Ok("linux no work yet".to_string())
        }
        "windows" => {
            let mID: String = HKLM
                .open_subkey("SOFTWARE\\Microsoft\\Cryptography")?
                .get_value("MachineGuid")?;

            Ok("Windows no work yet".to_string())
        }
        _ => {
            anyhow::bail!("Unsupported OS :(")
        }
    }
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
