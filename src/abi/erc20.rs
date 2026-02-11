use alloy::sol;

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);

    #[sol(rpc)]
    interface IERC20 {
        function symbol() external view returns (string);
        function totalSupply() external view returns (uint256);
    }
}
