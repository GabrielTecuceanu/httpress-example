mod common;

use std::collections::HashMap;
use std::time::Duration;

use bytes::Bytes;
use httpress::{BenchmarkBuilder, HookAction, HttpMethod, RequestConfig};

async fn seed_key(addr: &str, key: &str) {
    reqwest::Client::new()
        .post(format!("{addr}/keys"))
        .json(&serde_json::json!({ "key": key, "value": "x" }))
        .send()
        .await
        .unwrap();
}

fn json_body(key: &str, value: &str) -> Option<Bytes> {
    Some(Bytes::from(
        serde_json::json!({ "key": key, "value": value }).to_string(),
    ))
}

fn json_headers() -> HashMap<String, String> {
    HashMap::from([("content-type".to_string(), "application/json".to_string())])
}

// 50 concurrent GETs for 5s - asserts throughput > 5k req/s and p99 < 10ms.
#[tokio::test]
async fn read_throughput_sla() {
    let server = common::spawn_test_server().await;
    seed_key(&server.addr, "bench").await;

    let results = BenchmarkBuilder::new()
        .url(&format!("{}/keys/bench", server.addr))
        .concurrency(50)
        .duration(Duration::from_secs(5))
        .build()
        .unwrap()
        .run()
        .await
        .unwrap();

    results.print();

    assert!(
        results.throughput > 5_000.0,
        "expected >5k req/s, got {:.0}",
        results.throughput
    );
    assert!(
        results.latency_p99 < Duration::from_millis(10),
        "expected p99 < 10ms, got {:?}",
        results.latency_p99
    );
}

// 50 concurrent POSTs for 5s with unique keys - asserts throughput > 2k req/s and p99 < 20ms.
#[tokio::test]
async fn write_throughput_sla() {
    let server = common::spawn_test_server().await;
    let addr = server.addr.clone();

    let results = BenchmarkBuilder::new()
        .concurrency(50)
        .duration(Duration::from_secs(5))
        .request_fn(move |ctx| RequestConfig {
            url: format!("{addr}/keys"),
            method: HttpMethod::Post,
            headers: json_headers(),
            body: json_body(&format!("k_{}_{}", ctx.worker_id, ctx.request_number), "x"),
        })
        .build()
        .unwrap()
        .run()
        .await
        .unwrap();

    results.print();

    assert!(
        results.throughput > 2_000.0,
        "expected >2k req/s, got {:.0}",
        results.throughput
    );
    assert!(
        results.latency_p99 < Duration::from_millis(20),
        "expected p99 < 20ms, got {:?}",
        results.latency_p99
    );
}

// Mixed workload: 80% GET, 20% POST with unique keys - asserts p99 < 15ms and 0 errors.
#[tokio::test]
async fn mixed_workload_sla() {
    let server = common::spawn_test_server().await;
    seed_key(&server.addr, "bench").await;
    let addr = server.addr.clone();

    let results = BenchmarkBuilder::new()
        .concurrency(50)
        .duration(Duration::from_secs(5))
        .request_fn(move |ctx| {
            if ctx.request_number % 5 == 0 {
                RequestConfig {
                    url: format!("{addr}/keys"),
                    method: HttpMethod::Post,
                    headers: json_headers(),
                    body: json_body(&format!("k_{}_{}", ctx.worker_id, ctx.request_number), "x"),
                }
            } else {
                RequestConfig {
                    url: format!("{addr}/keys/bench"),
                    method: HttpMethod::Get,
                    headers: HashMap::new(),
                    body: None,
                }
            }
        })
        .build()
        .unwrap()
        .run()
        .await
        .unwrap();

    results.print();

    assert!(
        results.latency_p99 < Duration::from_millis(15),
        "expected p99 < 15ms, got {:?}",
        results.latency_p99
    );
    assert_eq!(results.failed_requests, 0, "expected 0 errors");
}

// Informational: same GET benchmark at c=1, c=10, c=50. No assertions - shows scaling behaviour.
#[tokio::test]
async fn concurrency_scaling() {
    let server = common::spawn_test_server().await;
    seed_key(&server.addr, "bench").await;

    for concurrency in [1, 10, 50] {
        println!("--- concurrency={concurrency} ---");
        let results = BenchmarkBuilder::new()
            .url(&format!("{}/keys/bench", server.addr))
            .concurrency(concurrency)
            .duration(Duration::from_secs(3))
            .build()
            .unwrap()
            .run()
            .await
            .unwrap();
        results.print();
    }
}

// Ramps from 100 to 2000 req/s linearly over 15s - asserts 0 errors under increasing load.
#[tokio::test]
async fn ramp_rate() {
    let server = common::spawn_test_server().await;
    seed_key(&server.addr, "bench").await;

    let results = BenchmarkBuilder::new()
        .url(&format!("{}/keys/bench", server.addr))
        .rate_fn(|ctx| {
            // Linear ramp: 100 req/s at t=0, 2000 req/s at t=15s.
            let t = ctx.elapsed.as_secs_f64().min(15.0);
            100.0 + (2000.0 - 100.0) * (t / 15.0)
        })
        .duration(Duration::from_secs(15))
        .build()
        .unwrap()
        .run()
        .await
        .unwrap();

    results.print();

    assert_eq!(results.failed_requests, 0, "expected 0 errors during ramp");
}

// Demonstrates before_request and after_request hooks:
// - before_request aborts new requests once 10 failures are seen (circuit breaker).
// - after_request prints a warning for any request that took longer than 5ms.
#[tokio::test]
async fn circuit_breaker() {
    let server = common::spawn_test_server().await;
    seed_key(&server.addr, "bench").await;

    let results = BenchmarkBuilder::new()
        .url(&format!("{}/keys/bench", server.addr))
        .concurrency(50)
        .duration(Duration::from_secs(5))
        .before_request(|ctx| {
            if ctx.failed_requests > 10 {
                println!(
                    "[circuit breaker] {} failures — aborting request",
                    ctx.failed_requests
                );
                HookAction::Abort
            } else {
                HookAction::Continue
            }
        })
        .after_request(|ctx| {
            if ctx.latency > Duration::from_millis(5) {
                println!(
                    "[slow request] worker={} req={} latency={:?}",
                    ctx.worker_id, ctx.request_number, ctx.latency
                );
            }
            HookAction::Continue
        })
        .build()
        .unwrap()
        .run()
        .await
        .unwrap();

    results.print();
}

// Steady 1k req/s for 30s - asserts 0 errors under sustained rate.
#[tokio::test]
async fn rate_limited_stability() {
    let server = common::spawn_test_server().await;
    seed_key(&server.addr, "bench").await;

    let results = BenchmarkBuilder::new()
        .url(&format!("{}/keys/bench", server.addr))
        .rate(1_000)
        .duration(Duration::from_secs(30))
        .build()
        .unwrap()
        .run()
        .await
        .unwrap();

    results.print();

    assert_eq!(
        results.failed_requests, 0,
        "expected 0 errors under rate limit"
    );
}
