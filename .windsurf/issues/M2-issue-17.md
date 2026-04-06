# M2 Issue #17: Database schema: PostgreSQL + TimescaleDB migrations

## Overview

Design and implement the database schema for storing market data, trades, and user information. Uses PostgreSQL with TimescaleDB extension for time-series data (trades, candles).

## Acceptance Criteria

- [ ] PostgreSQL schema with tables:
  - `markets`: market metadata (address, base/quote mints, lot sizes, fees, status)
  - `orders`: order history (order_id, market, owner, side, price, quantity, status, timestamps)
  - `trades`: executed trades (trade_id, market, price, quantity, maker/taker, fees, timestamp)
  - `candles`: OHLCV data (market, interval, open, high, low, close, volume, timestamp)
  - `balances`: user balance snapshots (market, owner, base_free, base_locked, quote_free, quote_locked)
- [ ] TimescaleDB hypertables for `trades` and `candles`
- [ ] Indexes for common query patterns (market lookups, time ranges, user queries)
- [ ] Migration scripts using sqlx
- [ ] Seed data for testing
- [ ] Documentation for schema design decisions

## Technical Approach

### 1. Project Structure

Create a new `indexer` crate for off-chain services:

```
indexer/
├── Cargo.toml
├── migrations/
│   ├── 20260201000000_create_markets.sql
│   ├── 20260201000001_create_trades.sql
│   ├── 20260201000002_create_orders.sql
│   ├── 20260201000003_create_balances.sql
│   └── 20260201000004_create_candles.sql
├── src/
│   ├── lib.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   └── schema.rs
│   └── seed.rs
└── tests/
    └── schema_tests.rs
```

### 2. Database Tables

#### markets
- `id`: SERIAL PRIMARY KEY
- `address`: VARCHAR(44) UNIQUE NOT NULL (Solana pubkey)
- `base_mint`: VARCHAR(44) NOT NULL
- `quote_mint`: VARCHAR(44) NOT NULL
- `base_decimals`: SMALLINT NOT NULL
- `quote_decimals`: SMALLINT NOT NULL
- `base_lot_size`: BIGINT NOT NULL
- `quote_lot_size`: BIGINT NOT NULL
- `tick_size`: BIGINT NOT NULL
- `min_order_size`: BIGINT NOT NULL
- `taker_fee_bps`: SMALLINT NOT NULL
- `maker_fee_bps`: SMALLINT NOT NULL
- `status`: SMALLINT NOT NULL DEFAULT 0
- `created_at`: TIMESTAMPTZ DEFAULT NOW()
- `updated_at`: TIMESTAMPTZ DEFAULT NOW()

#### trades (TimescaleDB hypertable)
- `id`: BIGSERIAL
- `market_id`: INTEGER REFERENCES markets(id)
- `maker_order_id`: NUMERIC(39) NOT NULL
- `taker_order_id`: NUMERIC(39) NOT NULL
- `maker_address`: VARCHAR(44) NOT NULL
- `taker_address`: VARCHAR(44) NOT NULL
- `price`: BIGINT NOT NULL
- `quantity`: BIGINT NOT NULL
- `taker_side`: SMALLINT NOT NULL
- `taker_fee`: BIGINT NOT NULL
- `maker_rebate`: BIGINT NOT NULL
- `slot`: BIGINT NOT NULL
- `seq_num`: BIGINT NOT NULL
- `timestamp`: TIMESTAMPTZ NOT NULL
- PRIMARY KEY (market_id, timestamp, id)

#### orders
- `id`: BIGSERIAL PRIMARY KEY
- `market_id`: INTEGER REFERENCES markets(id)
- `order_id`: NUMERIC(39) NOT NULL
- `owner`: VARCHAR(44) NOT NULL
- `side`: SMALLINT NOT NULL
- `price`: BIGINT NOT NULL
- `original_quantity`: BIGINT NOT NULL
- `filled_quantity`: BIGINT NOT NULL DEFAULT 0
- `status`: SMALLINT NOT NULL DEFAULT 0
- `order_type`: SMALLINT NOT NULL DEFAULT 0
- `time_in_force`: SMALLINT NOT NULL DEFAULT 0
- `client_order_id`: BIGINT
- `slot`: BIGINT NOT NULL
- `placed_at`: TIMESTAMPTZ NOT NULL
- `updated_at`: TIMESTAMPTZ NOT NULL

#### balances
- `id`: BIGSERIAL PRIMARY KEY
- `market_id`: INTEGER REFERENCES markets(id)
- `owner`: VARCHAR(44) NOT NULL
- `base_free`: BIGINT NOT NULL DEFAULT 0
- `base_locked`: BIGINT NOT NULL DEFAULT 0
- `quote_free`: BIGINT NOT NULL DEFAULT 0
- `quote_locked`: BIGINT NOT NULL DEFAULT 0
- `slot`: BIGINT NOT NULL
- `updated_at`: TIMESTAMPTZ NOT NULL
- UNIQUE (market_id, owner)

#### candles (TimescaleDB continuous aggregate)
- Created as materialized view from trades
- Intervals: 1m, 5m, 15m, 1h, 4h, 1d

### 3. Indexes

- `markets`: address, (base_mint, quote_mint)
- `trades`: (market_id, timestamp), (maker_address, timestamp), (taker_address, timestamp)
- `orders`: (owner, placed_at), (market_id, placed_at), (market_id, order_id)
- `balances`: (market_id, owner)

### 4. Migration Tool

Use `sqlx` for migrations with compile-time checked queries.

## Files to Create

| File | Description |
|------|-------------|
| `indexer/Cargo.toml` | Crate manifest with sqlx, tokio dependencies |
| `indexer/migrations/*.sql` | SQL migration files |
| `indexer/src/lib.rs` | Crate root |
| `indexer/src/db/mod.rs` | Database module |
| `indexer/src/db/models.rs` | Rust types for database rows |
| `indexer/src/db/schema.rs` | Schema documentation |
| `indexer/src/seed.rs` | Seed data for testing |

## Notes

- Use NUMERIC(39) for order IDs (u128 = 39 decimal digits)
- Use BIGINT for prices and quantities (u64)
- Use SMALLINT for enums (side, status, order_type)
- TimescaleDB hypertables for trades enable efficient time-range queries
- Continuous aggregates for candles reduce query load
- Consider retention policies for old data (configurable)
