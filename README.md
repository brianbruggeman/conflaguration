# conflaguration

Typed configuration from environment variables, files, and fluent builders.

For the task-oriented deep dive — nesting semantics, the builder's
override-vs-overlay split, a footgun with `#[serde(skip)]` fields, and what
each feature flag actually costs — see [`docs/guide.md`](docs/guide.md).

```sh
cargo add conflaguration --features derive
```

> **Version note — this documentation describes 2.0.** The latest release on
> crates.io is **1.2.0**, and a bare `cargo add` resolves to it. Two field
> attributes documented here do not exist in 1.2.0 and fail at compile time
> with `error: unknown setting attribute`:
>
> - `#[setting(flatten)]` — not recognized at all
> - `#[setting(nested, prefix = "X")]` — `nested` in 1.2.0 accepts only
>   `override_prefix`, not `prefix`
>
> Everything else in this README, including the Quick Start below, works on
> 1.2.0. Until 2.0 is published, depend on the repository directly if you need
> the nesting forms:
>
> ```toml
> conflaguration = { git = "https://github.com/brianbruggeman/conflaguration", features = ["derive"] }
> ```

## Quick start

```rust
use conflaguration::{Settings, Validate, init};

#[derive(Settings, Validate)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(default = 8080)]
    port: u16,

    #[setting(default = "localhost")]
    host: String,

    #[setting(default = false)]
    debug: bool,
}

fn main() -> conflaguration::Result<()> {
    let config: Config = init()?;
    println!("listening on {}:{}", config.host, config.port);
    Ok(())
}
```

