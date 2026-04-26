use anyhow::Result;
use ax_exchange_sdk::{
    environment::Environment,
    protocol::order_gateway::OrderGatewayEvent,
    trading::TimeInForce,
    types::{trading::Side, ws::ConnectionState, PlaceOrder},
    ArchitectX,
};
use rust_decimal::Decimal;
use std::{env, str::FromStr};

#[tokio::main]
async fn main() -> Result<()> {
    let api_key = env::var("AX_API_KEY")?;
    let api_secret = env::var("AX_API_SECRET")?;
    let environment: Environment = env::var("AX_ENVIRONMENT")
        .unwrap_or_else(|_| "sandbox".to_string())
        .parse()?;

    tracing_subscriber::fmt::init();

    let client = ArchitectX::new(environment, Some(api_key), Some(api_secret))?;

    let mut order_ws = client.order_gateway_ws().await?;

    println!("Waiting for connection...");
    order_ws.wait_for_connection().await;
    println!("Connected to order gateway.");

    let open_orders = order_ws.get_open_orders().await?;
    println!("Currently have {} open orders.", open_orders.orders.len());

    let symbol = "XAU-PERP";
    // we now get the market price of XAU such that we can place a resting order at a reasonable price level
    let md_api = client.api_gateway()?;
    let xau_instrument = md_api
        .get_tickers()
        .await?
        .tickers
        .into_iter()
        .find(|t| t.symbol == symbol)
        .expect("XAU-PERP ticker not found");
    println!("Current XAU price: {:?}", xau_instrument.bid_price);

    let side = Side::Buy;
    let quantity = 1;
    let price =
        xau_instrument.bid_price.expect("No Price for symbol!") - Decimal::from_str("10.0")?; // place a resting order $10 below the current price

    let place_order = PlaceOrder {
        symbol: symbol.to_string(),
        side,
        quantity,
        price,
        time_in_force: TimeInForce::GoodTillCanceled, // ensure the order stays open until we cancel it
        post_only: true, // ensure the order rests and doesn't take liquidity
        tag: Some("test_order".to_string()),
        clord_id: None,
    };

    let res = order_ws.place_order(place_order).await?;

    println!("Placed order: {:?}", res);

    // we now cancel the order immediately to trigger a cancel event as well
    let _cancel_res = order_ws.cancel_order(&res.order_id).await?;

    let mut watcher = order_ws.state_watcher();

    println!("Streaming order events (Ctrl-C to exit)...");
    loop {
        tokio::select! {
            msg = order_ws.event_receiver.recv() => {
                match msg {
                    Some(event) => print_event(&event),
                    None => {
                        println!("Order event stream closed.");
                        break;
                    }
                }
            }
            state = watcher.run_till_event() => {
                println!("[connection] state → {state:?}");
                match state {
                    ConnectionState::Exited => break,
                    ConnectionState::Disconnected => {
                        println!("[connection] lost — waiting for reconnect...");
                    }
                    ConnectionState::Connected => {
                        println!("[connection] reconnected.");
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_event(event: &OrderGatewayEvent) {
    match event {
        OrderGatewayEvent::Heartbeat(ts) => {
            println!("[heartbeat] {ts:?}");
        }
        OrderGatewayEvent::OrderAcked(e) => {
            println!(
                "[acked]     order_id={} symbol={} side={:?} qty={} price={}",
                e.order.order_id, e.order.symbol, e.order.side, e.order.quantity, e.order.price
            );
        }
        OrderGatewayEvent::OrderRejected(e) => {
            println!(
                "[rejected]  order_id={} reason={:?}",
                e.order.order_id, e.reject_reason
            );
        }
        OrderGatewayEvent::OrderCanceled(e) => {
            println!("[canceled]  order_id={}", e.order.order_id);
        }
        OrderGatewayEvent::CancelRejected(e) => {
            println!("[cancel_rejected] order_id={}", e.order_id);
        }
        OrderGatewayEvent::OrderPartiallyFilled(e) => {
            println!(
                "[partial]   order_id={} filled={} remaining={}",
                e.order.order_id, e.order.filled_quantity, e.order.remaining_quantity
            );
        }
        OrderGatewayEvent::OrderFilled(e) => {
            println!(
                "[filled]    order_id={} qty={}",
                e.order.order_id, e.order.quantity
            );
        }
        OrderGatewayEvent::OrderReplacedOrAmended(e) => {
            println!(
                "[replaced]  old_order_id={} new_order_id={:?}",
                e.replaced_order.order_id, e.replacement_order_id
            );
        }
        OrderGatewayEvent::OrderExpired(e) => {
            println!("[expired]   order_id={}", e.order.order_id);
        }
        OrderGatewayEvent::OrderDoneForDay(e) => {
            println!("[done_for_day] order_id={}", e.order.order_id);
        }
    }
}
