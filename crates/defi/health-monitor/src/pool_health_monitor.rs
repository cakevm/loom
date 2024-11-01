use alloy_primitives::Address;
use eyre::Result;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, error, info};

use loom_core_actors::{subscribe, Accessor, Actor, ActorResult, Broadcaster, Consumer, SharedState, WorkerResult};
use loom_core_actors_macros::{Accessor, Consumer};
use loom_core_blockchain::Blockchain;
use loom_types_entities::{Market, Pool, PoolEnumTrait};
use loom_types_events::{HealthEvent, MessageHealthEvent};

pub async fn pool_health_monitor_worker<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static>(
    market: SharedState<Market<PoolEnum>>,
    pool_health_monitor_rx: Broadcaster<MessageHealthEvent>,
) -> WorkerResult {
    subscribe!(pool_health_monitor_rx);

    let mut pool_errors_map: HashMap<Address, u32> = HashMap::new();

    loop {
        tokio::select! {
            msg = pool_health_monitor_rx.recv() => {

                let pool_health_update : Result<MessageHealthEvent, RecvError>  = msg;
                match pool_health_update {
                    Ok(pool_health_message)=>{
                        if let HealthEvent::PoolSwapError(swap_error) = pool_health_message.inner {
                            debug!("Pool health_monitor message update: {:?} {} {} ", swap_error.pool, swap_error.msg, swap_error.amount);
                            let entry = pool_errors_map.entry(swap_error.pool).or_insert(0);
                            *entry += 1;
                            if *entry >= 10 {
                                let mut market_guard = market.write().await;
                                market_guard.set_pool_ok(swap_error.pool, false);
                                match market_guard.get_pool(&swap_error.pool) {
                                    Some(pool)=>{
                                        info!("Disabling pool: protocol={}, address={:?}, msg={} amount={}", pool.get_protocol(),swap_error.pool, swap_error.msg, swap_error.amount);
                                    }
                                    _=>{
                                        error!("Disabled pool missing in market: address={:?}, msg={} amount={}", swap_error.pool, swap_error.msg, swap_error.amount);
                                    }
                                }
                            }
                        }
                    }
                    Err(e)=>{
                        error!("pool_health_update error {}", e)
                    }
                }

            }
        }
    }
}

#[derive(Accessor, Consumer)]
pub struct PoolHealthMonitorActor<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> {
    #[accessor]
    market: Option<SharedState<Market<PoolEnum>>>,
    #[consumer]
    pool_health_update_rx: Option<Broadcaster<MessageHealthEvent>>,
}

impl<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> PoolHealthMonitorActor<PoolEnum> {
    pub fn new() -> Self {
        PoolHealthMonitorActor { market: None, pool_health_update_rx: None }
    }

    pub fn on_bc(self, bc: &Blockchain<PoolEnum>) -> Self {
        Self { market: Some(bc.market()), pool_health_update_rx: Some(bc.pool_health_monitor_channel()) }
    }
}

impl<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> Actor for PoolHealthMonitorActor<PoolEnum> {
    fn start(&self) -> ActorResult {
        let task =
            tokio::task::spawn(pool_health_monitor_worker(self.market.clone().unwrap(), self.pool_health_update_rx.clone().unwrap()));
        Ok(vec![task])
    }

    fn name(&self) -> &'static str {
        "PoolHealthMonitorActor"
    }
}
