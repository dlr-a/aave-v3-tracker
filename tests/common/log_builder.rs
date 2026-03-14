use alloy::primitives::{Address, B256, U256};
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;

use aave_v3_tracker::abi::{
    AssetBorrowableInEModeChanged, AssetCollateralInEModeChanged, AssetLtvzeroInEModeChanged,
    BorrowCapChanged, CollateralConfigurationChanged, DebtCeilingChanged, EModeCategoryAdded,
    LiquidationProtocolFeeChanged, ReserveActive, ReserveBorrowing, ReserveDataUpdated,
    ReserveDropped, ReserveFactorChanged, ReserveFlashLoaning, ReserveFrozen,
    ReserveInterestRateStrategyChanged, ReservePaused, ReserveUnfrozen, SiloedBorrowingChanged,
    SupplyCapChanged, UnbackedMintCapChanged, UserEModeSet,
};

pub struct LogBuilder {
    block_number: u64,
    log_index: u64,
    tx_hash: B256,
}

impl LogBuilder {
    pub fn new() -> Self {
        Self {
            block_number: 100,
            log_index: 0,
            tx_hash: B256::repeat_byte(0x01),
        }
    }

    pub fn at_block(mut self, block: u64) -> Self {
        self.block_number = block;
        self
    }

    pub fn log_index(mut self, idx: u64) -> Self {
        self.log_index = idx;
        self
    }

    pub fn tx_hash(mut self, hash: B256) -> Self {
        self.tx_hash = hash;
        self
    }

    fn wrap_log_data(&self, address: Address, data: alloy::primitives::LogData) -> Log {
        Log {
            inner: alloy::primitives::Log { address, data },
            block_hash: None,
            block_number: Some(self.block_number),
            block_timestamp: None,
            transaction_hash: Some(self.tx_hash),
            transaction_index: Some(0),
            log_index: Some(self.log_index),
            removed: false,
        }
    }

    pub fn reserve_frozen(self, asset: Address) -> Log {
        let event = ReserveFrozen { asset };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_unfrozen(self, asset: Address) -> Log {
        let event = ReserveUnfrozen { asset };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_paused(self, asset: Address, paused: bool) -> Log {
        let event = ReservePaused { asset, paused };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_borrowing(self, asset: Address, enabled: bool) -> Log {
        let event = ReserveBorrowing { asset, enabled };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_active(self, asset: Address, active: bool) -> Log {
        let event = ReserveActive { asset, active };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_dropped(self, asset: Address) -> Log {
        let event = ReserveDropped { asset };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn supply_cap_changed(self, asset: Address, old_cap: u64, new_cap: u64) -> Log {
        let event = SupplyCapChanged {
            asset,
            oldSupplyCap: U256::from(old_cap),
            newSupplyCap: U256::from(new_cap),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn borrow_cap_changed(self, asset: Address, old_cap: u64, new_cap: u64) -> Log {
        let event = BorrowCapChanged {
            asset,
            oldBorrowCap: U256::from(old_cap),
            newBorrowCap: U256::from(new_cap),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_factor_changed(self, asset: Address, old_factor: u64, new_factor: u64) -> Log {
        let event = ReserveFactorChanged {
            asset,
            oldReserveFactor: U256::from(old_factor),
            newReserveFactor: U256::from(new_factor),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn collateral_config_changed(
        self,
        asset: Address,
        ltv: u64,
        threshold: u64,
        bonus: u64,
    ) -> Log {
        let event = CollateralConfigurationChanged {
            asset,
            ltv: U256::from(ltv),
            liquidationThreshold: U256::from(threshold),
            liquidationBonus: U256::from(bonus),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn interest_rate_strategy_changed(
        self,
        asset: Address,
        old_strategy: Address,
        new_strategy: Address,
    ) -> Log {
        let event = ReserveInterestRateStrategyChanged {
            asset,
            oldStrategy: old_strategy,
            newStrategy: new_strategy,
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn flash_loan_changed(self, asset: Address, enabled: bool) -> Log {
        let event = ReserveFlashLoaning { asset, enabled };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn asset_collateral_in_emode_changed(
        self,
        asset: Address,
        category_id: u8,
        collateral: bool,
    ) -> Log {
        let event = AssetCollateralInEModeChanged { asset, categoryId: category_id, collateral };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn asset_borrowable_in_emode_changed(
        self,
        asset: Address,
        category_id: u8,
        borrowable: bool,
    ) -> Log {
        let event = AssetBorrowableInEModeChanged { asset, categoryId: category_id, borrowable };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn asset_ltvzero_in_emode_changed(
        self,
        asset: Address,
        category_id: u8,
        ltvzero: bool,
    ) -> Log {
        let event = AssetLtvzeroInEModeChanged { asset, categoryId: category_id, ltvzero };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn emode_category_added(
        self,
        category_id: u8,
        ltv: u64,
        liquidation_threshold: u64,
        liquidation_bonus: u64,
        oracle: Address,
        label: String,
    ) -> Log {
        let event = EModeCategoryAdded {
            categoryId: category_id,
            ltv: U256::from(ltv),
            liquidationThreshold: U256::from(liquidation_threshold),
            liquidationBonus: U256::from(liquidation_bonus),
            oracle,
            label,
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn user_emode_set(self, user: Address, category_id: u8) -> Log {
        let event = UserEModeSet { user, categoryId: category_id };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn debt_ceiling_changed(self, asset: Address, old_ceiling: u64, new_ceiling: u64) -> Log {
        let event = DebtCeilingChanged {
            asset,
            oldDebtCeiling: U256::from(old_ceiling),
            newDebtCeiling: U256::from(new_ceiling),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn liquidation_protocol_fee_changed(
        self,
        asset: Address,
        old_fee: u64,
        new_fee: u64,
    ) -> Log {
        let event = LiquidationProtocolFeeChanged {
            asset,
            oldFee: U256::from(old_fee),
            newFee: U256::from(new_fee),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn siloed_borrowing_changed(self, asset: Address, old_state: bool, new_state: bool) -> Log {
        let event = SiloedBorrowingChanged {
            asset,
            oldState: old_state,
            newState: new_state,
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn unbacked_mint_cap_changed(self, asset: Address, old_cap: u64, new_cap: u64) -> Log {
        let event = UnbackedMintCapChanged {
            asset,
            oldUnbackedMintCap: U256::from(old_cap),
            newUnbackedMintCap: U256::from(new_cap),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }

    pub fn reserve_data_updated(
        self,
        reserve: Address,
        liquidity_rate: u64,
        stable_borrow_rate: u64,
        variable_borrow_rate: u64,
        liquidity_index: u64,
        variable_borrow_index: u64,
    ) -> Log {
        let event = ReserveDataUpdated {
            reserve,
            liquidityRate: U256::from(liquidity_rate),
            stableBorrowRate: U256::from(stable_borrow_rate),
            variableBorrowRate: U256::from(variable_borrow_rate),
            liquidityIndex: U256::from(liquidity_index),
            variableBorrowIndex: U256::from(variable_borrow_index),
        };
        self.wrap_log_data(Address::ZERO, event.encode_log_data())
    }
}
