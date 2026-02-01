-- Migration: Create trades table with TimescaleDB hypertable
-- Description: Stores executed trades with time-series optimization

-- Create trades table
CREATE TABLE IF NOT EXISTS trades (
    id BIGSERIAL,
    market_id INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    
    -- Order IDs (u128 requires NUMERIC(39) for full precision)
    maker_order_id NUMERIC(39) NOT NULL,
    taker_order_id NUMERIC(39) NOT NULL,
    
    -- Participant addresses
    maker_address VARCHAR(44) NOT NULL,
    taker_address VARCHAR(44) NOT NULL,
    
    -- Trade details (in native units)
    price BIGINT NOT NULL,
    quantity BIGINT NOT NULL,
    
    -- Taker side: 0=Bid, 1=Ask
    taker_side SMALLINT NOT NULL,
    
    -- Fees (in quote token native units)
    taker_fee BIGINT NOT NULL,
    maker_rebate BIGINT NOT NULL,
    
    -- Solana slot and sequence number
    slot BIGINT NOT NULL,
    seq_num BIGINT NOT NULL,
    
    -- Trade timestamp
    timestamp TIMESTAMPTZ NOT NULL,
    
    -- Composite primary key for hypertable partitioning
    PRIMARY KEY (market_id, timestamp, id)
);

-- Convert to TimescaleDB hypertable for time-series optimization
-- Partitions by timestamp with 1-day chunks
SELECT create_hypertable('trades', 'timestamp', 
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Index for time-range queries per market (most common pattern)
CREATE INDEX IF NOT EXISTS idx_trades_market_time 
    ON trades(market_id, timestamp DESC);

-- Index for maker trade history
CREATE INDEX IF NOT EXISTS idx_trades_maker 
    ON trades(maker_address, timestamp DESC);

-- Index for taker trade history
CREATE INDEX IF NOT EXISTS idx_trades_taker 
    ON trades(taker_address, timestamp DESC);

-- Index for slot-based queries (for indexer catchup)
CREATE INDEX IF NOT EXISTS idx_trades_slot 
    ON trades(slot DESC);

-- Unique constraint on sequence number per market
CREATE UNIQUE INDEX IF NOT EXISTS idx_trades_market_seq 
    ON trades(market_id, seq_num);

COMMENT ON TABLE trades IS 'Executed trades stored as TimescaleDB hypertable for efficient time-series queries';
COMMENT ON COLUMN trades.maker_order_id IS 'Order ID of the maker (resting order)';
COMMENT ON COLUMN trades.taker_order_id IS 'Order ID of the taker (aggressing order)';
COMMENT ON COLUMN trades.price IS 'Execution price in quote lots per base lot';
COMMENT ON COLUMN trades.quantity IS 'Executed quantity in base lots';
COMMENT ON COLUMN trades.taker_side IS 'Side of the taker: 0=Bid (buy), 1=Ask (sell)';
COMMENT ON COLUMN trades.taker_fee IS 'Fee paid by taker in quote token native units';
COMMENT ON COLUMN trades.maker_rebate IS 'Rebate received by maker in quote token native units';
COMMENT ON COLUMN trades.slot IS 'Solana slot when trade was executed';
COMMENT ON COLUMN trades.seq_num IS 'Event queue sequence number for deduplication';
