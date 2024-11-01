use alloy_primitives::Address;
use eyre::Result;
use loom_core_actors::SharedState;
use loom_types_blockchain::GethStateUpdateVec;
use loom_types_entities::{Market, Pool, PoolEnumTrait, PoolWrapper};
use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

pub async fn get_affected_pools<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static>(
    market: SharedState<Market<PoolEnum>>,
    state_update: &GethStateUpdateVec,
) -> Result<BTreeMap<PoolWrapper<PoolEnum>, Vec<(Address, Address)>>> {
    let market_guard = market.read().await;

    let mut affected_pools: BTreeMap<PoolWrapper<PoolEnum>, Vec<(Address, Address)>> = BTreeMap::new();

    for state_update_record in state_update.iter() {
        for (address, _state_update_entry) in state_update_record.iter() {
            if let Some(pool) = market_guard.get_pool(address) {
                if affected_pools.contains_key(pool) || !market_guard.is_pool(address) {
                    continue;
                }
                let swap_directions = pool.get_swap_directions();
                affected_pools.insert(pool.clone(), swap_directions.clone());
            }
        }
    }

    Ok(affected_pools)
}
