-- Migration: Create orders table
-- Description: Stores order history including status and fill information

CREATE TABLE IF NOT EXISTS orders (
    id BIGSERIAL PRIMARY KEY,
    market_id INTEGER NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    
    -- Order ID (u128 requires NUMERIC(39) for full precision)
    order_id NUMERIC(39) NOT NULL,
    
    -- Owner address
    owner VARCHAR(44) NOT NULL,
    
    -- Order side: 0=Bid, 1=Ask
    side SMALLINT NOT NULL,
    
    -- Price and quantity (in native units)
    price BIGINT NOT NULL,
    original_quantity BIGINT NOT NULL,
    filled_quantity BIGINT NOT NULL DEFAULT 0,
    
    -- Order status: 0=Open, 1=PartiallyFilled, 2=Filled, 3=Cancelled, 4=Expired
    status SMALLINT NOT NULL DEFAULT 0,
    
    -- Order type: 0=Limit, 1=PostOnly, 2=IOC, 3=FOK
    order_type SMALLINT NOT NULL DEFAULT 0,
    
    -- Time in force: 0=GTC, 1=IOC, 2=FOK, 3=PostOnly
    time_in_force SMALLINT NOT NULL DEFAULT 0,
    
    -- Client-provided order ID (optional)
    client_order_id BIGINT,
    
    -- Solana slot when order was placed
    slot BIGINT NOT NULL,
    
    -- Timestamps
    placed_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Index for user order history (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_orders_owner 
    ON orders(owner, placed_at DESC);

-- Index for market order history
CREATE INDEX IF NOT EXISTS idx_orders_market 
    ON orders(market_id, placed_at DESC);

-- Index for order lookup by ID
CREATE INDEX IF NOT EXISTS idx_orders_order_id 
    ON orders(market_id, order_id);

-- Index for filtering by status
CREATE INDEX IF NOT EXISTS idx_orders_status 
    ON orders(status, placed_at DESC);

-- Index for client order ID lookups
CREATE INDEX IF NOT EXISTS idx_orders_client_id 
    ON orders(owner, client_order_id) 
    WHERE client_order_id IS NOT NULL;

COMMENT ON TABLE orders IS 'Order history including placement, fills, and cancellations';
COMMENT ON COLUMN orders.order_id IS 'On-chain order ID (u128)';
COMMENT ON COLUMN orders.side IS 'Order side: 0=Bid (buy), 1=Ask (sell)';
COMMENT ON COLUMN orders.price IS 'Limit price in quote lots per base lot';
COMMENT ON COLUMN orders.original_quantity IS 'Original order quantity in base lots';
COMMENT ON COLUMN orders.filled_quantity IS 'Cumulative filled quantity in base lots';
COMMENT ON COLUMN orders.status IS 'Order status: 0=Open, 1=PartiallyFilled, 2=Filled, 3=Cancelled, 4=Expired';
COMMENT ON COLUMN orders.order_type IS 'Order type: 0=Limit, 1=PostOnly, 2=IOC, 3=FOK';
COMMENT ON COLUMN orders.time_in_force IS 'Time in force: 0=GTC, 1=IOC, 2=FOK, 3=PostOnly';
COMMENT ON COLUMN orders.client_order_id IS 'Optional client-provided order ID for tracking';
COMMENT ON COLUMN orders.slot IS 'Solana slot when order was placed';
