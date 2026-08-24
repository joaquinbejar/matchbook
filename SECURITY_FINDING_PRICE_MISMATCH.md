# Matchbook DEX — Security Finding: Price-Mismatch Vault Drain

## Severity: HIGH (aggregate vault insolvency / unbacked credit)

## Summary

`match_orders.rs` hardcodes `match_price = ask_price` for every fill. But a
bid's quote tokens are locked at **its own price** in `place_order.rs`. When a
bid matches against a resting ask priced higher than the bid, the settlement
debits the taker at the ask price — more than was ever locked — and
`saturating_sub` in `settle_taker_bid` silently absorbs the deficit instead of
reverting. The maker is credited the full ask-price amount from the market's
pooled locks. Repeated trades make total credited quote exceed real locked
quote → the market vault becomes insolvent.

## Affected code

| Step | File | Lines |
|---|---|---|
| Lock computed at bid's own price | `program/src/instructions/place_order.rs` | `calculate_lock_amount`, Side::Bid arm |
| Match price hardcoded to ask | `program/src/instructions/match_orders.rs` | ~257 (`match_price = ask_price`) |
| Settle debits at match price | `program/src/instructions/consume_events.rs` | ~301-303 (`quote_amount`), ~372 (`saturating_sub`) |
| Maker credited full amount | `program/src/instructions/consume_events.rs` | ~452 (`settle_maker_ask`) |

## Exploit scenario

1. Attacker places bid: qty=10 lots @ price=100 → locks 1,000 quote units
2. Any resting ask exists at price=110
3. Orders cross; `match_price = ask_price = 110`
4. Settle computes `quote_amount = 10 × 110 = 1,100`
5. `settle_taker_bid`: `quote_locked.saturating_sub(1100 + fee)` → 0 (deficit of
   110+fee silently absorbed; no revert)
6. Taker receives full base; maker credited 1,100 + rebate from pooled vault
7. The 110-unit gap per trade comes out of **other users' locks**

## Proof-of-concept

See `program/tests/poc_price_mismatch.rs` (this branch). Both tests pass:

```
running 2 tests
test test_aggregate_vault_insolvency_accumulates ... ok
test test_saturating_sub_hides_deficit ... ok
```

- `test_saturating_sub_hides_deficit`: reproduces the exact settle math
  (locked=1000, settled=1100, fee=10 → remaining=0 without reverting)
- `test_aggregate_vault_insolvency_accumulates`: simulates 1,000 such trades;
  asserts `total_credit_issued > vault_backing` (insolvency) after the loop

## Proposed fix (two layers)

1. **Settle must never exceed lock**: in `settle_taker_bid`, use
   `checked_sub(quote_amount.saturating_add(taker_fee))` with an explicit
   `InsufficientLock` error instead of `saturating_sub`.
2. **Match price should be the maker's price**: replace
   `match_price = ask_price` with the resting (older) order's side — bid price
   when the bid is the older order, per standard CLOB price-time priority.
   This also fixes taker-side fee attribution (the fill event currently always
   marks `Side::Ask` as taker).

## Impact

Any market on Matchbook with crossing orders where the resting ask is priced
above an aggressive bid accumulates unbacked quote credit. Over time the
market vault cannot cover withdrawals — aggregate insolvency funded by
silent dilution of all depositors' locks.
