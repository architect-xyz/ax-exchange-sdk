// examples/price_streaming.rs
use anyhow::Result;
use ax_exchange_sdk::{
    environment::Environment, protocol::marketdata_publisher::SubscriptionLevel, ArchitectX,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("AX_API_KEY")?;
    let api_secret = env::var("AX_API_SECRET")?;
    let environment: Environment = env::var("AX_ENVIRONMENT")
        .unwrap_or_else(|_| "sandbox".to_string())
        .parse()?;

    simple_logger::init_with_level(log::Level::Info)?;
    let client = ArchitectX::new(environment, Some(api_key), Some(api_secret))?;

    let api = client.api_gateway()?;
    println!("Fetching instruments...");
    let instruments = api.get_instruments().await?;

    println!("Collected a total of {}", instruments.instruments.len());

    let mut market_ws = client.marketdata_ws().await?;

    for instrument in instruments.instruments {
        market_ws
            .subscribe(instrument.0.symbol.clone(), SubscriptionLevel::Level1)
            .await?;
    }

    let mut msg_count = 10;
    loop {
        let msg = market_ws.next().await?;
        println!("Received market data event: {:?}", msg);

        msg_count -= 1;
        if msg_count == 0 {
            println!("Received 10 messages, closing connection.");
            break;
        }
    }

    Ok(())
}
