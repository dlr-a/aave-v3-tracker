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

### Planned

- Fetching asset prices
- Tracking protocol contract address changes via address provider events
- Tracking user positions
- Calculating users' health factors

## Known Issues

- Reorg handling is not implemented. To avoid inconsistencies, WebSocket indexing is disabled and events are written to the database via backfill only, with a ~20 block delay. As a result, indexed data is not real-time and may arrive with latency.
- RPC rate limits may cause indexing delays or failures. The indexer currently relies on a single RPC endpoint; while exponential backoff is implemented, it may not be sufficient under heavy load or strict provider limits.

## Database Schema

- **reserves** - Asset-level configuration and risk parameters (mostly static, but updatable via governance)
- **reserve_state** - Latest on-chain reserve state derived from events (rates, indices, liquidity, debt, treasury accruals)
- **processed_events** - Deduplication log tracking processed tx hash + log index pairs
- **sync_status** - Backfill checkpoint storing last processed block number

## Tracked Events

### Pool

- **ReserveInitialized** - New asset added to the protocol
- **ReserveDataUpdated** - Interest rates and indices changed

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
