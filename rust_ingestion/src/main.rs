use futures_util::{SinkExt, StreamExt};
use reqwest;
use serde::Deserialize;
use std::sync::Arc;
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

            let mut block_height_interval = tokio::time::interval(Duration::from_secs(60));
            let mut bitcoin_price_interval = tokio::time::interval(Duration::from_secs(600));

            loop {
                tokio::select! {
                    // Fetch on-chain block height every 60 seconds
                    _ = block_height_interval.tick() => {
                        if let Ok(block_height) = fetch_block_height().await {
                            let data = format!(r#"{{"block_height": {}, "price": null}}"#, block_height);
                            let msg = Message::Text(data);
                            if let Err(e) = write.send(msg).await {
                                eprintln!("Failed to send block height: {}", e);
                                break;
                            }
                            println!("Sent block height: {}", block_height);
                        }
                    }

                    // Fetch off-chain Bitcoin price every 600 seconds
                    _ = bitcoin_price_interval.tick() => {
                        if let Ok(bitcoin_price) = fetch_bitcoin_price().await {
                            let data = format!(r#"{{"block_height": null, "price": {}}}"#, bitcoin_price);
                            let msg = Message::Text(data);
                            if let Err(e) = write.send(msg).await {
                                eprintln!("Failed to send Bitcoin price: {}", e);
                                break;
                            }
                            println!("Sent Bitcoin price: ${}", bitcoin_price);
                        }
                    }
                }
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

    // Use an Arc to share the client reference across tasks
    let client = Arc::new(client);

    // Task for fetching block height every 60 seconds
    let client_clone = Arc::clone(&client);
    tokio::spawn(async move {
        loop {
            match fetch_block_height().await {
                Ok(block_height) => {
                    if let Err(e) = client_clone
                        .execute(
                            "INSERT INTO block_data (block_height) VALUES ($1)",
                            &[&(block_height as i32)],
                        )
                        .await
                    {
                        eprintln!("Error inserting block height: {}", e);
                    } else {
                        println!("Inserted block height: {}", block_height);
                    }
                }
                Err(e) => eprintln!("Error fetching block height: {}", e),
            }
            sleep(Duration::from_secs(600)).await;
        }
    });

    // Task for fetching Bitcoin price every 600 seconds
    let client_clone = Arc::clone(&client);
    tokio::spawn(async move {
        loop {
            match fetch_bitcoin_price().await {
                Ok(price) => {
                    if let Err(e) = client_clone
                        .execute(
                            "INSERT INTO price_data (price_usd) VALUES ($1::NUMERIC)",
                            &[&price],
                        )
                        .await
                    {
                        eprintln!("Error inserting Bitcoin price: {}", e);
                    } else {
                        println!("Inserted Bitcoin price: ${}", price);
                    }
                }
                Err(e) => eprintln!("Error fetching Bitcoin price: {}", e),
            }
            sleep(Duration::from_secs(60)).await;
        }
    });

    // Run indefinitely
    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::spawn(websocket_server());

    if let Err(e) = run_ingestion().await {
        eprintln!("Application error: {}", e);
    }
}
