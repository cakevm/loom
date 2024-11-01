use loom_core_blockchain::Blockchain;
use loom_storage_db::DbPool;
use loom_types_entities::{Pool, PoolEnumTrait};
use std::fmt::{Debug, Display};

#[derive(Clone)]
pub struct AppState<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static> {
    pub db: DbPool,
    pub bc: Blockchain<PoolEnum>,
}
