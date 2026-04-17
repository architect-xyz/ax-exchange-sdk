use anyhow::Result;
use ax_exchange_sdk::{environment::Environment, ArchitectX};

#[macro_use]
mod common;

#[tokio::test]
async fn test_instruments() -> Result<()> {
    with_private_client!(client, {
        // Get the API gateway client
        let api = client.api_gateway()?;

        // Fetch instruments
        let instruments = api.get_instruments().await?;

        // Assert that we got some instruments back
        assert!(
            !instruments.instruments.is_empty(),
            "Expected at least one instrument"
        );

        // Print some info for debugging
        println!("Fetched {} instruments", instruments.instruments.len());
        if let Some(first) = instruments.instruments.first() {
            println!("First instrument: {:?}", first.0.symbol);
        }

        Ok(())
    })
}
