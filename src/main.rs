use dotenv::dotenv;
use futures_util::stream::StreamExt;
use std::env;
use std::error::Error;
use tokio_tungstenite::connect_async;
use url::Url;

mod db;
mod models;

use models::TickerData;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    println!("Connecting to database: {}", database_url);

    let pool = db::init_db(&database_url).await?;

    let url = Url::parse("wss://fstream.binance.com/ws/!miniTicker@arr")?;
    let (mut ws_stream, _) = connect_async(url.as_str()).await?;

    println!("Connected to Binance WebSocket!");

    // Receive messages from the Binance WebSocket stream and save the ticker data to the database
    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(msg) => {
                if let Ok(tickers) = serde_json::from_str::<Vec<TickerData>>(&msg.to_string()) {
                    for ticker in tickers {
                        if let Err(e) = db::save_ticker_data(&pool, &ticker).await {
                            eprintln!("Error saving ticker data: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => eprintln!("Error receiving message: {:?}", e),
        }
    }

    Ok(())
}
