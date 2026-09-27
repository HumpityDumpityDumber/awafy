use crate::models::Session;
use anyhow::{Context, Error, Result};
use hex;
use petname::Petnames;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use sha2::{Digest, Sha256};
use std::{env, fs};
use tracing::debug;
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
    let device_id = get_device_id()?;

    let session = Session {
        access_token: String::new(),
        token_expiry: 0,
        refresh_token: env::var("AWAFY_TOKEN")?,
        device_id: device_id.clone(),
        device_name: get_device_name(&device_id)?,
    };
    debug!("{:?}", &session);
    Ok(session)
}

pub fn get_device_name(device_id: &str) -> Result<String> {
    let seed_u64 =
        u64::from_str_radix(&device_id, 16).context("Failed to parse device ID as hex")?;

    let mut rng = SmallRng::seed_from_u64(seed_u64);

    let petnames = Petnames::default();

    let name = petnames.namer(2, "-").iter(&mut rng).next().unwrap();

    Ok(name)
}
