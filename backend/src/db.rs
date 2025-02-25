use crate::models::{PaginatedResponse, PaginationParams, SymbolData, TickerData, VolumeData};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

pub async fn init_db(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(database_url).await?;

    // Create the timescaledb extension
    sqlx::query("CREATE EXTENSION IF NOT EXISTS timescaledb;")
        .execute(&pool)
        .await?;

    // Create the ticker_data table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ticker_data (
            symbol TEXT,
            close_price DECIMAL,
            open_price DECIMAL,
            high_price DECIMAL,
            low_price DECIMAL,
            quote_volume DECIMAL,
            created_at TIMESTAMPTZ
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Create the hypertable with a chunk interval of 10 minutes on the created_at column to store the data with time-series optimizations
    sqlx::query(
        r#"
        SELECT create_hypertable('ticker_data', 'created_at', 
            if_not_exists => TRUE,
            chunk_time_interval => INTERVAL '10 minutes'
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Compress the hypertable with the created_at column as the order and symbol as the segment
    sqlx::query(
        r#"
        ALTER TABLE ticker_data SET (
            timescaledb.compress,
            timescaledb.compress_orderby = 'created_at DESC',
            timescaledb.compress_segmentby = 'symbol'
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Create policy to drop chunks after 1 hour
    sqlx::query(
        r#"
        SELECT add_retention_policy('ticker_data', 
            INTERVAL '1 hour',
            if_not_exists => TRUE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Create policy to compress chunks after 10 minutes
    sqlx::query(
        r#"
        SELECT add_compression_policy('ticker_data', 
            INTERVAL '10 minutes',
            if_not_exists => TRUE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Create an index on the symbol and created_at columns to speed up queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_ticker_data_symbol 
        ON ticker_data (symbol, created_at DESC)
        WITH (timescaledb.transaction_per_chunk);
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn save_ticker_data(pool: &PgPool, ticker: &TickerData) -> Result<(), sqlx::Error> {
    // Parse string values to f64 before inserting
    let close_price = ticker.c.parse::<f64>().unwrap_or_default();
    let open_price = ticker.o.parse::<f64>().unwrap_or_default();
    let high_price = ticker.h.parse::<f64>().unwrap_or_default();
    let low_price = ticker.l.parse::<f64>().unwrap_or_default();
    let quote_volume = ticker.q.parse::<f64>().unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO ticker_data 
        (symbol, close_price, open_price, high_price, low_price, quote_volume, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7::double precision / 1000) AT TIME ZONE 'UTC')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(&ticker.s)
    .bind(close_price)
    .bind(open_price)
    .bind(high_price)
    .bind(low_price)
    .bind(quote_volume)
    .bind(ticker.E)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_latest_tickers(
    pool: &PgPool,
    page: i64,
    per_page: i64,
) -> Result<PaginatedResponse, sqlx::Error> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT symbol) FROM ticker_data")
        .fetch_one(pool)
        .await?;

    let offset = (page - 1) * per_page;

    // Query the latest data for each symbol and sort by volume in descending order with pagination
    let volume_data = sqlx::query(
        r#"
        WITH LatestData AS (
            SELECT DISTINCT ON (symbol) 
                symbol,
                close_price,
                quote_volume,
                created_at
            FROM ticker_data
            ORDER BY symbol ASC, created_at DESC
        ),
        SortedData AS (
            SELECT 
                symbol,
                CAST(close_price AS DOUBLE PRECISION) as close_price,
                CAST(quote_volume AS DOUBLE PRECISION) as quote_volume
            FROM LatestData
            ORDER BY quote_volume DESC
            LIMIT $1
            OFFSET $2
        )
        SELECT * FROM SortedData
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .try_map(|row: sqlx::postgres::PgRow| {
        Ok(VolumeData {
            symbol: row.try_get("symbol")?,
            price: row.try_get("close_price")?,
            volume: row.try_get("quote_volume")?,
        })
    })
    .fetch_all(pool)
    .await?;

    Ok(PaginatedResponse {
        data: volume_data,
        total,
        page,
        per_page,
    })
}

pub async fn get_currency_tickers(
    pool: &PgPool,
    currency: &str,
) -> Result<Vec<SymbolData>, sqlx::Error> {
    let tickers = sqlx::query(
        r#"
        SELECT 
            symbol,
            CAST(close_price AS DOUBLE PRECISION) as close_price,
            CAST(open_price AS DOUBLE PRECISION) as open_price,
            CAST(high_price AS DOUBLE PRECISION) as high_price,
            CAST(low_price AS DOUBLE PRECISION) as low_price,
            CAST(quote_volume AS DOUBLE PRECISION) as quote_volume,
            created_at
        FROM ticker_data 
        WHERE symbol = $1 
        ORDER BY created_at DESC 
        LIMIT 10
        "#,
    )
    .bind(currency)
    .try_map(|row: sqlx::postgres::PgRow| {
        Ok(SymbolData {
            event_time: row.try_get::<DateTime<Utc>, _>("created_at")?,
            symbol: row.try_get("symbol")?,
            close_price: row.try_get("close_price")?,
            open_price: row.try_get("open_price")?,
            high_price: row.try_get("high_price")?,
            low_price: row.try_get("low_price")?,
            quote_volume: row.try_get("quote_volume")?,
        })
    })
    .fetch_all(pool)
    .await?;

    Ok(tickers)
}
