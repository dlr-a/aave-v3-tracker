use alloy::primitives::U256;
use bigdecimal::BigDecimal;
use std::str::FromStr;

fn to_bigdecimal(val: U256) -> Result<BigDecimal, String> {
    BigDecimal::from_str(&val.to_string())
        .map_err(|e| format!("BigDecimal conversion error: {}", e))
}

#[test]
fn test_to_bigdecimal_zero() {
    let val = U256::ZERO;
    let result = to_bigdecimal(val).unwrap();
    assert_eq!(result, BigDecimal::from(0));
}

#[test]
fn test_to_bigdecimal_max_uint256() {
    let val = U256::MAX;
    let result = to_bigdecimal(val).unwrap();

    // U256::MAX = 2^256 - 1
    let expected = BigDecimal::from_str(
        "115792089237316195423570985008687907853269984665640564039457584007913129639935",
    )
    .unwrap();
    assert_eq!(result, expected);
}
