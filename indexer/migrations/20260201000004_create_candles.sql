-- Migration: Create candles continuous aggregates
-- Description: OHLCV data aggregated from trades using TimescaleDB continuous aggregates

-- 1-minute candles (base aggregate)
CREATE MATERIALIZED VIEW IF NOT EXISTS candles_1m
WITH (timescaledb.continuous) AS
SELECT
    market_id,
    time_bucket('1 minute', timestamp) AS bucket,
    first(price, timestamp) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, timestamp) AS close,
    sum(quantity) AS volume,
    sum(quantity * price) AS quote_volume,
    count(*) AS trade_count
FROM trades
GROUP BY market_id, time_bucket('1 minute', timestamp)
WITH NO DATA;

-- Refresh policy for 1-minute candles
SELECT add_continuous_aggregate_policy('candles_1m',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists => TRUE
);

-- 5-minute candles
CREATE MATERIALIZED VIEW IF NOT EXISTS candles_5m
WITH (timescaledb.continuous) AS
SELECT
    market_id,
    time_bucket('5 minutes', timestamp) AS bucket,
    first(price, timestamp) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, timestamp) AS close,
    sum(quantity) AS volume,
    sum(quantity * price) AS quote_volume,
    count(*) AS trade_count
FROM trades
GROUP BY market_id, time_bucket('5 minutes', timestamp)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_5m',
    start_offset => INTERVAL '6 hours',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes',
    if_not_exists => TRUE
);

-- 15-minute candles
CREATE MATERIALIZED VIEW IF NOT EXISTS candles_15m
WITH (timescaledb.continuous) AS
SELECT
    market_id,
    time_bucket('15 minutes', timestamp) AS bucket,
    first(price, timestamp) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, timestamp) AS close,
    sum(quantity) AS volume,
    sum(quantity * price) AS quote_volume,
    count(*) AS trade_count
FROM trades
GROUP BY market_id, time_bucket('15 minutes', timestamp)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_15m',
    start_offset => INTERVAL '12 hours',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes',
    if_not_exists => TRUE
);

-- 1-hour candles
CREATE MATERIALIZED VIEW IF NOT EXISTS candles_1h
WITH (timescaledb.continuous) AS
SELECT
    market_id,
    time_bucket('1 hour', timestamp) AS bucket,
    first(price, timestamp) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, timestamp) AS close,
    sum(quantity) AS volume,
    sum(quantity * price) AS quote_volume,
    count(*) AS trade_count
FROM trades
GROUP BY market_id, time_bucket('1 hour', timestamp)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_1h',
    start_offset => INTERVAL '2 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists => TRUE
);

-- 4-hour candles
CREATE MATERIALIZED VIEW IF NOT EXISTS candles_4h
WITH (timescaledb.continuous) AS
SELECT
    market_id,
    time_bucket('4 hours', timestamp) AS bucket,
    first(price, timestamp) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, timestamp) AS close,
    sum(quantity) AS volume,
    sum(quantity * price) AS quote_volume,
    count(*) AS trade_count
FROM trades
GROUP BY market_id, time_bucket('4 hours', timestamp)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_4h',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '4 hours',
    schedule_interval => INTERVAL '4 hours',
    if_not_exists => TRUE
);

-- 1-day candles
CREATE MATERIALIZED VIEW IF NOT EXISTS candles_1d
WITH (timescaledb.continuous) AS
SELECT
    market_id,
    time_bucket('1 day', timestamp) AS bucket,
    first(price, timestamp) AS open,
    max(price) AS high,
    min(price) AS low,
    last(price, timestamp) AS close,
    sum(quantity) AS volume,
    sum(quantity * price) AS quote_volume,
    count(*) AS trade_count
FROM trades
GROUP BY market_id, time_bucket('1 day', timestamp)
WITH NO DATA;

SELECT add_continuous_aggregate_policy('candles_1d',
    start_offset => INTERVAL '30 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Comments
COMMENT ON MATERIALIZED VIEW candles_1m IS '1-minute OHLCV candles aggregated from trades';
COMMENT ON MATERIALIZED VIEW candles_5m IS '5-minute OHLCV candles aggregated from trades';
COMMENT ON MATERIALIZED VIEW candles_15m IS '15-minute OHLCV candles aggregated from trades';
COMMENT ON MATERIALIZED VIEW candles_1h IS '1-hour OHLCV candles aggregated from trades';
COMMENT ON MATERIALIZED VIEW candles_4h IS '4-hour OHLCV candles aggregated from trades';
COMMENT ON MATERIALIZED VIEW candles_1d IS '1-day OHLCV candles aggregated from trades';
