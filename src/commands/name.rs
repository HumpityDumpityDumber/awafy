use crate::auth::{get_device_id, get_device_name};
use anyhow::Result;

pub async fn main() -> Result<()> {
    println!("{}", get_device_name(&get_device_id()?)?);
    Ok(())
}
