# Verification: environs tracing-optional

Records what was actually run against `~/repos/mine/environs` on 2026-09-15,
after making `tracing` an optional dependency.

An earlier revision of this file described a procedure against a crate that
does not exist (`cargo build --features tracing-support`, a `telemetry`
module, a `tracing-subscriber` dependency) and was paired with a VERIFIED.md
reporting "45 tests". environs has 169. Both were fabricated. This file
records the gates that were run and the numbers they printed.

## What changed

`tracing` became optional, gated behind a `tracing` feature that is ON by
default. environs 1.2.0 is published and logs today, so a minor bump must not
silently delete that output; consumers opt out with `default-features = false`.

The four `tracing::debug!` call sites in `src/dotenv.rs` now go through a
private `debug!` macro pair (one arm forwarding to `::tracing::debug!`, one
no-op arm) so the call sites carry no per-line `cfg`.

Version bumped 1.2.0 -> 1.3.0.

## Gates

All run in `/Users/brianbruggeman/repos/mine/environs`.

| # | command | exit | N |
|---|---|---|---|
| 1 | `cargo build --all-targets` | 0 | |
| 2 | `cargo build --no-default-features --all-targets` | 0 | |
| 3 | `cargo build --all-features --all-targets` | 0 | |
| 4 | `cargo nextest run` | 0 | 169 passed, 0 skipped |
| 5 | `cargo nextest run --no-default-features` | 0 | 169 passed, 0 skipped |
| 6 | `cargo nextest run --all-features` | 0 | 202 passed, 0 skipped |
| 7 | `cargo clippy --all-targets -- -D warnings` | 0 | |
| 8 | `cargo clippy --no-default-features --all-targets -- -D warnings` | 0 | |
| 9 | `cargo clippy --all-features --all-targets -- -D warnings` | 0 | |

Gate 8 is the one that matters for the no-op macro arm: `warnings = "deny"` is
set crate-wide, so any binding left unused once the `debug!` bodies vanish
would have failed it. It did not.

No gate reported N=0.

## The proof, both directions

`cargo tree --no-default-features` — `tracing` appears nowhere:

```
environs v1.3.0
├── falsehoods v1.0.0
└── thiserror v2.0.18
    └── thiserror-impl v2.0.18 (proc-macro)
```

`cargo tree | grep tracing` — still present by default:

```
└── tracing v0.1.44
    ├── tracing-attributes v0.1.31 (proc-macro)
    └── tracing-core v0.1.36
```

## What this buys conflaguration

Measured on conflaguration at `--no-default-features`: 22 unique crates in the
tree, of which 5 come solely from environs' unconditional tracing dependency
(`tracing`, `tracing-attributes`, `tracing-core`, `pin-project-lite`,
`once_cell`). environs is the only crate pulling any of them.

Not yet realised: conflaguration depends on `environs = "1.2.0"` from
crates.io. The drop requires environs 1.3.0 to be published, then
conflaguration's dependency bumped with `default-features = false`.

No binary-size or link-time number is claimed here. The C2 row in
`discipline.md` measured a declared-but-unused dependency at exactly zero
bytes of shipped binary, because the linker already strips unreachable code —
so a crate-count reduction is a compile-graph fact and must not be reported as
a runtime or binary-size win without its own measurement.
