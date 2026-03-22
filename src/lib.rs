pub mod routes;

use axum::{Router, routing::get};

use routes::Store;

pub fn build_router(store: Store) -> Router {
    Router::new()
        .route("/keys", get(routes::list_keys).post(routes::create_key))
        .route(
            "/keys/{key}",
            get(routes::get_key)
                .put(routes::update_key)
                .delete(routes::delete_key),
        )
        .route("/keys/{key}/slow", get(routes::get_key_slow))
        .with_state(store)
}
