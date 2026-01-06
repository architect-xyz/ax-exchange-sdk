//! Smoke tests for the ArchitectX SDK.
//!
//! These tests connect to the actual production endpoint and verify basic functionality.
//! They require API credentials to be provided via environment variables:
//! - `EXCHANGE_API_KEY`: Your API key
//! - `EXCHANGE_API_SECRET`: Your API secret
//!
//! Run with: `cargo test -p ax-exchange-sdk --features smoke-tests --test smoke_tests`

#![cfg(feature = "smoke-tests")]

use anyhow::Result;
use ax_exchange_sdk::ArchitectX;
use std::env;
use std::time::Duration;
use tokio::time::timeout;

fn get_credentials() -> Result<(String, String)> {
    let api_key = env::var("EXCHANGE_API_KEY")
        .map_err(|_| anyhow::anyhow!("EXCHANGE_API_KEY env var not set"))?;
    let api_secret = env::var("EXCHANGE_API_SECRET")
        .map_err(|_| anyhow::anyhow!("EXCHANGE_API_SECRET env var not set"))?;
    Ok((api_key, api_secret))
}

#[tokio::test]
async fn test_whoami() -> Result<()> {
    let (api_key, api_secret) = get_credentials()?;
    let client = ArchitectX::with_credentials(&api_key, &api_secret)?;
    client.refresh_user_token(true).await?;

    let api_gateway = client.api_gateway()?;
    let whoami = api_gateway.whoami().await?;

    assert!(!whoami.id.is_empty(), "user id should not be empty");
    assert!(!whoami.username.is_empty(), "username should not be empty");

    Ok(())
}

#[tokio::test]
async fn test_get_instruments() -> Result<()> {
    let (api_key, api_secret) = get_credentials()?;
    let client = ArchitectX::with_credentials(&api_key, &api_secret)?;

    let api_gateway = client.api_gateway()?;
    let instruments = api_gateway.get_instruments().await?;

    assert!(
        !instruments.instruments.is_empty(),
        "instruments list should not be empty"
    );

    Ok(())
}

#[tokio::test]
async fn test_marketdata_ws_connect() -> Result<()> {
    use ax_exchange_sdk::protocol::marketdata_publisher::MarketdataEvent;

    let (api_key, api_secret) = get_credentials()?;
    let client = ArchitectX::with_credentials(&api_key, &api_secret)?;
    client.refresh_user_token(true).await?;

    let mut md = client.marketdata_ws().await?;

    // Wait for a heartbeat with a timeout
    let received_event = timeout(Duration::from_secs(10), async {
        loop {
            if let Some(event) = md.next().await? {
                if matches!(event.as_ref(), MarketdataEvent::Heartbeat(_)) {
                    return Ok::<_, anyhow::Error>(true);
                }
            }
        }
    })
    .await;

    assert!(
        received_event.is_ok(),
        "should receive marketdata event within timeout"
    );

    Ok(())
}
