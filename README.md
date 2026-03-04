# Aave V3 Tracker

Indexes Aave V3 protocol data on Ethereum to PostgreSQL with replay-safe event processing. Tracks reserve configs and interest metrics.

## Architecture

### Planned Architecture

![AAVE V3 Tracker Architecture](./docs/architecture-planned.png)

### Current Architecture

![AAVE V3 Tracker Current Architecture](./docs/current-architecture.png)

## Features

### Implemented

- Adaptive backfill with checkpoint recovery
- Multicall batching for reduced RPC usage
- Event deduplication via tx hash + log index
- Transaction-wrapped writes for atomicity
- Exponential backoff with jitter on transient errors
- Multi-RPC provider with automatic failover on provider errors

### In Progress

- Tracking user positions (supply, borrow, collateral state per asset)

### Planned

- Fetching asset prices
- Tracking protocol contract address changes via address provider events
- Calculating users' health factors

## Known Issues

- Reorg handling is not implemented. To avoid inconsistencies, WebSocket indexing is disabled and events are written to the database via backfill only, with a ~20 block delay. As a result, indexed data is not real-time and may arrive with latency.
- It has been noticed that the Subgraph data used to bootstrap user positions may contain inaccuracies. Since the Subgraph is an external data source, it can have its own indexing issues or rounding differences. Any inaccuracy in the initial bootstrap state propagates into all subsequent position updates for that user.
- Scaled balance computations may drift by ±1 wei per event due to `rayDiv`/`rayMul` rounding in Aave's fixed-point arithmetic. This error may accumulate for users with many transactions and is a protocol-level property, not a bug in this indexer — the discrepancy is visible on-chain even between `Supply` and `Mint` events within the same transaction (e.g. [this example](https://etherscan.io/tx/0x5e9eb74f9f6130d951053c5a7fefdae88b229d28e0a53c3604fff66124319e91#eventlog)).

## Database Schema

- **reserves** - Asset-level configuration and risk parameters (mostly static, but updatable via governance)
- **reserve_state** - Latest on-chain reserve state derived from events (rates, indices, liquidity, debt, treasury accruals)
- **user_positions** - Per-user, per-asset position state (scaled aToken balance, scaled variable debt, collateral flag)
- **processed_events** - Deduplication log tracking processed tx hash + log index pairs
- **sync_status** - Backfill checkpoint storing last processed block number
- **bootstrap_state** - Subgraph bootstrap progress: cursor, meta block, and completion flag

## Tracked Events

### Pool

- **ReserveInitialized** - New asset added to the protocol
- **ReserveDataUpdated** - Interest rates and indices changed
- **ReserveUsedAsCollateralEnabled** - User enabled an asset as collateral
- **ReserveUsedAsCollateralDisabled** - User disabled an asset as collateral

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
- **EModeAssetCategoryChanged** - Efficiency mode category changed
- **SiloedBorrowingChanged** - Siloed borrowing status changed
- **UnbackedMintCapChanged** - Unbacked mint cap updated

### Token Contracts (aToken / Variable Debt Token)

- **Mint** - Tokens minted on supply or borrow increase (aToken and variable debt token)
- **Burn** - Tokens burned on withdrawal or repayment (aToken and variable debt token)
- **BalanceTransfer** - Scaled aToken balance transfer between users (aToken only)
