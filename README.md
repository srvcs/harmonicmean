# srvcs-harmonicmean

## Name

| Field | Value |
| --- | --- |
| Service | `srvcs-harmonicmean` |
| Slug | `harmonicmean` |
| Repository | `srvcs/harmonicmean` |
| Package | `srvcs-harmonicmean` |
| Kind | `orchestrator` |

## Function

arithmetic: harmonic mean

## Dependencies

| Dependency | Repository |
| --- | --- |
| `srvcs-reciprocal` | [srvcs/reciprocal](https://github.com/srvcs/reciprocal) |
| `srvcs-floatadd` | [srvcs/floatadd](https://github.com/srvcs/floatadd) |
| `srvcs-floatdivide` | [srvcs/floatdivide](https://github.com/srvcs/floatdivide) |

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity |
| `POST` | `/` | Evaluate the service function |
| `GET` | `/healthz` | Liveness probe |
| `GET` | `/readyz` | Readiness probe |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/openapi.json` | OpenAPI document |

## Inputs

| Name | Type | Required |
| --- | --- | --- |
| `values` | `json[]` | yes |

## Outputs

| Name | Type |
| --- | --- |
| `values` | `json[]` |
| `result` | `number` |

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |
| `SRVCS_FLOATADD_URL` | `http://127.0.0.1:8091` | Base URL for srvcs-floatadd |
| `SRVCS_FLOATDIVIDE_URL` | `http://127.0.0.1:8092` | Base URL for srvcs-floatdivide |
| `SRVCS_RECIPROCAL_URL` | `http://127.0.0.1:8090` | Base URL for srvcs-reciprocal |

## Error Behavior

- `422` means the request could not be evaluated for the documented input shape.
- `503` means a required dependency was unavailable or returned an unexpected response.
- Dependency validation errors are forwarded when this service delegates validation.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See the [srvcs service standard](https://github.com/srvcs/platform/blob/main/STANDARD.md) for the full operational contract.

## Metadata

Machine-readable service metadata lives in `srvcs.yaml`. Keep it aligned with this README when the service contract changes.
