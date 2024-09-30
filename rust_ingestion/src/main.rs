/*
 * @Author: Jiang Han
 * @Date: 2024-09-29 20:16:01
 * @Description:
 */
use postgres::{Client, NoTls};
use reqwest;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize)]
struct BlockInfo {
    height: u32,
}

async fn fetch_block_height() -> Result<u32, reqwest::Error> {
    let response = reqwest::get("https://blockchain.info/latestblock")
        .await?
        .json::<BlockInfo>()
        .await?;
    Ok(response.height)
}

async fn run_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::connect(
        "host=postgres user=postgres password=postgre dbname=bitcoin_explorer",
        NoTls,
    )?;

    loop {
        match fetch_block_height().await {
            Ok(block_height) => {
                client.execute(
                    "INSERT INTO block_data (block_height) VALUES ($1)",
                    &[&(block_height as i32)],
                )?;
                println!("Inserted block height: {}", block_height);
            }
            Err(e) => eprintln!("Error fetching block height: {}", e),
        }

        sleep(Duration::from_secs(60)).await;
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_ingestion().await {
        eprintln!("Application error: {}", e);
    }
}
