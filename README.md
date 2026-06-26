# conflaguration

Typed configuration from environment variables, files, and fluent builders.

```sh
cargo add conflaguration --features derive
```

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

let config: Config = init()?;
```

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

```rust
let config: Config = conflaguration::builder()
    .file("base.toml")          // base layer (full)
    .file("prod.toml")          // override: prod's keys win
    .overlay_mapping(fallbacks) // overlay: fill only keys still unset
    .env()                      // override: set env vars win
    .validate()
    .build()?;
```

`.value(T)` is the struct↔fluent flop — seed the chain from an owned value (or
`.value(T::default())` for a defaults base), and `.build()` hands it back:

```rust
let patched: Config = conflaguration::builder()
    .value(existing)
    .overlay_file("local.toml")
    .build()?;
```

For a value computed from the running state, `.override_with(|c| …)` is the
closure escape hatch. The file/mapping sources require the type to derive
`serde::Serialize` (the merge reads the current value back).

## File loading

Requires a format feature: `toml`, `json`, or `yaml`.

```rust
let config: Config = conflaguration::from_file("config.toml")?;
let config: Config = conflaguration::from_file_then_env("config.toml")?;
```

Format is detected by lowercase file extension (`.toml`, `.json`, `.yaml`, `.yml`).
Uppercase or mixed-case extensions are rejected.

## Validation

Derive `Validate` for automatic cascading into `nested` and `flatten` fields, or implement manually:

```rust
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

```rust
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

```rust
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
