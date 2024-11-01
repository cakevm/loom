use alloy_primitives::{Address, U256};
use eyre::ErrReport;
use eyre::Result;
use loom_defi_pools::{UniswapV2Pool, UniswapV3Pool};
use loom_evm_db::LoomDBType;
use loom_types_entities::required_state::RequiredState;
use loom_types_entities::AbiSwapEncoder;
use loom_types_entities::PoolClass;
use loom_types_entities::PoolProtocol;
use loom_types_entities::{Pool, PoolEnumTrait};
use revm::primitives::Env;
use std::ops::Deref;

#[enum_delegate::implement(Pool)]
pub enum MarketPoolEnum {
    UniswapV2(UniswapV2Pool),
    UniswapV3(UniswapV3Pool),
}

impl PoolEnumTrait for MarketPoolEnum {
    fn try_from_pool(pool: &(impl Pool)) -> Result<Self> {
        Ok(MarketPoolEnum::try_from(pool.into())?)
    }
}

#[test]
mod tests {
    use super::*;
    use loom_types_entities::PoolClass;

    #[test]
    fn can_delegate_pool() {
        let pool = MarketPoolEnum::UniswapV2(UniswapV2Pool::new(Address::ZERO));
        assert_eq!(pool.get_class(), PoolClass::UniswapV2);
    }
}
