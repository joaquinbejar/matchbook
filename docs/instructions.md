# Instruction reference

Program id: `MbKAoWL85AKTWWBvf5FbLJZFKQuBcH1DaWQdfnkJxTw` (set by `scripts/deploy.sh` when
you deploy with your own keypair).

The SDKs build every instruction below; this page is for readers of the wire format and for
other-language clients.

## Encoding

Every instruction starts with an 8-byte header:

| Byte | Field |
| --- | --- |
| 0 | Discriminator |
| 1, 2, 3 | Instruction-specific values `a`, `b`, `c` |
| 4–7 | `hint`, little-endian `u32` |

For instructions a trader signs, `hint` is the trader's seat index: the node index of its
seat, which the SDKs read from the market account. A wrong value still works (the program
falls back to a lookup by owner), but the right one saves compute. `u32::MAX` means "no hint".

All integers are little-endian and every payload field is naturally aligned. Prices are in
quote lots per base lot and sizes in base lots, so a price `p` for `n` lots moves
`p × n × quote_lot_size` quote atoms and `n × base_lot_size` base atoms.

Accounts are listed in order. `s` marks a signer and `w` a writable account.

## Market setup

### 0 · CreateMarket

| Accounts | payer (s, w), market (s, w), base mint, quote mint, base vault (w), quote vault (w), vault authority, base token program, quote token program, associated token program, system program |
| --- | --- |
| Payload | `CreateMarketArgs`, 72 bytes |

`CreateMarketArgs`: `base_lot_size u64`, `quote_lot_size u64`, `min_base_lots u64`,
`taker_fee_bps u32`, `maker_rebate_bps u32`, `initial_nodes u32`, `referral_share_bps u16`,
`flags u16`, `authority [u8; 32]`.

The market is a fresh keypair created here with `initial_nodes` nodes, up to 10 KiB. For a
larger initial size, create the account yourself (owned by the program, zeroed) in an
earlier instruction. The vaults are the vault authority's associated token accounts and are
created here. The taker fee is at most 1,000 bps; the maker rebate plus the referral share of
the fee must fit in the fee. `flags` bit 0 makes the market permissioned.

### 11 · CollectFees

| Accounts | authority (s), market (w), destination (w), quote vault (w), quote mint, vault authority, quote token program |
| --- | --- |

Sends the accrued fees to `destination`.

### 12 · SetStatus

| Accounts | authority (s), market (w) |
| --- | --- |
| Header | `a`: 0 active, 1 cancel-only, 2 closed |

Cancel-only allows cancels, withdrawals and fee collection. Closed allows withdrawals only
and requires an empty book with no stops.

## Seats and balances

### 1 · ClaimSeat

| Accounts | payer (s, w), owner (s), market (w), system program, and on a permissioned market the market authority (s) |
| --- | --- |

Creates the owner's seat. The payer funds the node's rent if the market has to grow.

### 2 · Deposit

| Accounts | owner (s), market (w), user token account (w), vault (w), mint, token program |
| --- | --- |
| Payload | `amount u64` |

### 3 · Withdraw

| Accounts | owner (s), market (w), user token account (w), vault (w), mint, vault authority, token program |
| --- | --- |
| Payload | `amount u64` |

Only the owner can withdraw, never a delegate.

### 14 · SetDelegate

| Accounts | owner (s, w), market (w), system program |
| --- | --- |
| Payload | `delegate [u8; 32]`, `expires_at u64` (0 = never), `permissions u64` |

Permissions: bit 0 trade (place, reduce, cancel, batch), bit 1 cancel and reduce only. A
seat can have 8 delegates. Granting an existing delegate again updates it.

### 15 · RemoveDelegate

| Accounts | signer (s), market (w) |
| --- | --- |
| Payload | `delegate [u8; 32]` |

Signed by the seat's owner or by the delegate itself.

## Orders

Trader instructions below are signed by the seat's owner or by a delegate with the right
permission.

### 4 · PlaceOrder

| Accounts | owner or delegate (s, w), market (w), system program, optionally a referrer (an account with a seat on this market) |
| --- | --- |
| Payload | `OrderArgs`, 56 bytes |
| Return data | Order id `u64`, 0 if nothing rested |

`OrderArgs`:

| Offset | Field | Notes |
| --- | --- | --- |
| 0 | `side u8` | 0 bid, 1 ask |
| 1 | `order_type u8` | 0 limit, 1 post-only, 2 post-only slide, 3 immediate or cancel, 4 fill or kill |
| 2 | `self_trade u8` | 0 cancel the resting order, 1 decrement both, 2 abort |
| 3 | `reserved u8` | 0 |
| 4 | `match_limit u32` | Resting orders to touch at most; 0 means 64 |
| 8 | `price u64` | |
| 16 | `base_lots u64` | |
| 24 | `client_order_id u64` | Echoed in events, usable to cancel |
| 32 | `expires_at u64` | Unix time; 0 = never |
| 40 | `quote_budget u64` | Immediate-or-cancel bids only: most quote atoms to spend, fees included; 0 = none |
| 48 | `reverse_spread u32` | Parts per million; 0 = an ordinary order. Resting order types only |
| 52 | `referrer_seat u32` | Seat hint of the referrer account, if one is passed |

