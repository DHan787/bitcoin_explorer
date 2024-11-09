use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use futures_util::{SinkExt, StreamExt};
use reqwest;
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
    let listener = TcpListener::bind("0.0.0.0:5001").await.unwrap();

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let ws_stream = accept_async(stream)
                .await
                .expect("Failed to accept WebSocket");
            let (mut write, _) = ws_stream.split();

            let mut block_height_interval = tokio::time::interval(Duration::from_secs(600));
            let mut bitcoin_price_interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                tokio::select! {
                    _ = block_height_interval.tick() => {
                        let block_height = fetch_block_height().await.unwrap_or(0);
                        let data = format!(
                            r#"{{"block_height": {}, "price": null}}"#,
                            block_height
                        );
                        let msg = Message::Text(data);
                        write.send(msg).await.unwrap();
                        println!("Sent block height: {}", block_height);
                    },
                    _ = bitcoin_price_interval.tick() => {
                        let bitcoin_price = fetch_bitcoin_price().await.unwrap_or(0.0);
                        let data = format!(
                            r#"{{"block_height": null, "price": {}}}"#,
                            bitcoin_price
                        );
                        let msg = Message::Text(data);
                        write.send(msg).await.unwrap();
                        println!("Sent Bitcoin price: ${}", bitcoin_price);
                    }
                }

                // Pause before fetching the next data
                sleep(Duration::from_secs(60)).await;
            }
        });
    }
}

async fn get_all_data() -> impl Responder {
    let (client, connection) = tokio_postgres::connect(
        "host=postgres user=postgres password=postgre dbname=bitcoin_explorer",
        NoTls,
    )
    .await
    .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let rows = client
        .query(
            "SELECT block_data.block_height, price_data.price_usd, block_data.block_timestamp \
        FROM block_data \
        JOIN price_data ON block_data.block_timestamp = price_data.price_timestamp",
            &[],
        )
        .await
        .unwrap();
    println!("fetched rows: {:?}", rows);
    let data: Vec<_> = rows.iter().map(|row| {
        let block_height: i32 = row.get(0);
        let price: f64 = row.get(1);
        let timestamp: String = row.get(2);
        serde_json::json!({ "block_height": block_height, "price": price, "timestamp": timestamp })
    }).collect();
    println!("fetched data: {:?}", data);
    HttpResponse::Ok().json(data)
}

async fn run_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let (client, connection) = tokio_postgres::connect(
        "host=postgres user=postgres password=postgre dbname=bitcoin_explorer",
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
                client
                    .execute(
                        "INSERT INTO price_data (price_usd) VALUES ($1::DOUBLE PRECISION)",
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tokio::spawn(websocket_server());
    tokio::spawn(async move {
        if let Err(e) = run_ingestion().await {
            eprintln!("Application error: {}", e);
        }
    });

    HttpServer::new(|| {
        App::new()
            .wrap(Cors::permissive())
            .route("/all-data", web::get().to(get_all_data))
    })
    .bind("0.0.0.0:5002")?
    .run()
    .await
}
