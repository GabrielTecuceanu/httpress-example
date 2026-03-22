use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub type Store = Arc<RwLock<HashMap<String, String>>>;

#[derive(Deserialize)]
pub struct CreatePayload {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct UpdatePayload {
    pub value: String,
}

#[derive(Serialize)]
struct ValueResponse {
    value: String,
}

pub async fn list_keys(State(store): State<Store>) -> impl IntoResponse {
    let keys: Vec<String> = store.read().await.keys().cloned().collect();
    Json(keys)
}

pub async fn get_key(State(store): State<Store>, Path(key): Path<String>) -> impl IntoResponse {
    match store.read().await.get(&key).cloned() {
        Some(value) => Json(ValueResponse { value }).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn create_key(
    State(store): State<Store>,
    Json(payload): Json<CreatePayload>,
) -> impl IntoResponse {
    let mut map = store.write().await;
    if map.contains_key(&payload.key) {
        return StatusCode::CONFLICT.into_response();
    }
    map.insert(payload.key, payload.value);
    StatusCode::CREATED.into_response()
}

pub async fn update_key(
    State(store): State<Store>,
    Path(key): Path<String>,
    Json(payload): Json<UpdatePayload>,
) -> impl IntoResponse {
    let mut map = store.write().await;
    if !map.contains_key(&key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    map.insert(key, payload.value);
    StatusCode::OK.into_response()
}

// Deliberately slow endpoint - sleeps 50ms before responding.
pub async fn get_key_slow(
    State(store): State<Store>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    match store.read().await.get(&key).cloned() {
        Some(value) => Json(ValueResponse { value }).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn delete_key(State(store): State<Store>, Path(key): Path<String>) -> impl IntoResponse {
    let mut map = store.write().await;
    if map.remove(&key).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}
