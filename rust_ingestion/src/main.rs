use futures_util::{SinkExt, StreamExt};
use reqwest;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tokio_postgres::NoTls;
use tokio_tungstenite::accept_async;
use tungstenite::protocol::Message;

#[derive(Deserialize)]
struct BlockInfo {
    height: u32,
}

#[derive(Deserialize)]
struct MarketData {
    bitcoin: CoinMetrics,
}

#[derive(Deserialize)]
struct CoinMetrics {
    usd: f64,
}
struct PriceData {
    price_usd: Decimal,
}

async fn fetch_block_height() -> Result<u32, reqwest::Error> {
    let response = reqwest::get("https://blockchain.info/latestblock")
        .await?
        .json::<BlockInfo>()
        .await?;
    Ok(response.height)
}

async fn fetch_bitcoin_price() -> Result<f64, reqwest::Error> {
    let response =
        reqwest::get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd")
            .await?
            .json::<MarketData>()
            .await?;
    Ok(response.bitcoin.usd)
}

async fn websocket_server() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Failed to accept WebSocket");
            let (mut write, _) = ws_stream.split();

            loop {
                // Fetch on-chain block height
                let block_height = fetch_block_height().await.unwrap_or(0);
                // Fetch off-chain Bitcoin price
                let bitcoin_price = fetch_bitcoin_price().await.unwrap_or(0.0);

                // Create the message
                let data = format!(
                    r#"{{"block_height": {}, "price": {}}}"#,
                    block_height, bitcoin_price
                );
                let msg = Message::Text(data);

                // Send the message to the WebSocket client
                write.send(msg).await.unwrap();

                // Pause before fetching the next data
                sleep(Duration::from_secs(60)).await;
            }
        });
    }
}

async fn run_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=postgre dbname=bitcoin_explorer",
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    loop {
        // Fetch on-chain block height
        match fetch_block_height().await {
            Ok(block_height) => {
                client
                    .execute(
                        "INSERT INTO block_data (block_height) VALUES ($1)",
                        &[&(block_height as i32)],
                    )
                    .await?;
                println!("Inserted block height: {}", block_height);
            }
            Err(e) => eprintln!("Error fetching block height: {}", e),
        }

        // Fetch off-chain Bitcoin price
        match fetch_bitcoin_price().await {
            Ok(price) => {
                // Directly pass `f64` for NUMERIC
                client
                    .execute(
                        "INSERT INTO price_data (price_usd) VALUES ($1::NUMERIC)",
                        &[&price],
                    )
                    .await?;
                println!("Inserted Bitcoin price: ${}", price);
            }
            Err(e) => eprintln!("Error fetching Bitcoin price: {}", e),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(websocket_server());

    if let Err(e) = run_ingestion().await {
        eprintln!("Application error: {}", e);
    }
}
