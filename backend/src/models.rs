use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TickerData {
    pub E: i64,    // Event time
    pub s: String, // Symbol
    pub c: String, // Close price
    pub o: String, // Open price
    pub h: String, // High price
    pub l: String, // Low price
    pub q: String, // Total traded quote asset volume
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeData {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolData {
    pub event_time: DateTime<Utc>,
    pub symbol: String,
    pub close_price: f64,
    pub open_price: f64,
    pub high_price: f64,
    pub low_price: f64,
    pub quote_volume: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub data: Vec<VolumeData>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
