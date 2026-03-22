use std::collections::HashMap;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::RwLock;

pub struct TestServer {
    pub addr: String,
}

pub async fn spawn_test_server() -> TestServer {
    let store = Arc::new(RwLock::new(HashMap::new()));
    let app = httpress_example::build_router(store);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestServer {
        addr: format!("http://127.0.0.1:{port}"),
    }
}
