use axum::body::Body;
use axum::extract::Json as AxumJson;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_harmonicmean::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

const DEAD_URL: &str = "http://127.0.0.1:1";

fn approx(got: f64, expected: f64) -> bool {
    (got - expected).abs() < 1e-9
}

// --- Computing mocks: each reads its request body and returns the real result.
//
// The harmonic-mean orchestration is genuinely driven by these answers rather
// than canned values, so the asserted cases exercise the full composition.

/// `srvcs-reciprocal`: `{"value": v}` -> `{"result": 1 / v}`.
async fn spawn_reciprocal() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let v = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": 1.0 / v }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatadd`: `{"a": x, "b": y}` -> `{"result": x + y}`.
async fn spawn_floatadd() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a + b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatmultiply`: `{"a": x, "b": y}` -> `{"result": x * y}`.
#[allow(dead_code)]
async fn spawn_floatmultiply() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a * b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatdivide`: `{"a": x, "b": y}` -> `{"result": x / y}`.
async fn spawn_floatdivide() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": a / b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatsubtract`: `{"a": x, "b": y}` -> `{"result": x - y}`.
#[allow(dead_code)]
async fn spawn_floatsubtract() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_f64).unwrap_or(0.0);
            let b = body.get("b").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": a - b }))
        }),
    );
    serve(app).await
}

/// `srvcs-floatpower`: `{"base": b, "exp": e}` -> `{"result": b.powf(e)}`.
#[allow(dead_code)]
async fn spawn_floatpower() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let base = body.get("base").and_then(Value::as_f64).unwrap_or(0.0);
            let exp = body.get("exp").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": base.powf(exp) }))
        }),
    );
    serve(app).await
}

/// `srvcs-ln`: `{"value": v}` -> `{"result": v.ln()}`.
#[allow(dead_code)]
async fn spawn_ln() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let v = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            Json(json!({ "result": v.ln() }))
        }),
    );
    serve(app).await
}

/// `srvcs-multiply`: `{"a": x, "b": y}` -> `{"result": x * y}` (integer).
#[allow(dead_code)]
async fn spawn_multiply() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let a = body.get("a").and_then(Value::as_i64).unwrap_or(0);
            let b = body.get("b").and_then(Value::as_i64).unwrap_or(0);
            Json(json!({ "result": a * b }))
        }),
    );
    serve(app).await
}

/// `srvcs-root`: `{"value": v, "n": n}` -> `{"result": v.powf(1 / n)}`.
#[allow(dead_code)]
async fn spawn_root() -> String {
    let app = AxumRouter::new().route(
        "/",
        post(|AxumJson(body): AxumJson<Value>| async move {
            let v = body.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            let n = body.get("n").and_then(Value::as_f64).unwrap_or(1.0);
            Json(json!({ "result": v.powf(1.0 / n) }))
        }),
    );
    serve(app).await
}

/// Spawn a mock returning a fixed status + body (used for error-path tests).
async fn spawn_fixed(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    serve(app).await
}

async fn serve(app: AxumRouter) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn app(reciprocal_url: &str, floatadd_url: &str, floatdivide_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            reciprocal_url: reciprocal_url.to_string(),
            floatadd_url: floatadd_url.to_string(),
            floatdivide_url: floatdivide_url.to_string(),
        },
    )
}

async fn harmonicmean(
    reciprocal_url: &str,
    floatadd_url: &str,
    floatdivide_url: &str,
    values: Value,
) -> (StatusCode, Value) {
    let res = app(reciprocal_url, floatadd_url, floatdivide_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "values": values }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL, DEAD_URL, DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

// --- Standard endpoints. ---

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn metrics_ok() {
    assert_eq!(status_of("/metrics").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let res = app(DEAD_URL, DEAD_URL, DEAD_URL)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.headers().contains_key("x-request-id"),
        "response must carry a generated x-request-id"
    );
}

#[tokio::test]
async fn index_reports_identity() {
    let res = app(DEAD_URL, DEAD_URL, DEAD_URL)
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["service"], "srvcs-harmonicmean");
    assert_eq!(body["concern"], "arithmetic: harmonic mean");
    assert_eq!(
        body["depends_on"],
        json!(["srvcs-reciprocal", "srvcs-floatadd", "srvcs-floatdivide"])
    );
}

// --- Correctness cases, against the computing mocks. ---

