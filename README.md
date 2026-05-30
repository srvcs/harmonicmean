# srvcs-harmonicmean

The harmonic-mean orchestrator of the srvcs.cloud distributed standard library.

Its single concern: **arithmetic: harmonic mean.** It owns the *control flow* —
composing three primitives — but does no arithmetic of its own. It asks
[`srvcs-reciprocal`](https://github.com/srvcs/reciprocal) for each element's
reciprocal, folds those through
[`srvcs-floatadd`](https://github.com/srvcs/floatadd), then asks
[`srvcs-floatdivide`](https://github.com/srvcs/floatdivide) to divide the element
count by that sum.

```
harmonicmean(values):
    if values is empty:
        return 422            # the harmonic mean of nothing is undefined
    n = len(values)
    sumrecip = 0
    for v in values:
        r = reciprocal(v)
        sumrecip = floatadd(sumrecip, r)
    return floatdivide(n, sumrecip)   # n / sum(1 / v_i)
```

For example, `harmonicmean([1, 2, 4]) == 3 / 1.75 == 1.7142857142857142`.

The result is an `f64` — a JSON number that may be fractional.

Validation is not handled here. This service never calls `srvcs-isnumber`
directly; instead its dependencies validate their own operands, and any `422`
they raise is forwarded verbatim.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Compute the harmonic mean of `values` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"values": [1, 2, 4]}'
# {"values":[1,2,4],"result":1.7142857142857142}
```

Responses:

- `200 {"values": [...], "result": x}` — evaluated; `result` is a float.
- `422` — the list is empty, or a dependency rejected the input (forwarded
  verbatim).
- `500` — a reachable dependency returned a `200` without a float `result`
  (a contract violation).
- `503` — a dependency is unavailable.

## Dependencies

- [`srvcs-reciprocal`](https://github.com/srvcs/reciprocal)
- [`srvcs-floatadd`](https://github.com/srvcs/floatadd)
- [`srvcs-floatdivide`](https://github.com/srvcs/floatdivide)

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_RECIPROCAL_URL` | `http://127.0.0.1:8090` | Base URL of `srvcs-reciprocal` |
| `SRVCS_FLOATADD_URL` | `http://127.0.0.1:8091` | Base URL of `srvcs-floatadd` |
| `SRVCS_FLOATDIVIDE_URL` | `http://127.0.0.1:8092` | Base URL of `srvcs-floatdivide` |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Orchestration tests stand up *computing* mock `srvcs-reciprocal`,
`srvcs-floatadd` and `srvcs-floatdivide` services in-process — they read the
request body and return the real `1 / v` / `a + b` / `a / b`, so the composition
is genuinely exercised against the asserted cases (compared approximately, since
the result is a float). See
[`srvcs/platform`](https://github.com/srvcs/platform) for the shared standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
