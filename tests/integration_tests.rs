use anyhow::Result;
use ax_exchange_sdk::{
    ArchitectX, PlaceOrder, SelfTradeBehavior, Side,
    environment::Environment,
    protocol::{api_gateway::GetTransactionsRequest, order_gateway::OrderReference},
    trading::{OrderState, TimeInForce},
};
use rust_decimal::Decimal;
use std::str::FromStr;

#[macro_use]
mod common;

#[tokio::test]
async fn test_instruments() -> Result<()> {
    with_private_client!(client, {
        let api = client.api_gateway()?;
        let instruments = api.get_instruments().await?;
        assert!(
            !instruments.instruments.is_empty(),
            "Expected at least one instrument"
        );

        println!("Fetched {} instruments", instruments.instruments.len());
        if let Some(first) = instruments.instruments.first() {
            println!("First instrument: {:?}", first.0.symbol);
        }

        Ok(())
    })
}

#[tokio::test]
async fn test_order_life_cycle() -> Result<()> {
    with_private_client!(client, {
        let order_ws = client.order_gateway_ws().await?;

        println!("Waiting for connection...");
        order_ws.wait_for_connection().await;
        println!("Connected to order gateway.");

        let open_orders = order_ws.get_open_orders().await?;
        println!("Currently have {} open orders.", open_orders.orders.len());

        let symbol = "EURUSD-PERP";
        // we now get the market price of EURUSD such that we can place a resting order at a reasonable price level
        let md_api = client.api_gateway()?;
        let eurusd_ticker = md_api
            .get_tickers()
            .await?
            .tickers
            .into_iter()
            .find(|t| t.symbol == symbol)
            .unwrap_or_else(|| panic!("{} ticker not found", symbol));
        println!("Current {} price: {:?}", symbol, eurusd_ticker.bid_price);

        let eurusd_instrument = md_api
            .get_instruments()
            .await?
            .instruments
            .into_iter()
            .find(|i| i.0.symbol == symbol)
            .unwrap_or_else(|| panic!("{} instrument not found", symbol));

        let side = Side::Buy;
        let quantity = 100;

        // we now place a resting order at a price slightly below the current market price to ensure it rests in the order book.
        // we round to the tick size of the instrument to ensure the price is valid.
        let tick_size = eurusd_instrument.0.tick_size;
        let raw_price = eurusd_ticker
            .bid_price
            .ok_or_else(|| anyhow::anyhow!("{} ticker has no bid price", symbol))?;

        let price = Decimal::from_str(&format!(
            "{:.1$}",
            raw_price * Decimal::new(99, 2), // place order at 99% of the current market price
            tick_size.normalize().scale() as usize
        ))?;

        let place_order = PlaceOrder {
            symbol: symbol.to_string(),
            side,
            quantity,
            price,
            time_in_force: TimeInForce::GoodTillCanceled, // ensure the order stays open until we cancel it
            post_only: true, // ensure the order rests and doesn't take liquidity
            tag: Some("test_order".to_string()),
            clord_id: None,
            self_trade_prevention: SelfTradeBehavior::CancelBoth,
            account_id: None, // use default account
        };

        let res = order_ws.place_order(place_order).await?;

        println!("Placed order: {:?}", res);

        let order_status = client
            .order_gateway()
            .expect("Failed to get order gateway client")
            .order_status(OrderReference::OrderId(res.order_id.clone()))
            .await;

        assert!(
            order_status.is_ok(),
            "Failed to fetch order status after placing order: {:?}",
            order_status.err()
        );

        let status = order_status.unwrap();
        assert_eq!(
            status.order_id, res.order_id,
            "Order ID in status does not match placed order"
        );

        assert!([OrderState::Accepted, OrderState::Pending].contains(&status.state),);

        println!("Order status after placing order: {:?}", status);
        // we now cancel the order immediately to trigger a cancel event as well
        let cancel_res = order_ws.cancel_order(&res.order_id).await;

        assert!(
            cancel_res.is_ok(),
            "Failed to cancel order: {:?}",
            cancel_res.err()
        );

        let cancel_accepted = cancel_res.unwrap();
        assert!(
            cancel_accepted.cancel_request_accepted,
            "Cancel request was not accepted: {:?}",
            cancel_accepted
        );

        tokio::time::sleep(std::time::Duration::from_secs(1)).await; // wait a second to ensure the cancel event is processed
        let order_status_after_cancel = client
            .order_gateway()
            .expect("Failed to get order gateway client")
            .order_status(OrderReference::OrderId(res.order_id.clone()))
            .await?;

        assert_eq!(
            order_status_after_cancel.order_id, res.order_id,
            "Order ID in status does not match placed order after cancel"
        );

        assert_eq!(
            order_status_after_cancel.state,
            OrderState::Canceled,
            "Order state after cancel is not Canceled"
        );
        println!(
            "Order status after canceling order: {:?}",
            order_status_after_cancel
        );
        Ok(())
    })
}

// we test the endpoint for risk
#[tokio::test]
async fn test_user_risk_snapshot() -> Result<()> {
    with_private_client!(client, {
        match client.refresh_user_token(true).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to refresh user token: {:?}", e);
                return Ok(());
            }
        }
        let api = client.api_gateway()?;
        let risk_snapshot = api.get_risk_snapshot().await;
        assert!(
            risk_snapshot.is_ok(),
            "Failed to fetch risk snapshot: {:?}",
            risk_snapshot.err()
        );
        Ok(())
    })
}

// we test the endpoint for risk
#[tokio::test]
async fn test_positions() -> Result<()> {
    with_private_client!(client, {
        match client.refresh_user_token(true).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to refresh user token: {:?}", e);
                return Ok(());
            }
        }
        let api = client.api_gateway()?;
        let positions = api.get_positions().await;
        assert!(
            positions.is_ok(),
            "Failed to fetch positions: {:?}",
            positions.err()
        );
        Ok(())
    })
}

// we test the endpoint for funding transactions
#[tokio::test]
async fn test_funding_transactions() -> Result<()> {
    with_private_client!(client, {
        match client.refresh_user_token(true).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to refresh user token: {:?}", e);
                return Ok(());
            }
        }
        let request = GetTransactionsRequest {
            transaction_types: vec!["funding".to_string()],
        };

        let api = client.api_gateway()?;
        let funding_transactions = api.get_transactions(request).await;
        assert!(
            funding_transactions.is_ok(),
            "Failed to fetch funding transactions: {:?}",
            funding_transactions.err()
        );
        Ok(())
    })
}
