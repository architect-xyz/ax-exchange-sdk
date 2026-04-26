use anyhow::Result;
use ax_exchange_sdk::{
    environment::Environment, protocol::marketdata_publisher::SubscriptionLevel,
    types::ws::ConnectionState, ArchitectX,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("AX_API_KEY")?;
    let api_secret = env::var("AX_API_SECRET")?;
    let environment: Environment = env::var("AX_ENVIRONMENT")
        .unwrap_or_else(|_| "sandbox".to_string())
        .parse()?;

    tracing_subscriber::fmt::init();

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

    let mut watcher = market_ws.state_watcher();

    loop {
        tokio::select! {
            msg = market_ws.market_data_receiver.recv() => {
                match msg {
                    Some(event) => println!("Received market data event: {:?}", event),
                    None => {
                        println!("Market data stream closed.");
                        break;
                    }
                }
            }
            state = watcher.run_till_event() => {
                println!("Connection state changed: {:?}", state);
                match state {
                    ConnectionState::Exited => break,
                    ConnectionState::Disconnected => {
                        println!("Connection lost, waiting for reconnect...");
                    }
                    ConnectionState::Connected => {
                        println!("Reconnected.");
                    }
                }
            }
        }
    }
    Ok(())
}
