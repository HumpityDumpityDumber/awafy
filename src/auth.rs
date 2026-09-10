use crate::models::Session;
use anyhow::{Context, Error, Result};
use hex;
use sha2::{Digest, Sha256};
use std::{env, fs};
use winreg::HKLM;

pub fn get_device_id() -> Result<String> {
    let m_id: String = match std::env::consts::OS {
        "linux" => fs::read_to_string("/etc/machine-id")
            .or_else(|_| fs::read_to_string("/var/lib/dbus/machine-id"))
            .context("Failed to read Linux machine-id from standard locations")?,
        "windows" => HKLM
            .open_subkey("SOFTWARE\\Microsoft\\Cryptography")?
            .get_value("MachineGuid")?,
        _ => anyhow::bail!("Unsupported OS :("),
    };

    let hash = Sha256::digest(&m_id.trim());

    Ok(hex::encode(&hash[..8]))
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
