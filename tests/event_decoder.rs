use aave_v3_tracker::abi::{
    BorrowCapChanged, CollateralConfigurationChanged, DebtCeilingChanged,
    EModeAssetCategoryChanged, LiquidationProtocolFeeChanged, ReserveActive, ReserveBorrowing,
    ReserveDataUpdated, ReserveDropped, ReserveFactorChanged, ReserveFlashLoaning, ReserveFrozen,
    ReserveInitialized, ReserveInterestRateStrategyChanged, ReservePaused,
    ReserveStableRateBorrowing, ReserveUnfrozen, SiloedBorrowingChanged, SupplyCapChanged,
    UnbackedMintCapChanged,
};
use aave_v3_tracker::sync_reserves::reserve_event_handler::{ProcessedLog, decode_log_type};
use alloy::primitives::{Address, B256, LogData};
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;

fn create_log_with_topic(topic0: B256, data: Vec<u8>, topics: Vec<B256>) -> Log {
    let mut all_topics = vec![topic0];
    all_topics.extend(topics);

    Log {
        inner: alloy::primitives::Log {
            address: Address::ZERO,
            data: LogData::new(all_topics, data.into()).unwrap(),
        },
        block_hash: None,
        block_number: Some(100),
        block_timestamp: None,
        transaction_hash: Some(B256::ZERO),
        transaction_index: Some(0),
        log_index: Some(0),
        removed: false,
    }
}

#[test]
fn test_decode_log_type_unknown_returns_none() {
    let unknown_topic = B256::repeat_byte(0xFF);
    let log = create_log_with_topic(unknown_topic, vec![], vec![]);

    let result = decode_log_type(&log);
    assert!(result.is_none());
}

#[test]
fn test_decode_log_type_empty_topics_returns_none() {
    let log = Log {
        inner: alloy::primitives::Log {
            address: Address::ZERO,
            data: LogData::new(vec![], vec![].into()).unwrap(),
        },
        block_hash: None,
        block_number: Some(100),
        block_timestamp: None,
        transaction_hash: Some(B256::ZERO),
        transaction_index: Some(0),
        log_index: Some(0),
        removed: false,
    };

    let result = decode_log_type(&log);
    assert!(result.is_none());
}

#[test]
fn test_decode_reserve_frozen_event() {
    let asset = Address::repeat_byte(0x11);

    let topic0 = ReserveFrozen::SIGNATURE_HASH;
    let asset_topic = B256::left_padding_from(asset.as_slice());

    let log = create_log_with_topic(topic0, vec![], vec![asset_topic]);

    let result = decode_log_type(&log);
    assert!(matches!(result, Some(ProcessedLog::ReserveFrozen(_))));
}

#[test]
fn test_decode_reserve_unfrozen_event() {
    let asset = Address::repeat_byte(0x22);

    let topic0 = ReserveUnfrozen::SIGNATURE_HASH;
    let asset_topic = B256::left_padding_from(asset.as_slice());

    let log = create_log_with_topic(topic0, vec![], vec![asset_topic]);

    let result = decode_log_type(&log);
    assert!(matches!(result, Some(ProcessedLog::ReserveUnfrozen(_))));
}

#[test]
fn test_decode_reserve_paused_event() {
    let asset = Address::repeat_byte(0x33);

    let topic0 = ReservePaused::SIGNATURE_HASH;
    let asset_topic = B256::left_padding_from(asset.as_slice());

    let mut data = vec![0u8; 32];
    data[31] = 1;

    let log = create_log_with_topic(topic0, data, vec![asset_topic]);

    let result = decode_log_type(&log);
    assert!(matches!(result, Some(ProcessedLog::ReservePaused(_))));
}

#[test]
fn test_signature_hashes_are_distinct() {
    // ensure all event signatures are unique
    let signatures = vec![
        ReserveInitialized::SIGNATURE_HASH,
        ReserveDataUpdated::SIGNATURE_HASH,
        ReserveStableRateBorrowing::SIGNATURE_HASH,
        ReserveDropped::SIGNATURE_HASH,
        ReserveFactorChanged::SIGNATURE_HASH,
        ReserveInterestRateStrategyChanged::SIGNATURE_HASH,
        CollateralConfigurationChanged::SIGNATURE_HASH,
        ReserveFrozen::SIGNATURE_HASH,
        ReserveUnfrozen::SIGNATURE_HASH,
        ReservePaused::SIGNATURE_HASH,
        ReserveBorrowing::SIGNATURE_HASH,
        ReserveActive::SIGNATURE_HASH,
        BorrowCapChanged::SIGNATURE_HASH,
        SupplyCapChanged::SIGNATURE_HASH,
        ReserveFlashLoaning::SIGNATURE_HASH,
        EModeAssetCategoryChanged::SIGNATURE_HASH,
        DebtCeilingChanged::SIGNATURE_HASH,
        LiquidationProtocolFeeChanged::SIGNATURE_HASH,
        SiloedBorrowingChanged::SIGNATURE_HASH,
        UnbackedMintCapChanged::SIGNATURE_HASH,
    ];

    for (i, sig1) in signatures.iter().enumerate() {
        for (j, sig2) in signatures.iter().enumerate() {
            if i != j {
                assert_ne!(sig1, sig2, "Signature {} and {} should be different", i, j);
            }
        }
    }
}
