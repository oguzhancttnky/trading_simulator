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
