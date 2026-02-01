-- Migration: Create balances table
-- Description: Stores user balance snapshots per market

CREATE TABLE IF NOT EXISTS balances (
    id BIGSERIAL PRIMARY KEY,
    market_id INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    
    -- Owner address
    owner VARCHAR(44) NOT NULL,
    
    -- Base token balances (in native units)
    base_free BIGINT NOT NULL DEFAULT 0,
    base_locked BIGINT NOT NULL DEFAULT 0,
    
    -- Quote token balances (in native units)
    quote_free BIGINT NOT NULL DEFAULT 0,
    quote_locked BIGINT NOT NULL DEFAULT 0,
    
    -- Solana slot when balance was last updated
    slot BIGINT NOT NULL,
    
    -- Timestamp of last update
    updated_at TIMESTAMPTZ NOT NULL,
    
    -- Unique constraint: one balance record per user per market
    CONSTRAINT uq_balances_market_owner UNIQUE (market_id, owner)
);

-- Index for user balance lookups (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_balances_owner 
    ON balances(owner);

-- Index for market balance queries
CREATE INDEX IF NOT EXISTS idx_balances_market 
    ON balances(market_id);

-- Index for slot-based queries (for indexer catchup)
CREATE INDEX IF NOT EXISTS idx_balances_slot 
    ON balances(slot DESC);

COMMENT ON TABLE balances IS 'User balance snapshots per market (OpenOrders account state)';
COMMENT ON COLUMN balances.owner IS 'User wallet address (base58)';
COMMENT ON COLUMN balances.base_free IS 'Available base token balance in native units';
COMMENT ON COLUMN balances.base_locked IS 'Locked base token balance (in open orders) in native units';
COMMENT ON COLUMN balances.quote_free IS 'Available quote token balance in native units';
COMMENT ON COLUMN balances.quote_locked IS 'Locked quote token balance (in open orders) in native units';
COMMENT ON COLUMN balances.slot IS 'Solana slot when balance was last updated';
