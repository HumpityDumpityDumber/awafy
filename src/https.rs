use crate::constants::JSON_TYPE;
use crate::models::Session;
use anyhow::Context;
use anyhow::Result;
use reqwest::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};

pub fn build_client(device_id: &str, session: Option<&Session>) -> Result<Client> {
    let mut headers = HeaderMap::new();

    headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON_TYPE));
    headers.insert(ACCEPT, HeaderValue::from_static(JSON_TYPE));
    headers.insert("X-Device-Id-Type", HeaderValue::from_static("4"));
    headers.insert("X-Device-Id", HeaderValue::from_str(device_id)?);

    if let Some(session) = session {
        headers.insert(
            "X-Access-Token",
            HeaderValue::from_str(&session.access_token)?,
        );
    }

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .context("Failed to build reqwest client")?;

    return Ok(client);
}
