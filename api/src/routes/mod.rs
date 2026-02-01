//! Route definitions for the API.
//!
//! Provides route registration for all API endpoints.

use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::state::AppState;

/// Creates the API router with all routes.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Market endpoints
        .route("/v1/markets", get(handlers::list_markets))
        .route("/v1/markets/{market}", get(handlers::get_market))
        .route(
            "/v1/markets/{market}/orderbook",
            get(handlers::get_orderbook),
        )
        .route("/v1/markets/{market}/trades", get(handlers::get_trades))
        .route("/v1/markets/{market}/candles", get(handlers::get_candles))
        // Account endpoints
        .route(
            "/v1/accounts/{owner}/orders",
            get(handlers::get_user_orders),
        )
        .route(
            "/v1/accounts/{owner}/trades",
            get(handlers::get_user_trades),
        )
        .route(
            "/v1/accounts/{owner}/balances",
            get(handlers::get_user_balances),
        )
        // Transaction building endpoints
        .route("/v1/tx/place-order", post(handlers::build_place_order))
        .route("/v1/tx/cancel-order", post(handlers::build_cancel_order))
        .route("/v1/tx/deposit", post(handlers::build_deposit))
        .route("/v1/tx/withdraw", post(handlers::build_withdraw))
        .with_state(state)
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router() {
        let state = AppState::default();
        let _router = create_router(state);
    }
}
