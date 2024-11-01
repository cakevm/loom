use crate::handler::blocks::latest_block;
use crate::handler::flashbots::flashbots;
use crate::handler::pools::{market_stats, pool, pool_quote, pools};
use crate::handler::ws::ws_handler;
use crate::openapi::ApiDoc;
use axum::routing::{get, post};
use axum::Router;
use loom_rpc_state::AppState;
use loom_types_entities::Pool;
use std::fmt::{Debug, Display};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn router<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static>(
    app_state: AppState<PoolEnum>,
) -> Router<()> {
    Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .nest("/block", router_block::<PoolEnum>()) // rename to node
                .nest("/markets", router_market::<PoolEnum>())
                .nest("/flashbots", Router::new().route("/", post(flashbots::<PoolEnum>))),
        )
        .route("/ws", get(ws_handler::<PoolEnum>))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(app_state)
}

pub fn router_block<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static>() -> Router<AppState<PoolEnum>> {
    Router::new().route("/latest_block", get(latest_block::<PoolEnum>))
}

pub fn router_market<PoolEnum: PoolEnumTrait + Pool + Clone + Eq + Send + Sync + Display + Debug + 'static>() -> Router<AppState<PoolEnum>>
{
    Router::new()
        .route("/pools/:address", get(pool::<PoolEnum>))
        .route("/pools/:address/quote", post(pool_quote::<PoolEnum>))
        .route("/pools", get(pools::<PoolEnum>))
        .route("/", get(market_stats::<PoolEnum>))
}