A referrer receives `referral_share_bps` of the taker fee this order pays.

### 5 · CancelOrder

| Accounts | signer (s), market (w) |
| --- | --- |
| Payload | `order_id u64` |

Cancels an order or a stop.

### 6 · CancelAll

| Accounts | signer (s), market (w) |
| --- | --- |
| Header | `a`: 0 bids, 1 asks, 2 both |

### 7 · CancelByClientId

| Accounts | signer (s), market (w) |
| --- | --- |
| Payload | `client_order_id u64` |

Cancels every order and stop of the seat with this client id.

### 8 · ReduceOrder

| Accounts | signer (s), market (w) |
| --- | --- |
| Payload | `order_id u64`, `new_base_lots u64` |

Shrinks an order in place. It keeps its place in the queue.

### 9 · BatchUpdate

| Accounts | owner or delegate (s, w), market (w), system program |
| --- | --- |
| Header | `a`: cancel all first (0 none, 1 bids, 2 asks, 3 both), `b`: number of cancels, `c`: number of orders |
| Payload | `b` order ids (`u64` each), then `c` × `OrderArgs` |
| Return data | One order id `u64` per order, 0 where nothing rested |

At most 32 cancels and 32 orders. Cancels of orders that are already gone are skipped, so a
market maker can requote without first checking what filled.

### 16 · PlaceStop

| Accounts | owner or delegate (s, w), market (w), system program |
| --- | --- |
| Header | `a`: 0 fires at or above the trigger, 1 at or below |
| Payload | `StopArgs`, 72 bytes: `order OrderArgs`, `trigger_price u64`, `oco_with u64` |
| Return data | Stop id `u64` |

The order may not carry a quote budget or a reverse spread, and its match limit is capped at
16. `oco_with` pairs the new stop with one of the seat's stops as one-cancels-other (0 for
none). A trigger the last trade already reached is rejected with `StopWouldTrigger`.

### 17 · TriggerStops

| Accounts | market (w) |
| --- | --- |
| Header | `hint`: most stops to place |

Permissionless. Every trade already places up to two triggered stops; this places the rest.

### 13 · PruneExpired

| Accounts | market (w) |
| --- | --- |
| Header | `hint`: most orders to remove |

Permissionless. Matching skips expired orders anyway; pruning frees their nodes and funds.

## Swaps

### 10 · Swap

| Accounts | user (s), market (w), user base token account (w), user quote token account (w), base vault (w), quote vault (w), base mint, quote mint, vault authority, base token program, quote token program, optionally a referrer with a seat |
| --- | --- |
| Header | `a`: 0 buy base, 1 sell base |
| Payload | `base_lots u64`, `quote_budget u64`, `limit_price u64`, `min_out u64`, `match_limit u32`, `referrer_seat u32` |

Trades from the user's token accounts with no seat. For a buy, `base_lots` is the most to buy
and `quote_budget` the most quote atoms to spend including fees; for a sell, `base_lots` is the
amount to sell. The swap fails with `SlippageExceeded` if the user receives less than
`min_out` atoms, measured on the user's token account after any Token-2022 transfer fee. Disabled on permissioned markets.

## Errors

Custom error codes. Engine errors come first:

| Code | Name | Code | Name |
| ---: | --- | ---: | --- |
| 0 | InvalidAccountData | 14 | Overflow |
| 1 | AlreadyInitialized | 15 | MarketNotActive |
| 2 | InvalidMarketParams | 16 | BudgetRequiresIoc |
| 3 | OutOfSpace | 17 | AlreadyExpired |
| 4 | TraderNotFound | 18 | Corrupted |
| 5 | SeatAlreadyClaimed | 19 | DelegateNotFound |
| 6 | SeatNotEmpty | 20 | TooManyDelegates |
| 7 | InvalidPrice | 21 | NotPermitted |
| 8 | InvalidQuantity | 22 | DelegateExpired |
| 9 | InsufficientFunds | 23 | InvalidDelegate |
| 10 | PostOnlyWouldCross | 24 | InvalidReverseSpread |
| 11 | FillOrKillNotFilled | 25 | InvalidStop |
| 12 | SelfTrade | 26 | StopWouldTrigger |
| 13 | OrderNotFound | 27 | Permissioned |

Program errors:

| Code | Name | Code | Name |
| ---: | --- | ---: | --- |
| 100 | InvalidInstruction | 106 | InvalidMint |
| 101 | InvalidInstructionData | 107 | SlippageExceeded |
| 102 | InvalidAccount | 108 | TransferAmountMismatch |
| 103 | MissingSignature | 109 | NotAuthority |
| 104 | InvalidMarketOwner | 110 | TooManyItems |
| 105 | SameMints | | |
