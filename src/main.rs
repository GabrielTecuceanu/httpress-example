use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let store = Arc::new(RwLock::new(HashMap::new()));
    let app = httpress_example::build_router(store);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