This block is itself compiled and run as a doctest (the whole README is
wired into `cargo test --doc --features derive,toml,codegen` via
`src/lib.rs`'s `readme_doctests` module). It is closely based on, but not
identical to, the running doctest on the `Settings` derive macro's own docs
(`src/lib.rs:124-142`): that version uses only `port`/`host` under the
prefix `CONFLAGURATION_TUTORIAL_BASIC` to avoid colliding with other
doctests in the same crate, while this one adds the `debug` field and uses
`APP` since that's the prefix you'd actually write.

## Derive attributes

### Struct-level `#[settings(...)]`

| Attribute                | Effect                                                  |
| ------------------------ | ------------------------------------------------------- |
| `prefix = "APP"`         | Prepend `APP_` to all env var keys                      |
| `resolve_with = "my_fn"` | Default custom parser for fields without typed defaults |

### Field-level `#[setting(...)]`

| Attribute               | Effect                                                 |
| ----------------------- | ------------------------------------------------------ |
| `default`               | Use `T::default()` as fallback when env var is missing |
| `default = value`       | Typed fallback when env var is missing                 |
| `default_str = "str"`   | String fallback, parsed at resolution time             |
| `envs = "KEY"`          | Override the auto-generated env var name               |
| `envs = ["K1", "K2"]`   | Cascade — first set key wins                           |
| `override`              | Use exact key names, ignoring prefix                   |
| `resolve_with = "fn"`   | Custom `fn(&str) -> Result<T, E>` parser               |
| `nested`                | Sub-struct namespaced by `{parent}_{FIELD}`            |
| `nested, prefix = "X"`  | Sub-struct namespaced by `{parent}_X` (rename segment) |
| `nested, override_prefix = "X"` | Sub-struct with absolute prefix `X` (ignores parent) |
| `flatten`               | Merge sub-struct fields into the parent namespace      |
| `skip`                  | Use `Default::default()`, ignore env                   |
| `sensitive`             | Mask value in `ConfigDisplay` output                   |

Conflicting combinations are rejected at compile time:

- `default` + `default_str`
- `skip` + any other attribute
- `nested`/`flatten` + `default`/`default_str`/`resolve_with`/`envs`/`override`/`sensitive`
- `nested` + `flatten`
- `prefix`/`override_prefix` without `nested`
- `prefix` + `override_prefix`

## Nested configuration

A field whose type also derives `Settings` becomes a sub-section. The default
keys its env vars by accumulating the parent prefix and the field name, so the
same sub-struct type can be embedded more than once without collisions:

```rust
use conflaguration::Settings;

#[derive(Settings)]
struct Database {
    #[setting(default = "localhost")]
    host: String,
}

#[derive(Settings)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(nested)]
    primary: Database,   // APP_PRIMARY_HOST

    #[setting(nested, prefix = "RO")]
    replica: Database,   // APP_RO_HOST

    #[setting(flatten)]
    extra: Database,     // APP_HOST  (merged, no segment)

    #[setting(nested, override_prefix = "SHARED")]
    cache: Database,     // SHARED_HOST  (absolute, ignores APP)
}
```

A sub-struct's own `#[settings(prefix = "...")]` only applies when it is
constructed directly as a root (`Database::from_env()`); when embedded, the
embedding field decides the namespace.

## Custom parsing with `resolve_with`

Bypass `FromEnvStr` and parse raw env var strings with your own function:

```rust
use conflaguration::Settings;

fn parse_comma_list(value: &str) -> Result<Vec<String>, std::convert::Infallible> {
    Ok(value.split(',').map(|s| s.trim().to_string()).collect())
}

#[derive(Settings)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(resolve_with = "parse_comma_list")]
    tags: Vec<String>,

    #[setting(resolve_with = "parse_comma_list", default_str = "a,b")]
    features: Vec<String>,
}
```

Apply to all fields at the struct level:

```rust
use conflaguration::Settings;

# fn parse_comma_list(value: &str) -> Result<Vec<String>, std::convert::Infallible> {
#     Ok(value.split(',').map(|s| s.trim().to_string()).collect())
# }
#[derive(Settings)]
#[settings(prefix = "APP", resolve_with = "parse_comma_list")]
struct Config {
    tags: Vec<String>,
    labels: Vec<String>,
}
```

## Builder

Stack sources as layers. The first source builds the value in full; each later
source provides a set of keys and combines them one of two ways:

- **override** — `.file()`, `.mapping()`, `.env()`: replace existing keys, insert new ones.
- **overlay** — `.overlay_file()`, `.overlay_mapping()`: insert keys the value lacks, but keep existing non-null values.

So order is precedence for overrides (last wins per key); overlays fill gaps
without clobbering.

```rust,no_run
# // no_run: base.toml/prod.toml aren't present in a doctest's working directory.
# use conflaguration::{ConfigDisplay, Settings, Validate};
#
# #[derive(Settings, Validate, ConfigDisplay, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "APP")]
# struct Config {
#     #[setting(default = "unset")]
#     host: String,
#     #[setting(default = 0)]
#     port: u16,
# }
#
fn main() -> conflaguration::Result<()> {
#   let fallbacks = Config { host: "fallback-host".to_string(), port: 1 };
    let config: Config = conflaguration::builder()
        .file("base.toml")          // base layer (full)
        .file("prod.toml")          // override: prod's keys win
        .overlay_mapping(fallbacks) // overlay: fill only keys still unset
        .env()                      // override: set env vars win
        .validate()
        .build()?;
    Ok(())
}
```

`.value(T)` is the struct↔fluent flop — seed the chain from an owned value (or
`.value(T::default())` for a defaults base), and `.build()` hands it back:

```rust,no_run
# // no_run: local.toml isn't present in a doctest's working directory.
# use conflaguration::{ConfigDisplay, Settings, Validate};
#
# #[derive(Settings, Validate, ConfigDisplay, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "APP")]
# struct Config {
#     #[setting(default = "unset")]
#     host: String,
#     #[setting(default = 0)]
#     port: u16,
# }
#
fn main() -> conflaguration::Result<()> {
#   let existing = Config { host: "existing-host".to_string(), port: 42 };
    let patched: Config = conflaguration::builder()
        .value(existing)
        .overlay_file("local.toml")
        .build()?;
    Ok(())
}
```

For a value computed from the running state, `.override_with(|c| …)` is the
closure escape hatch. The file/mapping sources require the type to derive
`serde::Serialize` (the merge reads the current value back). Overlay only
fills a key that is genuinely missing or `None` — `0`, `false`, and `""` all
count as present and an overlay will not touch them; see the guide's
[builder section](docs/guide.md#3-the-builders-source-palette) for a worked,
run example with every key's winner printed, and the
[`#[serde(skip)]` footgun](docs/guide.md#4-the-serdeskip-footgun) this merge
strategy creates for nested sub-configs.

## File loading

Requires a format feature: `toml`, `json`, or `yaml`.

```rust,no_run
# // no_run: config.toml isn't present in a doctest's working directory.
# use conflaguration::Settings;
#
# #[derive(Settings, serde::Deserialize)]
# struct Config {
#     #[setting(default = 8080)]
#     port: u16,
# }
#
fn main() -> conflaguration::Result<()> {
    let config: Config = conflaguration::from_file(std::path::Path::new("config.toml"))?;
    let config: Config = conflaguration::from_file_then_env(std::path::Path::new("config.toml"))?;
    Ok(())
}
```

Format is detected by lowercase file extension (`.toml`, `.json`, `.yaml`, `.yml`).
Uppercase or mixed-case extensions are rejected.

## Validation

Derive `Validate` for automatic cascading into `nested` and `flatten` fields, or implement manually:

```rust
# struct Config { port: u16 }
#
impl conflaguration::Validate for Config {
    fn validate(&self) -> conflaguration::Result<()> {
        let mut errors = vec![];
        if self.port == 0 {
            errors.push(conflaguration::ValidationMessage::new("port", "must be > 0"));
        }
        if errors.is_empty() { Ok(()) } else {
            Err(conflaguration::Error::Validation { errors })
        }
    }
}
```

## Display

Derive `ConfigDisplay` to render config with env var keys and sensitive masking:

```rust
use conflaguration::{Settings, ConfigDisplay};

#[derive(Settings, ConfigDisplay)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(default = 8080)]
    port: u16,

    #[setting(sensitive, default = "secret")]
    token: String,
}
// Output:
// port = 8080 (APP_PORT)
// token = *** (APP_TOKEN)
```

## Compile-time constants with a build script

For values that must be `const` (array lengths, capacities, feature flags) but
still want a runtime override, resolve the same `Settings` struct at build time
and bake it into constants with `#[derive(ConfigCodegen)]` and the `codegen`
build-support module (feature `codegen`):

```rust,no_run
# // no_run: write_consts() calls std::env::var_os("OUT_DIR"), which cargo
# // only sets for a real build.rs invocation, never in a doctest process.
// build.rs
use conflaguration::{Settings, ConfigCodegen, codegen};

#[derive(Settings, ConfigCodegen)]
#[settings(prefix = "APP")]
struct Build {
    #[setting(default = 1024)]
    ring_capacity: usize,
}

fn main() -> conflaguration::Result<()> {
    let build = Build::from_env()?;             // defaults + APP_* at build time
    codegen::write_consts(&build, "build.rs")?; // pub const RING_CAPACITY: usize = …
    build.emit_cfg("app");                      // cargo:rustc-cfg=app_* directives
    codegen::rerun_for::<Build>();              // rerun-if-env-changed, derived from the struct
    Ok(())
}
```

```rust,ignore
// ignore: env!("OUT_DIR") is resolved at compile time and only exists when
// this file is compiled as part of a real build.rs run; a doctest process
// never has it, and there is no generated build.rs to include! either.
mod generated { include!(concat!(env!("OUT_DIR"), "/build.rs")); }
type RingBuffer = [u8; generated::RING_CAPACITY]; // build-time const sizes the type
```

`ConfigCodegen` covers flat structs of scalar fields; the rerun list is derived
from the struct's own attributes, so there's no hand-kept env-var list to drift.
See `examples/codegen` (derive-based) and `examples/sizing` (hand-rolled TOML).

## Examples

Runnable crates under `examples/`:

- `codegen` — build-time constants from a `Settings` struct via `ConfigCodegen`
- `sizing` — the same two-tier pattern hand-rolled from a TOML
- `database` / `logging` — reusable sub-config structs
- `http` — composes them with `nested` / `override_prefix`, file + env layering, `override_with`

## Features

| Feature   | Effect                                                       |
| --------- | ------------------------------------------------------------ |
| `derive`  | Enable `#[derive(Settings, Validate, ConfigDisplay, ConfigCodegen)]` |
| `codegen` | `codegen` build-support module (build scripts)               |
| `toml`    | TOML file parsing                                            |
| `json`    | JSON file parsing                                            |
| `yaml`    | YAML file parsing                                            |

## Cost

None of these features are free, and layering the builder isn't either —
measured, not guessed. Full tables, provenance, and reproduce commands are
in the guide's [Cost section](docs/guide.md#cost); this is the summary.

- **Binary size measures what your code calls, not what your `Cargo.toml`
  lists as a dependency.** A crate that depends on conflaguration but never
  calls into it links to a binary 434 bytes *smaller* than a no-dependency
  control, with a byte-identical `__TEXT` section — the linker strips
  anything unreachable. Read the guide's Cost section before treating any
  size number as "the cost of this dependency."
- Feature-flag binary size (release build, five standalone consumers): a
  `derive`-only consumer is **720 bytes** over a `default-features = false`
  one — nearly free. Enabling `toml` on top roughly **doubles** both binary
  size and symbol count (455,824 -> 912,832 bytes). If every value you read
  comes from an env var, skip `toml`.
- At `features = []`, the crate still links `environs`' own dependency on
  `tracing`, unconditionally. "Zero features" is not "zero dependencies":
  5 of the 22 crates in the `--no-default-features` tree are that subtree,
  and `environs` is the only crate pulling it. `environs` 1.3.0 makes it
  optional (default on, dropped with `default-features = false`), but
  conflaguration still depends on `environs = "1.2.0"`, so this does not
  reach consumers until 1.3.0 is published and the dependency is bumped.
  A crate-count drop is not a binary-size win — see the guide's
  [Cost section](docs/guide.md#cost).
- `from_env()` costs roughly 0.8–0.9 µs for a 5-field flat struct and
  3.5–3.6 µs for 20 fields — see the guide's [Cost section](docs/guide.md#cost)
  for the nested-struct number too.
- Each additional `.file()`/`.mapping()`/`.overlay_*()` layer in the builder
  costs from ~15 µs (2 layers) up to ~96-122 µs median (8 layers, high
  run-to-run variance at this depth), linear, but **~89-91% of that is disk
  I/O** (`std::fs::read_to_string` plus TOML parsing per layer), not
  the `serde_value` merge — which costs only ≈1.8-1.9 µs/round trip in
  memory (measured against a matched 6-round-trip in-memory arm; see the
  guide's [Cost section](docs/guide.md#cost) for the round-trip accounting).
  A caching fix for the merge itself was tried and **rolled back** (no time
  win, +11 allocations/chain) — see the guide's
  [Cost section](docs/guide.md#cost) for the full mechanism.
- Reach for the [build-time `codegen` path](docs/guide.md#6-build-time-constants)
  instead of runtime config when a value needs to be a compile-time `const`
  (array length, const generic) — you pay the resolution cost once, in
  `build.rs`, not on every process start.
