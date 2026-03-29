use anyhow::Result;
use ax_exchange_sdk::{environment::Environment, ArchitectX};

#[tokio::main]
async fn main() -> Result<()> {
    let client = ArchitectX::new(
        Environment::Sandbox,
        Some("your-api-key"),
        Some("your-api-secret"),
    )?;

    let api = client.api_gateway()?;
    let instruments = api.get_instruments().await?;
    println!("{:?}", instruments);

    Ok(())
}
