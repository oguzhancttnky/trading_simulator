use dotenv::dotenv;
use std::env;
use std::error::Error;
use tokio_tungstenite::{connect_async, accept_async, tungstenite::Message};
use url::Url;
use tokio::net::TcpListener;
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use tokio::time::{interval, Duration};

mod db;
mod models;

use models::{TickerData, PaginationParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Connecting to database: {}", database_url);
    let pool = Arc::new(db::init_db(&database_url).await?);

    // Spawn Binance WebSocket listener as a separate task
    let binance_pool = Arc::clone(&pool);
    tokio::spawn(async move {
        if let Err(e) = handle_binance_ws(binance_pool).await {
            eprintln!("Binance WebSocket error: {:?}", e);
        }
    });

    let bind_addr = env::var("WEBSOCKET_URL").expect("WEBSOCKET_URL must be set");
    let listener = TcpListener::bind(&bind_addr).await?;
    println!("WebSocket server started on {}", bind_addr);

    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from {}", addr);
        let pool_clone = Arc::clone(&pool);
        tokio::spawn(handle_connection(stream, pool_clone));
    }

    Ok(())
}

async fn handle_binance_ws(pool: Arc<sqlx::PgPool>) -> Result<(), Box<dyn Error>> {
    let url = Url::parse("wss://fstream.binance.com/ws/!miniTicker@arr")?;
    let (mut ws_stream, _) = connect_async(url.as_str()).await?;

    println!("Connected to Binance WebSocket!");

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

async fn handle_connection(
    stream: tokio::net::TcpStream,
    pool: Arc<sqlx::PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    println!("WebSocket connection established");

    let (mut write, mut read) = ws_stream.split();
    let mut interval = interval(Duration::from_secs(60)); // Changed to 60 seconds

    let mut current_page = 1;
    let mut items_per_page = 30;

    // Send initial data immediately
    if let Ok(tickers) = db::get_latest_tickers(&pool, current_page, items_per_page).await {
        if let Ok(json) = serde_json::to_string(&tickers) {
            let _ = write.send(Message::Text(json.into())).await;
        }
    }

    loop {
        tokio::select! {
            Some(msg_result) = read.next() => {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(params) = serde_json::from_str::<PaginationParams>(&text) {
                            if let Some(page) = params.page {
                                current_page = page;
                                // Send updated data immediately after page change
                                if let Ok(tickers) = db::get_latest_tickers(&pool, current_page, items_per_page).await {
                                    if let Ok(json) = serde_json::to_string(&tickers) {
                                        let _ = write.send(Message::Text(json.into())).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        eprintln!("Error receiving message: {:?}", e);
                        break;
                    }
                    _ => {}
                }
            }

            _ = interval.tick() => {
                if let Ok(tickers) = db::get_latest_tickers(&pool, current_page, items_per_page).await {
                    if let Ok(json) = serde_json::to_string(&tickers) {
                        if let Err(e) = write.send(Message::Text(json.into())).await {
                            eprintln!("Error sending message: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
