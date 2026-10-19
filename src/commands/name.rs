use crate::auth::{get_device_id, get_device_name};
use anyhow::Result;

pub async fn main() -> Result<()> {
    let d_id = get_device_id()?;
    println!("device id: {}", d_id);
    println!("device name: {}", get_device_name(&d_id)?);
    Ok(())
}
