# ax-exchange-sdk

Rust SDK for the [ArchitectX](https://architect.exchange) derivatives exchange.

Provides typed REST and WebSocket clients for market data, order management, and account operations with automatic token management.

## Quick Start

```toml
[dependencies]
ax-exchange-sdk = { git = "https://github.com/architect-xyz/adx.git" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

```rust
// examples/create_client.rs
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

```

## Features

- REST clients for API Gateway and Order Gateway
- WebSocket clients for streaming market data and order management
- Automatic bearer token caching and refresh
- Typed request/response structs with serde
- Local order book maintenance via WebSocket

## Documentation

- [API Reference](https://docs.architect.exchange/api-reference)
- [OpenAPI Spec](https://docs.architect.exchange/openapi/api-gateway.json)

## License

AGPL-3.0-only

## Contributing
Contributions are welcome! Please open an issue or submit a pull request.

## Development

The sdk is built using a makefile for common tasks:
- Run `make fmt` to format the code
- Run `make lint` to check for lint warnings
- Run `make test` to run tests
- Run `make build` to build the library
- Run `make all` to run all of the above