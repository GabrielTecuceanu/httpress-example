mod common;

use serde_json::json;

#[tokio::test]
async fn create_and_get_key() {
    let server = common::spawn_test_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/keys", server.addr))
        .json(&json!({ "key": "foo", "value": "bar" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 201);

    let res = client
        .get(format!("{}/keys/foo", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["value"], "bar");
}

#[tokio::test]
async fn update_key() {
    let server = common::spawn_test_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{}/keys", server.addr))
        .json(&json!({ "key": "foo", "value": "bar" }))
        .send()
        .await
        .unwrap();

    let res = client
        .put(format!("{}/keys/foo", server.addr))
        .json(&json!({ "value": "baz" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = client
        .get(format!("{}/keys/foo", server.addr))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["value"], "baz");
}

#[tokio::test]
async fn delete_key() {
    let server = common::spawn_test_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{}/keys", server.addr))
        .json(&json!({ "key": "foo", "value": "bar" }))
        .send()
        .await
        .unwrap();

    let res = client
        .delete(format!("{}/keys/foo", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let res = client
        .get(format!("{}/keys/foo", server.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn list_keys() {
    let server = common::spawn_test_server().await;
    let client = reqwest::Client::new();

    for (k, v) in [("a", "1"), ("b", "2")] {
        client
            .post(format!("{}/keys", server.addr))
            .json(&json!({ "key": k, "value": v }))
            .send()
            .await
            .unwrap();
    }

    let mut keys: Vec<String> = client
        .get(format!("{}/keys", server.addr))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    keys.sort();
    assert_eq!(keys, ["a", "b"]);
}

#[tokio::test]
async fn conflict_on_duplicate_key() {
    let server = common::spawn_test_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{}/keys", server.addr))
        .json(&json!({ "key": "foo", "value": "bar" }))
        .send()
        .await
        .unwrap();

    let res = client
        .post(format!("{}/keys", server.addr))
        .json(&json!({ "key": "foo", "value": "baz" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 409);
}
