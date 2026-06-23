#[macro_export]
macro_rules! with_private_client {
    ($client:ident, $body:expr) => {{
        dotenv::dotenv().ok();

        let (api_key, api_secret, env_str) =
            require_env!("AX_API_KEY", "AX_API_SECRET", "AX_ENVIRONMENT");

        let environment: Environment = env_str.parse().expect("Invalid environment variable");
        let $client = ArchitectX::new(environment, Some(api_key), Some(api_secret))?;

        let result = { $body };

        result
    }};
}

#[macro_export]
macro_rules! with_public_client {
    ($client:ident, $body:expr) => {{
        let $client = WsClient::new_public(thalex_rust_sdk::types::Environment::Testnet)
            .await
            .unwrap();

        let result = { $body };

        match $client.shutdown("Test complete").await {
            Ok(_) => (),
            Err(e) => eprintln!("Error during client shutdown: {:?}", e),
        }
        result
    }};
}

#[macro_export]
macro_rules! require_env {
    ($($var:expr),+ $(,)?) => {
        (
            $(
                match std::env::var($var) {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Skipping test: {} not set", $var);
                        return Ok(());
                    }
                }
            ),+
        )
    };
}

// #[macro_export]
// macro_rules! no_params_private_rpc_test {
//     ($name:ident, $method:ident, $label:literal, $namespace:ident) => {
//         #[tokio::test]
//         #[serial_test::serial(private_rpc)]
//         async fn $name() {
//             let result =
//                 with_private_client!(client, { client.rpc().$namespace().$method().await });
//             assert!(result.is_ok(), "{} failed: {:?}", $label, result.err());
//         }
//     };
// }

// #[macro_export]
// macro_rules! params_private_rpc_test {
//     ($name:ident, $params:expr, $method:ident, $label:literal, $namespace:ident) => {
//         #[tokio::test]
//         #[serial_test::serial(private_rpc)]
//         async fn $name() {
//             let result =
//                 with_private_client!(client, { client.rpc().$namespace().$method($params).await });
//             assert!(result.is_ok(), "{} failed: {:?}", $label, result.err());
//         }
//     };
// }
// #[macro_export]
// macro_rules! params_rpc_test {
//     ($name:ident, $params:expr, $method:ident, $label:literal, $namespace:ident, $result:ident) => {
//         #[tokio::test]
//         #[serial_test::serial(public_rpc)]
//         async fn $name() {
//             let result =
//                 with_public_client!(client, { client.rpc().$namespace().$method($params).await });
//             assert!(result.$result(), "{} failed: {:?}", $label, result.err());
//         }
//     };
// }

// #[macro_export]
// macro_rules! no_params_rpc_test {
//     ($name:ident, $method:ident, $label:literal, $namespace:ident, $result:ident) => {
//         #[tokio::test]
//         #[serial_test::serial(public_rpc)]
//         async fn $name() {
//             let result = with_public_client!(client, { client.rpc().$namespace().$method().await });
//             assert!(result.$result(), "{} failed: {:?}", $label, result.err());
//         }
//     };
// }
