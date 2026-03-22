# httpress-example

An example project showing how to use [httpress](https://github.com/GabrielTecuceanu/httpress)
inside a Rust test suite as performance regression tests that fail CI if SLAs are violated.

## What this demonstrates

Most load testing tools are CLI-only - you run them manually and eyeball the output. This repo
shows a different approach: httpress has a Rust API, so you can write performance assertions
directly in `cargo test`, the same way you write correctness assertions. If throughput drops or
latency spikes, the test fails and CI catches it.

## The server

A simple in-memory key-value store built with axum, used as the system under test.

```
GET    /keys              - list all keys
GET    /keys/:key         - get a value  (404 if missing)
GET    /keys/:key/slow    - same as above but sleeps 50ms (used to demonstrate a failing SLA test)
POST   /keys              - create  { "key": "...", "value": "..." }  (409 if duplicate)
PUT    /keys/:key         - update  { "value": "..." }  (404 if missing)
DELETE /keys/:key         - delete  (404 if missing)
```

## Running the tests

```sh
# correctness tests
cargo test --test functional

# performance tests (run sequentially to avoid CPU contention between benchmarks)
cargo test --test performance -- --test-threads=1

# everything
cargo test
```

Performance results are written to `target/httpress-tests-out/<test_name>.txt`
after each run.

## Performance tests

| Test                     | Scenario                                     | SLA                               |
| ------------------------ | -------------------------------------------- | --------------------------------- |
| `read_throughput_sla`    | 50 concurrent GETs, 5s                       | throughput > 5k req/s, p99 < 10ms |
| `write_throughput_sla`   | 50 concurrent POSTs (unique keys), 5s        | throughput > 2k req/s, p99 < 20ms |
| `mixed_workload_sla`     | 80% GET / 20% POST via `request_fn`, 5s      | p99 < 15ms, 0 errors              |
| `concurrency_scaling`    | GET at c=1, c=10, c=50                       | informational, no assertion       |
| `ramp_rate`              | `rate_fn` linear ramp 100 -> 2000 req/s, 15s | 0 errors                          |
| `circuit_breaker`        | `before_request` + `after_request` hooks, 5s | informational, no assertion       |
| `rate_limited_stability` | steady 1k req/s, 30s                         | 0 errors                          |
| `slow_endpoint_sla`      | 50 concurrent GETs against `/slow`, 5s       | expected to fail (p99 < 5ms)      |

### `request_fn`

Used in `write_throughput_sla` and `mixed_workload_sla` to generate a different
request per worker/iteration.

```rust
.request_fn(move |ctx| RequestConfig {
    url: format!("{addr}/keys"),
    method: HttpMethod::Post,
    headers: json_headers(),
    body: json_body(&format!("k_{}_{}", ctx.worker_id, ctx.request_number), "x"),
})
```

### `rate_fn`

Used in `ramp_rate` to control request rate dynamically based on elapsed time:

```rust
.rate_fn(|ctx| {
    let t = ctx.elapsed.as_secs_f64().min(15.0);
    100.0 + (2000.0 - 100.0) * (t / 15.0)
})
```

### Hooks

Used in `circuit_breaker` to observe and react to individual requests:

```rust
.before_request(|ctx| {
    if ctx.failed_requests > 10 {
        HookAction::Abort  // stop sending to a failing server
    } else {
        HookAction::Continue
    }
})
.after_request(|ctx| {
    if ctx.latency > Duration::from_millis(5) {
        // log slow requests to the output file
    }
    HookAction::Continue
})
```

## CI

GitHub Actions runs the full test suite on every push/pull request. Performance
test results are uploaded as artifacts so you can inspect them from the Actions UI.
