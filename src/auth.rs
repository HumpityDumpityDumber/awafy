use crate::models::Session;
use anyhow::{Context, Error, Result};
use machineid_rs::HWIDComponent;
use machineid_rs::{Encryption, IdBuilder};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use sha2::{Digest, Sha256};
use std::env;
use tracing::debug;

pub fn get_device_id() -> Result<String> {
    let mut builder = IdBuilder::new(Encryption::SHA256);
    builder
        .add_component(HWIDComponent::SystemID)
        .add_component(HWIDComponent::Username)
        .add_component(HWIDComponent::MacAddress);
    let m_id = builder.build("xxxAWAfySuperSecureString420xxx").unwrap();

    let mut hasher = Sha256::new();
    hasher.update(m_id.as_bytes());
    let result = hasher.finalize();

    let hex_string = hex::encode(result);

    Ok(hex_string.chars().take(16).collect())
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

    let petnames = petname::petnames!("names");

    let name = petnames.namer(4, "-").iter(&mut rng).next().unwrap();

    Ok(name)
}
