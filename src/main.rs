mod routes;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::sync::RwLock;

use routes::Store;

#[tokio::main]
async fn main() {
    let store: Store = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new()
        .route("/keys", get(routes::list_keys).post(routes::create_key))
        .route(
            "/keys/{key}",
            get(routes::get_key)
                .put(routes::update_key)
                .delete(routes::delete_key),
        )
        .with_state(store);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
