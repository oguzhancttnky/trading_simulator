use dotenv::dotenv;
use std::env;
use std::error::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{connect_async, accept_async, tungstenite::Message};
use url::Url;
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
        tokio::spawn(async move {
            if let Err(e) = route_connection(stream, pool_clone).await {
                eprintln!("Error handling connection: {:?}", e);
            }
        });
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

async fn route_connection(
    stream: TcpStream,
    pool: Arc<sqlx::PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut http_buffer = [0; 1024];
    let bytes_read = stream.peek(&mut http_buffer).await?;
    let request = String::from_utf8_lossy(&http_buffer[..bytes_read]);
    
    // Extract the path from the HTTP request
    let path = request.lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    println!("Received WebSocket request for path: {}", path);
    
    match path {
        "/" => {
            handle_all_currencies(stream, pool).await
        }
        _ if path.starts_with("/currency/") => {
            // Extract currency symbol by splitting the path
            let currency = path.strip_prefix("/currency/")
                .unwrap_or("")
                .to_string();
            
            if currency.is_empty() {
                eprintln!("Invalid currency path: {}", path);
                return Err("Invalid currency path".into());
            }
            
            // Make sure the currency is available in the database
            match db::get_currency_tickers(&pool, &currency).await {
                Ok(tickers) if !tickers.is_empty() => {
                    handle_single_currency(stream, pool, currency).await
                }
                Ok(_) => {
                    eprintln!("No data found for currency: {}", currency);
                    Err(format!("No data found for currency: {}", currency).into())
                }
                Err(e) => {
                    eprintln!("Error fetching currency data: {:?}", e);
                    Err(Box::new(e))
                }
            }
        }
        _ => {
            // Invalid path
            eprintln!("Invalid WebSocket path: {}", path);
            Err("Invalid WebSocket path".into())
        }
    }
}

// Handler for all currencies
async fn handle_all_currencies(
    stream: TcpStream,
    pool: Arc<sqlx::PgPool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    println!("WebSocket connection established for all currencies");

    let (mut write, mut read) = ws_stream.split();
    let mut interval = interval(Duration::from_secs(60));

    let mut current_page = 1;
    let items_per_page = 30;

    // Send initial data
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

// Handler for single currency
async fn handle_single_currency(
    stream: TcpStream,
    pool: Arc<sqlx::PgPool>,
    currency: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Attempting to establish WebSocket connection for currency: {}", currency);
    
    // Check if we have data for this currency before accepting the connection
    match db::get_currency_tickers(&pool, &currency).await {
        Ok(tickers) => {
            if tickers.is_empty() {
                println!("No data found for currency: {}", currency);
                return Err(format!("No data found for currency: {}", currency).into());
            }
            
            println!("Found {} ticker data points for {}", tickers.len(), currency);
        }
        Err(e) => {
            eprintln!("Database error when fetching tickers for {}: {:?}", currency, e);
            return Err(Box::new(e));
        }
    }
    
    // Now accept the WebSocket connection
    let ws_stream = match accept_async(stream).await {
        Ok(stream) => {
            println!("WebSocket handshake successful for currency: {}", currency);
            stream
        }
        Err(e) => {
            eprintln!("WebSocket handshake failed for {}: {:?}", currency, e);
            return Err(Box::new(e));
        }
    };
    
    println!("WebSocket connection established for currency: {}", currency);

    let (mut write, mut read) = ws_stream.split();
    let mut interval = interval(Duration::from_secs(10)); // Reduced to 10 seconds for debugging

    // Send initial data for the specific currency
    match db::get_currency_tickers(&pool, &currency).await {
        Ok(tickers) => {
            match serde_json::to_string(&tickers) {
                Ok(json) => {
                    println!("Sending initial data for {}: {} records", currency, tickers.len());
                    if let Err(e) = write.send(Message::Text(json.into())).await {
                        eprintln!("Error sending initial data for {}: {:?}", currency, e);
                        return Err(Box::new(e));
                    }
                }
                Err(e) => {
                    eprintln!("Error serializing tickers for {}: {:?}", currency, e);
                    return Err(Box::new(e));
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching initial tickers for {}: {:?}", currency, e);
            return Err(Box::new(e));
        }
    }

    loop {
        tokio::select! {
            Some(msg_result) = read.next() => {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        println!("Received message from client for {}: {}", currency, text);
                        match db::get_currency_tickers(&pool, &currency).await {
                            Ok(tickers) => {
                                match serde_json::to_string(&tickers) {
                                    Ok(json) => {
                                        println!("Sending data update for {}: {} records", currency, tickers.len());
                                        if let Err(e) = write.send(Message::Text(json.into())).await {
                                            eprintln!("Error sending data for {}: {:?}", currency, e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error serializing tickers for {}: {:?}", currency, e);
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error fetching tickers for {}: {:?}", currency, e);
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(reason)) => {
                        println!("WebSocket close requested for {}: {:?}", currency, reason);
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error receiving message for {}: {:?}", currency, e);
                        break;
                    }
                    _ => {
                        println!("Received non-text message for {}", currency);
                    }
                }
            }

            _ = interval.tick() => {
                println!("Sending periodic update for {}", currency);
                match db::get_currency_tickers(&pool, &currency).await {
                    Ok(tickers) => {
                        if tickers.is_empty() {
                            println!("No data found for currency: {} during periodic update", currency);
                            continue;
                        }
                        
                        match serde_json::to_string(&tickers) {
                            Ok(json) => {
                                println!("Sending periodic data for {}: {} records", currency, tickers.len());
                                if let Err(e) = write.send(Message::Text(json.into())).await {
                                    eprintln!("Error sending periodic data for {}: {:?}", currency, e);
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error serializing tickers for {}: {:?}", currency, e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error fetching tickers during periodic update for {}: {:?}", currency, e);
                        break;
                    }
                }
            }
        }
    }

    println!("WebSocket connection closed for currency: {}", currency);
    Ok(())
}