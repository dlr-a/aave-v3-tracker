# Aave V3 Tracker

An Ethereum event indexer for the Aave V3 core Ethereum market that stores protocol state in PostgreSQL. Tracks reserve configurations, interest rate state, user supply/borrow positions, and eMode categories.

## Tech Stack

- **Rust** (edition 2024) — async runtime via [tokio](https://tokio.rs)
- **PostgreSQL** — primary data store
- **Diesel** (async)
- **[alloy](https://github.com/alloy-rs/alloy)** — Ethereum provider, ABI decoding, multicall
- **testcontainers** — real PostgreSQL instances spun up per test

## How to Run

**Prerequisites:**
- [Rust](https://rustup.rs)
- PostgreSQL (running locally)
- `diesel_cli`: `cargo install diesel_cli --no-default-features --features postgres`
- Docker (required for tests)

**1. Configure environment**

Create a `.env` file at the project root (`.env` is gitignored):

```
# PostgreSQL connection string
DATABASE_URL=postgres://user:password@localhost:5432/aave_v3_tracker

# Comma-separated list of HTTP RPC endpoints, tried in order on failure
HTTP_RPC_URLS=https://eth-mainnet.g.alchemy.com/v2/<key>,https://eth.llamarpc.com

# Required for historical user position data — bootstraps positions from The Graph before
# event replay. Without this, position tracking starts from the current block only.
SUBGRAPH_API_KEY=<key>
```

The PostgreSQL user must have `CREATEDB` privilege for `diesel setup` to create the database. On a fresh local install this is usually the case for the default superuser.

**2. Create database and run migrations**

```bash
diesel setup
```

**3. Start the indexer**

```bash
cargo run
```

On first run with an empty database, the indexer fetches the current reserve snapshot via multicall before starting the backfill loop. If `SUBGRAPH_API_KEY` is set, it also bootstraps historical user positions from The Graph before beginning event replay.

**4. Run tests**

```bash
cargo test
```

Tests require Docker to be **installed and running** — testcontainers spins up a real PostgreSQL container for each test. If Docker is not running, tests will fail.

## Architecture

### Current Architecture

![AAVE V3 Tracker Current Architecture](./docs/current-architecture.png)

### Planned Architecture

![AAVE V3 Tracker Architecture](./docs/architecture-planned.png)

## Features

### Implemented

- Adaptive backfill with checkpoint recovery
- Multicall batching for reduced RPC usage
- Event deduplication via tx hash + log index
- Transaction-wrapped writes for atomicity
- Exponential backoff with jitter on transient errors
- Multi-RPC provider with automatic failover on provider errors
- Tracking user positions (supply, borrow, collateral state per asset)
- Tracking user eMode category assignments
- Tracking eMode category definitions (LTV, liquidation parameters, asset bitmaps)

### Planned

- Tracking total supply per reserve (aggregated from aToken Mint/Burn events)
- Fetching asset prices
- Tracking protocol contract address changes via address provider events
- Calculating users' health factors

## Known Issues

- Reorg handling is not implemented. To avoid inconsistencies, WebSocket indexing is disabled and events are written to the database via backfill only, with a ~20 block delay. As a result, indexed data is not real-time and may arrive with latency.
- It has been noticed that the Subgraph data used to bootstrap user positions may contain inaccuracies. Since the Subgraph is an external data source, it can have its own indexing issues or rounding differences. Any inaccuracy in the initial bootstrap state propagates into all subsequent position updates for that user.
- Scaled balance computations may drift by ±1 wei per event due to `rayDiv`/`rayMul` rounding in Aave's fixed-point arithmetic. This error may accumulate for users with many transactions and is a protocol-level property, not a bug in this indexer — the discrepancy is visible on-chain even between `Supply` and `Mint` events within the same transaction (e.g. [this example](https://etherscan.io/tx/0x5e9eb74f9f6130d951053c5a7fefdae88b229d28e0a53c3604fff66124319e91#eventlog)).
- Aave V3 Pool and PoolConfigurator addresses are hardcoded. If the protocol upgrades and deploys new contract addresses via the `PoolAddressesProvider`, the indexer will not detect the change and will silently stop receiving events from the new contracts. Tracking address provider events is listed as a planned feature.

## Database Schema

- **reserves** - Asset-level configuration and risk parameters (mostly static, but updatable via governance)
- **reserve_state** - Latest on-chain reserve state derived from events (rates, indices, liquidity, debt, treasury accruals)
- **user_positions** - Per-user, per-asset position state (scaled aToken balance, scaled variable debt, collateral flag)
- **emode_categories** - Efficiency mode category definitions (LTV, liquidation threshold/bonus, collateral/borrowable/ltvzero bitmaps, label)
- **user_emode** - Per-user active eMode category (category 0 = no eMode)
- **processed_events** - Deduplication log tracking processed tx hash + log index pairs
- **sync_status** - Backfill checkpoint storing last processed block number
- **bootstrap_state** - Subgraph bootstrap progress: cursor, meta block, and completion flag

## Tracked Events

### Pool

- **ReserveInitialized** - New asset added to the protocol
- **ReserveDataUpdated** - Interest rates and indices changed
- **ReserveUsedAsCollateralEnabled** - User enabled an asset as collateral
- **ReserveUsedAsCollateralDisabled** - User disabled an asset as collateral
- **UserEModeSet** - User switched to a different eMode category (0 = disabled)

### PoolConfigurator

- **CollateralConfigurationChanged** - LTV, liquidation threshold/bonus updated
- **ReserveFrozen / ReserveUnfrozen** - Asset freeze status changed
- **ReservePaused** - Asset paused or unpaused
- **ReserveBorrowing** - Borrowing enabled/disabled
- **ReserveStableRateBorrowing** - Stable rate borrowing toggled
- **ReserveActive** - Asset activated
- **ReserveDropped** - Asset removed from protocol
- **ReserveFactorChanged** - Protocol fee percentage updated
- **ReserveInterestRateStrategyChanged** - Interest rate model changed
- **SupplyCapChanged / BorrowCapChanged** - Supply/borrow limits updated
- **DebtCeilingChanged** - Isolation mode debt ceiling updated
- **LiquidationProtocolFeeChanged** - Liquidation fee updated
- **ReserveFlashLoaning** - Flash loan availability toggled
- **EModeCategoryAdded** - New efficiency mode category created or updated
- **AssetCollateralInEModeChanged** - Asset collateral eligibility within an eMode category changed
- **AssetBorrowableInEModeChanged** - Asset borrowability within an eMode category changed
- **AssetLtvzeroInEModeChanged** - Asset zero-LTV flag within an eMode category changed
- **SiloedBorrowingChanged** - Siloed borrowing status changed
- **UnbackedMintCapChanged** - Unbacked mint cap updated

### Token Contracts (aToken / Variable Debt Token)

- **Mint** - Tokens minted on supply or borrow increase (aToken and variable debt token)
- **Burn** - Tokens burned on withdrawal or repayment (aToken and variable debt token)
- **BalanceTransfer** - Scaled aToken balance transfer between users (aToken only)
