use crate::models::TickerData;
use sqlx::PgPool;

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
    sqlx::query(
        r#"
        INSERT INTO ticker_data 
        (symbol, close_price, open_price, high_price, low_price, quote_volume, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7::double precision / 1000) AT TIME ZONE 'UTC')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(&ticker.s)
    .bind(ticker.c.parse::<f64>().unwrap_or_default())
    .bind(ticker.o.parse::<f64>().unwrap_or_default())
    .bind(ticker.h.parse::<f64>().unwrap_or_default())
    .bind(ticker.l.parse::<f64>().unwrap_or_default())
    .bind(ticker.q.parse::<f64>().unwrap_or_default())
    .bind(ticker.E)
    .execute(pool)
    .await?;

    Ok(())
}
