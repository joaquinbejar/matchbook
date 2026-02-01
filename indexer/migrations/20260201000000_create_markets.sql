-- Migration: Create markets table
-- Description: Stores market metadata including addresses, lot sizes, and fees

CREATE TABLE IF NOT EXISTS markets (
    id SERIAL PRIMARY KEY,
    
    -- Solana addresses (base58 encoded, max 44 chars)
    address VARCHAR(44) UNIQUE NOT NULL,
    base_mint VARCHAR(44) NOT NULL,
    quote_mint VARCHAR(44) NOT NULL,
    
    -- Token decimals
    base_decimals SMALLINT NOT NULL,
    quote_decimals SMALLINT NOT NULL,
    
    -- Lot sizes (in native token units)
    base_lot_size BIGINT NOT NULL,
    quote_lot_size BIGINT NOT NULL,
    
    -- Trading parameters
    tick_size BIGINT NOT NULL,
    min_order_size BIGINT NOT NULL,
    
    -- Fees in basis points
    taker_fee_bps SMALLINT NOT NULL,
    maker_fee_bps SMALLINT NOT NULL,  -- Can be negative for rebates
    
    -- Market status: 0=Inactive, 1=Active, 2=Paused, 3=Closed
    status SMALLINT NOT NULL DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for address lookups (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_markets_address ON markets(address);

-- Index for finding markets by token pair
CREATE INDEX IF NOT EXISTS idx_markets_mints ON markets(base_mint, quote_mint);

-- Index for filtering by status
CREATE INDEX IF NOT EXISTS idx_markets_status ON markets(status);

COMMENT ON TABLE markets IS 'Market metadata including addresses, lot sizes, and fee configuration';
COMMENT ON COLUMN markets.address IS 'Solana market account address (base58)';
COMMENT ON COLUMN markets.base_lot_size IS 'Minimum base token increment in native units';
COMMENT ON COLUMN markets.quote_lot_size IS 'Minimum quote token increment in native units';
COMMENT ON COLUMN markets.tick_size IS 'Minimum price increment in quote lots';
COMMENT ON COLUMN markets.taker_fee_bps IS 'Taker fee in basis points (1 bps = 0.01%)';
COMMENT ON COLUMN markets.maker_fee_bps IS 'Maker fee in basis points (negative = rebate)';
COMMENT ON COLUMN markets.status IS 'Market status: 0=Inactive, 1=Active, 2=Paused, 3=Closed';
