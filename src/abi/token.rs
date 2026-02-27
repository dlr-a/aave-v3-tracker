use alloy::sol;

sol! {
    event Mint(
        address indexed caller,
        address indexed onBehalfOf,
        uint256 value,
        uint256 balanceIncrease,
        uint256 index
    );

    event Burn(
        address indexed from,
        address indexed target,
        uint256 value,
        uint256 balanceIncrease,
        uint256 index
    );

    event BalanceTransfer(
        address indexed from,
        address indexed to,
        uint256 value,
        uint256 index
    );

    #[sol(rpc)]
    interface IScaledBalanceToken {
        function scaledBalanceOf(address user) external view returns (uint256);
    }
}
