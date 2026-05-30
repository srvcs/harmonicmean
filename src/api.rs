use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-harmonicmean";
pub const CONCERN: &str = "arithmetic: harmonic mean";
pub const DEPENDS_ON: &[&str] = &["srvcs-reciprocal", "srvcs-floatadd", "srvcs-floatdivide"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub reciprocal_url: String,
    pub floatadd_url: String,
    pub floatdivide_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The list of numbers to take the harmonic mean of. Must be non-empty.
    #[schema(value_type = Object)]
    pub values: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct HarmonicMeanResponse {
    #[schema(value_type = Object)]
    pub values: Vec<Value>,
    pub result: f64,
}

fn ok(values: Vec<Value>, result: f64) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "values": values, "result": result })),
    )
        .into_response()
}

fn invalid(reason: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(json!({ "error": reason })),
    )
        .into_response()
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// A reachable dependency answered `200` but its body lacked a float `result`.
/// That is a contract violation we cannot recover from, so surface a `500`
/// rather than guessing.
fn malformed(dependency: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(
            json!({ "error": "dependency returned a malformed result", "dependency": dependency }),
        ),
    )
        .into_response()
}

/// Call one float dependency at `url` with `body`, returning its `f64` `result`
/// on success, or an error `Response` the caller should surface verbatim:
///
/// - unreachable / non-`200`/`422` -> `503` degraded
/// - `422` -> forwarded `422` (the dependency rejected the input)
/// - `200` without a float `result` -> `500` malformed
async fn ask_f64(url: &str, body: &Value, dependency: &str) -> Result<f64, Response> {
    match client::call(url, body).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => body
            .get("result")
            .and_then(Value::as_f64)
            .ok_or_else(|| malformed(dependency)),
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — compute the harmonic mean of `values` as an `f64`.
///
/// This service owns the *control flow* but delegates every arithmetic step to
/// its dependencies, exactly as specified:
///
/// 1. reject the empty list with `422` (no dependency calls);
/// 2. fold `srvcs-floatadd` over `reciprocal(v)` for each element, starting at
///    `0`, where each reciprocal comes from `srvcs-reciprocal`;
/// 3. ask `srvcs-floatdivide` for `n / sumrecip` — the harmonic mean.
///
/// `n` (the element count) is a trivial local index. If a dependency is
/// unreachable it reports itself degraded (`503`); if a dependency rejects the
/// input it forwards the `422`.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = HarmonicMeanResponse),
        (status = 422, description = "the list is empty, or a dependency rejected the input (forwarded)"),
        (status = 500, description = "a dependency returned a malformed result"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    if req.values.is_empty() {
        return invalid("harmonic mean of empty list");
    }

    let n = req.values.len() as f64;

    // Fold floatadd over reciprocal(values[i]), starting from 0.
    let mut sumrecip: f64 = 0.0;
    for v in &req.values {
        // reciprocal(v)
        let r = match ask_f64(
            &deps.reciprocal_url,
            &json!({ "value": v }),
            "srvcs-reciprocal",
        )
        .await
        {
            Ok(r) => r,
            Err(resp) => return resp,
        };

        // sumrecip = floatadd(sumrecip, r)
        sumrecip = match ask_f64(
            &deps.floatadd_url,
            &json!({ "a": sumrecip, "b": r }),
            "srvcs-floatadd",
        )
        .await
        {
            Ok(s) => s,
            Err(resp) => return resp,
        };
    }

    // result = floatdivide(n, sumrecip)
    let result = match ask_f64(
        &deps.floatdivide_url,
        &json!({ "a": n, "b": sumrecip }),
        "srvcs-floatdivide",
    )
    .await
    {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    ok(req.values, result)
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, HarmonicMeanResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_all_dependencies() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-harmonicmean");
        assert_eq!(info.concern, "arithmetic: harmonic mean");
        assert_eq!(
            info.depends_on,
            vec!["srvcs-reciprocal", "srvcs-floatadd", "srvcs-floatdivide"]
        );
    }

    #[tokio::test]
    async fn empty_list_is_unprocessable_without_calling_deps() {
        let deps = Deps {
            reciprocal_url: "http://127.0.0.1:1".to_string(),
            floatadd_url: "http://127.0.0.1:1".to_string(),
            floatdivide_url: "http://127.0.0.1:1".to_string(),
        };
        let resp = evaluate(State(deps), Json(EvalRequest { values: vec![] })).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
