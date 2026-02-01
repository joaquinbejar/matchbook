-- Migration: Create retention policies
-- Description: Configures data retention for time-series tables

-- Retention policy for trades: keep 1 year of data
-- Older data is automatically dropped by TimescaleDB
SELECT add_retention_policy('trades', 
    INTERVAL '1 year',
    if_not_exists => TRUE
);

-- Note: Continuous aggregates (candles) are not affected by retention policies
-- on the underlying hypertable. They maintain their own data.
-- If you want to drop old candle data, create separate retention policies:

-- Optional: Retention for 1-minute candles (keep 30 days)
-- Uncomment if you want to limit 1m candle history
-- SELECT add_retention_policy('candles_1m', INTERVAL '30 days', if_not_exists => TRUE);

-- Optional: Retention for 5-minute candles (keep 90 days)
-- SELECT add_retention_policy('candles_5m', INTERVAL '90 days', if_not_exists => TRUE);

COMMENT ON EXTENSION timescaledb IS 'TimescaleDB extension for time-series data with automatic partitioning and retention';
