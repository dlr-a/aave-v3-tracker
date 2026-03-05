use alloy::sol;

sol! {
    #[sol(rpc)]
    interface IUiPoolDataProviderV3 {
        #[derive(Debug)]
        struct EModeCategory {
            uint16 ltv;
            uint16 liquidationThreshold;
            uint16 liquidationBonus;
            uint128 collateralBitmap;
            string label;
            uint128 borrowableBitmap;
            uint128 ltvzeroBitmap;
        }

        #[derive(Debug)]
        struct Emode {
            uint8 id;
            EModeCategory eMode;
        }

        function getEModes(address provider) external view returns (Emode[] memory);
    }
}
