use alloy_sol_types::sol;

sol! {
    event ReserveStableRateBorrowing(
        address indexed asset,
        bool enabled
    );

    event ReserveDropped(
        address indexed asset
    );

    event ReserveFactorChanged(
        address indexed asset,
        uint256 oldReserveFactor,
        uint256 newReserveFactor
    );

    event ReserveInterestRateStrategyChanged(
        address indexed asset,
        address oldStrategy,
        address newStrategy
    );

    event CollateralConfigurationChanged(
        address indexed asset,
        uint256 ltv,
        uint256 liquidationThreshold,
        uint256 liquidationBonus
    );

    event ReserveFrozen(
        address indexed asset
    );

    event ReserveUnfrozen(
        address indexed asset
    );

    event ReservePaused(
        address indexed asset,
        bool paused
    );

    event ReserveBorrowing(
        address indexed asset,
        bool enabled
    );

    event ReserveActive(
        address indexed asset
    );

    event MarketIdSet(
        string indexed oldMarketId,
        string indexed newMarketId
    );

    event BorrowCapChanged(
        address indexed asset,
        uint256 oldBorrowCap,
        uint256 newBorrowCap
    );

    event SupplyCapChanged(
        address indexed asset,
        uint256 oldSupplyCap,
        uint256 newSupplyCap
    );

    event LiquidationProtocolFeeChanged(
        address indexed asset,
        uint256 oldFee,
        uint256 newFee
    );

    event DebtCeilingChanged(
        address indexed asset,
        uint256 oldDebtCeiling,
        uint256 newDebtCeiling
    );

    event ReserveFlashLoaning(
        address indexed asset,
        bool enabled
    );

    event EModeAssetCategoryChanged(
        address indexed asset,
        uint8 oldCategoryId,
        uint8 newCategoryId
    );

    event SiloedBorrowingChanged(
        address indexed asset,
        bool oldState,
        bool newState
    );

    event UnbackedMintCapChanged(
        address indexed asset,
        uint256 oldUnbackedMintCap,
        uint256 newUnbackedMintCap
    );
}