#[tokio::test]
async fn harmonicmean_1_2_4() {
    let (r, a, d) = (
        spawn_reciprocal().await,
        spawn_floatadd().await,
        spawn_floatdivide().await,
    );
    let (status, body) = harmonicmean(&r, &a, &d, json!([1, 2, 4])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["values"], json!([1, 2, 4]));
    // sum(1/1 + 1/2 + 1/4) = 1.75; 3 / 1.75 = 1.7142857142857142
    let got = body["result"].as_f64().expect("float result");
    assert!(approx(got, 1.7142857142857142), "got {got}");
}

#[tokio::test]
async fn harmonicmean_single_element_is_itself() {
    let (r, a, d) = (
        spawn_reciprocal().await,
        spawn_floatadd().await,
        spawn_floatdivide().await,
    );
    let (status, body) = harmonicmean(&r, &a, &d, json!([5])).await;
    assert_eq!(status, StatusCode::OK);
    // 1 / (1/5) = 5
    let got = body["result"].as_f64().expect("float result");
    assert!(approx(got, 5.0), "got {got}");
}

#[tokio::test]
async fn harmonicmean_equal_elements_is_that_value() {
    let (r, a, d) = (
        spawn_reciprocal().await,
        spawn_floatadd().await,
        spawn_floatdivide().await,
    );
    let (status, body) = harmonicmean(&r, &a, &d, json!([2, 2, 2, 2])).await;
    assert_eq!(status, StatusCode::OK);
    // sum(4 * 1/2) = 2; 4 / 2 = 2
    let got = body["result"].as_f64().expect("float result");
    assert!(approx(got, 2.0), "got {got}");
}

#[tokio::test]
async fn harmonicmean_fractions() {
    let (r, a, d) = (
        spawn_reciprocal().await,
        spawn_floatadd().await,
        spawn_floatdivide().await,
    );
    let (status, body) = harmonicmean(&r, &a, &d, json!([0.5, 0.25])).await;
    assert_eq!(status, StatusCode::OK);
    // sum(2 + 4) = 6; 2 / 6 = 0.3333333333333333
    let got = body["result"].as_f64().expect("float result");
    assert!(approx(got, 2.0 / 6.0), "got {got}");
}

#[tokio::test]
async fn harmonicmean_empty_is_422_without_calling_deps() {
    // Empty list short-circuits to 422 before any dependency is consulted: point
    // every dependency at a dead port to prove no call is made.
    let (status, _) = harmonicmean(DEAD_URL, DEAD_URL, DEAD_URL, json!([])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// --- Error / degraded paths. ---

#[tokio::test]
async fn degrades_when_reciprocal_unreachable() {
    let (a, d) = (spawn_floatadd().await, spawn_floatdivide().await);
    let (status, body) = harmonicmean(DEAD_URL, &a, &d, json!([1, 2, 4])).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-reciprocal");
}

#[tokio::test]
async fn degrades_when_floatadd_unreachable() {
    let (r, d) = (spawn_reciprocal().await, spawn_floatdivide().await);
    let (status, body) = harmonicmean(&r, DEAD_URL, &d, json!([1, 2, 4])).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-floatadd");
}

#[tokio::test]
async fn degrades_when_floatdivide_unreachable() {
    // reciprocal + floatadd reachable, so the pipeline reaches the floatdivide
    // call.
    let (r, a) = (spawn_reciprocal().await, spawn_floatadd().await);
    let (status, body) = harmonicmean(&r, &a, DEAD_URL, json!([1, 2, 4])).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-floatdivide");
}

#[tokio::test]
async fn forwards_422_from_reciprocal() {
    // reciprocal rejects (e.g. reciprocal of zero) -> forward 422.
    let (a, d) = (spawn_floatadd().await, spawn_floatdivide().await);
    let r = spawn_fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "reciprocal of zero" }),
    )
    .await;
    let (status, _) = harmonicmean(&r, &a, &d, json!([0, 1])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn forwards_422_from_floatdivide() {
    let (r, a) = (spawn_reciprocal().await, spawn_floatadd().await);
    let d = spawn_fixed(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "divide by zero" }),
    )
    .await;
    let (status, _) = harmonicmean(&r, &a, &d, json!([1, 2, 4])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn malformed_reciprocal_result_is_500() {
    // reciprocal answers 200 but with no float result -> contract violation -> 500.
    let (a, d) = (spawn_floatadd().await, spawn_floatdivide().await);
    let r = spawn_fixed(StatusCode::OK, json!({ "result": "not-a-number" })).await;
    let (status, body) = harmonicmean(&r, &a, &d, json!([1, 2, 4])).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["dependency"], "srvcs-reciprocal");
}
