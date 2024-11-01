use crate::tips::Tips;
use crate::{Pool, PoolEnumTrait, Swap};
use alloy_primitives::{Address, BlockNumber, Bytes, U256};
use eyre::Result;
use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::sync::Arc;

pub trait SwapEncoder<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> {
    /// Encodes Swap
    ///
    /// - next_block_number - number of the next block
    /// - next_block_gas_price - base_fee + priority fee for transaction
    /// - sender_address - EOA of of the transaction
    /// - sender_eth_balance - balance of EOA
    ///
    /// returns (to. value, call_data) for transaction

    fn encode(
        &self,
        swap: Swap<PoolEnum>,
        tips_pct: Option<u32>,
        next_block_number: Option<BlockNumber>,
        gas_cost: Option<U256>,
        sender_address: Option<Address>,
        sender_eth_balance: Option<U256>,
    ) -> Result<(Address, Option<U256>, Bytes, Vec<Tips>)>
    where
        Self: Sized;
}

#[derive(Clone)]
pub struct SwapEncoderWrapper<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> {
    pub inner: Arc<dyn SwapEncoder<PoolEnum>>,
}

impl<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> SwapEncoderWrapper<PoolEnum> {
    pub fn new(encoder: Arc<dyn SwapEncoder<PoolEnum>>) -> Self {
        SwapEncoderWrapper { inner: encoder }
    }
}

/*
impl<T: 'static + SwapEncoder<PoolEnum> + Clone, PoolEnum: Pool> From<T> for SwapEncoderWrapper<PoolEnum> {
    fn from(pool: T) -> Self {
        Self { inner: Arc::new(pool) }
    }
}

 */

impl<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> Deref for SwapEncoderWrapper<PoolEnum> {
    type Target = dyn SwapEncoder<PoolEnum>;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}
