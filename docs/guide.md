# conflaguration guide

Task-oriented reference for wiring `conflaguration` into a real project. If
you just want the five-minute version, read the [README](../README.md)
first — this guide goes deeper on the parts that surprise people: nesting,
the builder's override/overlay split, and a footgun in the file-merge path
that will silently reset a field if you don't know about it.

Every code sample below either runs as part of `cargo test --doc` (cited by
its `file:line` in the crate) or is a real, built example under `examples/`
whose output is pasted verbatim (never transcribed from memory) — the
command to reproduce it is given alongside.

## 1. Wiring your first config struct

Derive [`Settings`] to resolve fields from environment variables, and
[`Validate`] to check them after construction. [`init()`] is `from_env()`
then `validate()`, short-circuiting on the first error:

```rust
use conflaguration::{Settings, Validate, init};

#[derive(Settings, Validate)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(default = 8080)]
    port: u16,                    // reads APP_PORT

    #[setting(default = "localhost")]
    host: String,                 // reads APP_HOST
}

fn main() -> conflaguration::Result<()> {
    let config: Config = init()?;
    println!("listening on {}:{}", config.host, config.port);
    Ok(())
}
```

This block is itself compiled and run as a doctest — the whole guide is
wired into `cargo test --doc --features derive,toml,codegen` via
`src/lib.rs`'s `guide_doctests` module. It is closely based on, but not
identical to, the running doctest on the [`Settings`] derive macro's own
docs (`src/lib.rs:124-142`, verified via `cargo test --doc --features
derive`): that version uses the prefix `CONFLAGURATION_TUTORIAL_BASIC`
instead of `APP`, to avoid colliding with other doctests in the same crate.

**The struct-level attribute table**, read directly from the attribute
parser (`crates/conflaguration-derive/src/lib.rs:150-175`):

| `#[settings(...)]` attribute | Accepted forms | Effect |
| --- | --- | --- |
| `prefix = "APP"` | string literal | prepended to every field's env key (`crates/conflaguration-derive/src/lib.rs:158-163`) |
| `resolve_with = "path"` | string literal naming a `fn(&str) -> Result<T, E>` | default parser for every field that has no typed `default`/`default_str` and no field-level `resolve_with` (`crates/conflaguration-derive/src/lib.rs:164-169`, `:419-423`) |

Anything else on `#[settings(...)]` is a hard compile error: `"unknown
settings attribute"` (`crates/conflaguration-derive/src/lib.rs:171`,
exercised by `tests/compile_fail/unknown_struct_attribute.rs`).

**The field-level attribute table**, read directly from
`parse_field_attrs`/`validate_field_attrs`
(`crates/conflaguration-derive/src/lib.rs:188-306`):

| `#[setting(...)]` attribute | Accepted forms | Env key produced |
| --- | --- | --- |
| `default` (bare) | no value | `T::default()` if the key is unset (`:219-227`) |
| `default = value` | typed literal (`8080`, `"localhost"`, `false`, ...) | that literal, typed, if the key is unset (`:219-227`) |
| `default_str = "s"` | string literal | `s` parsed through the field's normal resolver if the key is unset (`:229-234`) |
| `envs = "KEY"` | string literal | replaces the auto-generated `{PREFIX}_{FIELD}` key with exactly `KEY` — still prefix-joined unless combined with `override` (`:211-214`, `:373-379`) |
| `envs = ["K1", "K2"]` | bracketed string list | cascades: first key that is *set* wins (`:177-186`) |
| `override` (also `r#override`) | no value | the key(s) from `envs`, or the field's uppercased name, are used exactly as given — the prefix is not joined on (`:215-218`, `:384-388`) |
| `resolve_with = "path"` | string literal naming `fn(&str) -> Result<T, E>` | bypasses `FromEnvStr`; parses the raw string yourself (`:235-241`) |
| `nested` | no value | field's type must also derive `Settings`; segment = the field's uppercased name unless `prefix`/`override_prefix` says otherwise (`:242-245`, `:393-401`) |
| `nested, prefix = "X"` | string literal, requires `nested` | segment renamed to `X` (relative — still joined onto the parent) (`:250-255`, `:292-294`) |
| `nested, override_prefix = "X"` | string literal, requires `nested` | absolute prefix `X`; ignores everything accumulated so far (`:256-261`, `:292-294`) |
| `flatten` | no value | field's type must also derive `Settings`; no segment — its fields join the parent's own namespace (`:246-249`, `:437-439`) |
| `skip` | no value | `Default::default()`; the env is never consulted, for construction or override (`:262-265`, `:433-435`, `:457-459`) |
| `sensitive` | no value | masked as `***` by `#[derive(ConfigDisplay)]`; has no effect on resolution (`:266-269`) |

**Rejected combinations**, each with its own `tests/compile_fail/*.rs` case
compiled and its `.stderr` snapshot asserted by `trybuild`
(driven from `tests/compile_fail.rs`):

| Combination | Compile error | Test case |
| --- | --- | --- |
| two of `default` / `default = value` / `default_str` | `"only one of default, default = value, or default_str allowed"` (`:284`) | `default_and_default_str.rs`, `bare_default_with_default_value.rs` |
| `nested` + `flatten` | `"nested and flatten are mutually exclusive"` (`:287`) | `nested_and_flatten.rs` |
| `prefix` + `override_prefix` | `"only one of prefix or override_prefix allowed"` (`:290`) | (covered by the derive unit test `prefix_and_override_prefix_conflict_errors`, `crates/conflaguration-derive/src/lib.rs:936-946`) |
| `prefix` or `override_prefix` without `nested` | `"prefix and override_prefix require nested"` (`:293`) | `override_prefix_without_nested.rs` |
| `skip` + any other `#[setting(...)]` attribute | `"skip cannot be combined with other setting attributes"` (`:300`) | `skip_with_default.rs`, `skip_with_override.rs` |
| `nested`/`flatten` + `default`/`default_str`/`resolve_with`/`envs`/`override`/`sensitive` | `"nested and flatten cannot be combined with default, default_str, resolve_with, envs, override, or sensitive"` (`:303`) | `nested_with_default.rs`, `nested_with_resolve_with.rs`, `nested_with_override.rs`, `flatten_with_default.rs` |
| `nested` on a field whose type doesn't derive `Settings` | `E0277: the trait bound "..: Settings" is not satisfied` (ordinary Rust trait-bound error, not a custom message) | `nested_on_non_settings.rs` |
| unknown `#[setting(...)]`/`#[settings(...)]` name | `"unknown setting attribute"` / `"unknown settings attribute"` (`:270`, `:171`) | `unknown_field_attribute.rs`, `unknown_struct_attribute.rs` |
| `Settings`/`Validate`/`ConfigDisplay` on an enum, tuple struct, or unit struct | `"... can only be derived on structs"` / `"only named struct fields supported"` | `settings_on_enum.rs`, `settings_on_tuple_struct.rs`, `settings_on_unit_struct.rs`, `validate_on_enum.rs`, `config_display_on_enum.rs` |

## 2. The nesting semantics

This is the part that surprises people, because all four forms look similar
and produce very different keys. A field whose type also derives `Settings`
becomes a sub-section; the env key is built by **accumulating** the parent's
prefix with a segment as resolution recurses down the struct tree
(`crates/conflaguration-derive/src/lib.rs:393-401`, the `nested_child_prefix`
function that every nested/flatten code path calls):

- **`#[setting(nested)]`** — segment = the field's own uppercased name.
  `APP_PRIMARY_HOST` for a `primary: Database` field under prefix `APP`.
- **`#[setting(nested, prefix = "X")]`** — segment renamed to `X`, still
  joined onto the accumulated parent prefix. `APP_RO_HOST` for a
  `replica: Database` field with `prefix = "RO"`.
- **`#[setting(nested, override_prefix = "X")]`** — **absolute**. `X`
  replaces everything accumulated so far, including the struct's own
  `prefix`. `SHARED_HOST`, not `APP_SHARED_HOST`.
- **`#[setting(flatten)]`** — no segment at all. The inner struct's own
  fields join the parent's *current* namespace directly. `APP_HOST`, using
  the parent's prefix with no extra segment.

A sub-struct's own `#[settings(prefix = "...")]` is used **only** when that
struct is resolved directly as a root (`Database::from_env()`). Once it is
embedded as a nested/flatten field, the embedding field decides the
namespace entirely — the inner struct's own prefix is ignored
(`tests/derive.rs:395-401`, `nested_ignores_inner_struct_own_prefix`).

Here is one struct exercising all four forms, and the **actual** env-key
list, printed by a real build (not transcribed from a design doc):

```rust
use conflaguration::{ConfigDisplay, Settings, Validate, init};

#[derive(Settings, Validate, ConfigDisplay)]
struct Database {
    #[setting(default = "localhost")]
    host: String,
    #[setting(default = 5432)]
    port: u16,
}

#[derive(Settings, Validate, ConfigDisplay)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(nested)]
    primary: Database,

    #[setting(nested, prefix = "RO")]
    replica: Database,

    #[setting(nested, override_prefix = "SHARED")]
    cache: Database,

    #[setting(flatten)]
    extra: Database,
}

fn main() -> conflaguration::Result<()> {
    let config: Config = init()?;
    println!("{config}");
    Ok(())
}
```

Built and run (`cargo run`, workspace-local scratch crate depending on this
path, `--features derive`) with no env vars set, so every value falls back
to its `#[setting(default = ...)]`:

```text
primary:
  host = "localhost" (APP_PRIMARY_HOST)
  port = 5432 (APP_PRIMARY_PORT)
replica:
  host = "localhost" (APP_RO_HOST)
  port = 5432 (APP_RO_PORT)
cache:
  host = "localhost" (SHARED_HOST)
  port = 5432 (SHARED_PORT)
extra:
  host = "localhost" (APP_HOST)
  port = 5432 (APP_PORT)
```

Four fields, same `Database` type, zero key collisions — `primary` and
`replica` differ by segment, `cache` breaks out of the `APP` namespace
entirely, and `extra` shares `APP_HOST`/`APP_PORT` with nothing else in this
struct because `flatten` is the only form that doesn't add its own segment.

This same struct (with different, uniquely-prefixed env keys to avoid
collisions with other doctests) is the second runnable example on the
[`Settings`] derive macro's docs (`src/lib.rs:149-189`), asserted rather
than printed there.

## 3. The builder's source palette

[`init()`] reads exactly one source: the environment. Real deployments
stack several — a checked-in base file, a per-environment overlay, real env
vars, maybe a CLI-flag override on top. [`ConfigBuilder`] composes them.

Every builder method falls into one of three groups
(`src/builder.rs:6-28`):

- **override** — [`.file()`](../src/builder.rs), `.mapping()`, `.env()`,
  `.env_with_prefix()`: **replace** an existing key, **insert** a new one.
  As the *first* source, these build the value from empty.
- **overlay** — `.overlay_file()`, `.overlay_mapping()`: insert a key the
  running value is missing; leave a key that already has a non-null value
  alone.
- **escape hatches** — `.value(T)` seeds the chain from an owned value (the
  struct↔fluent flop: build it however you want, then keep layering), and
  `.override_with(|cfg| ...)` runs a closure against the running value
  unconditionally, last, for anything computed rather than sourced (parsed
  CLI flags, derived fields).

"Empty" for overlay purposes means exactly two things, from the merge
function itself (`src/file.rs:93-97`, `is_empty`): a config value that was
never set — the format-neutral `Value::Unit` a fresh slot starts as — or a
field that deserialized to `Option::None`. **`0`, `false`, and `""` are all
real, present values** and an overlay will never touch them. This trips
people up: `.overlay_file("defaults.toml")` will not "fill in" a `count: 0`
or a `debug: false` you already have, because those aren't empty — only a
genuinely-missing key or an explicit `None` is.

Errors short-circuit: once any source in the chain fails, every later
source is skipped and `.build()` returns that first error. `.value()`,
`.env()`, `.env_with_prefix()`, and the internal file/mapping merges all
match `Some(Err(_)) => self` as their first arm (`src/builder.rs:69`, `:84`,
`:99`, `:219`, `:231`); `.override_with()` and `.validate()` reach the same
result through a catch-all `other => Self { state: other }` arm
(`src/builder.rs:119`, `:133`) that passes an error or empty state through
untouched.

Here is a layered chain — two files (override), a mapping (overlay), and an
env var (override) — with the actual per-key winner, run for real:

```rust
use conflaguration::{ConfigDisplay, Settings, Validate};

#[derive(Settings, Validate, ConfigDisplay, serde::Serialize, serde::Deserialize)]
#[settings(prefix = "APP")]
struct Config {
    #[setting(default = "unset")]
    host: String,
    #[setting(default = 0)]
    port: u16,
    #[setting(default = false)]
    debug: bool,
    // skip: never touched by .env()/.env_with_prefix(); only a file/mapping
    // layer below can set it.
    #[setting(skip)]
    region: Option<String>,
}

fn main() -> conflaguration::Result<()> {
    // set up base.toml/prod.toml for real, so this doctest exercises the
    // actual merge instead of asserting a story about it.
#   let dir = tempfile::tempdir().expect("tempdir");
#   std::fs::write(dir.path().join("base.toml"), "host = \"localhost\"\nport = 8080\ndebug = false\n").expect("write base.toml");
#   std::fs::write(dir.path().join("prod.toml"), "port = 9090\n").expect("write prod.toml");
#   std::env::set_current_dir(dir.path()).expect("chdir into tempdir");
    unsafe { std::env::set_var("APP_DEBUG", "true") };

    let config: Config = conflaguration::builder()
        .file("base.toml")   // base layer (full): host, port, debug all set here
        .file("prod.toml")   // override: prod's port (9090) replaces base's (8080)
        .overlay_mapping(Config {
            host: "fallback-host".to_string(),        // ignored: host already set
            port: 1,                                   // ignored: port already set
            debug: true,                                // ignored: debug already set
            region: Some("from-overlay".to_string()),   // applied: region was still None
        })
        .env()   // override: APP_DEBUG=true wins over the file value
        .validate()
        .build()?;

#   assert_eq!(config.host, "localhost");
#   assert_eq!(config.port, 9090);
#   assert!(config.debug);
#   assert_eq!(config.region.as_deref(), Some("from-overlay"));
    println!("{config}");
#   unsafe { std::env::remove_var("APP_DEBUG") };
    Ok(())
}
```

With `base.toml` = `host = "localhost"`, `port = 8080`, `debug = false` and
`prod.toml` = `port = 9090`, the printed output is:

```text
host = "localhost" (APP_HOST)
port = 9090 (APP_PORT)
debug = true (APP_DEBUG)
region = Some("from-overlay") (skipped)
```

| field | value | won by |
| --- | --- | --- |
| `host` | `"localhost"` | `base.toml` — never overridden or overlaid |
| `port` | `9090` | `prod.toml` — the second `.file()` call, override mode |
| `debug` | `true` | `.env()` — override mode, `APP_DEBUG=true` |
| `region` | `Some("from-overlay")` | `.overlay_mapping()` — the only key that was still empty when it ran |

`.file()`/`.mapping()`/`.overlay_file()`/`.overlay_mapping()` all require
the config type to also implement `serde::Serialize` (the merge reads the
current value back out through `serde_value` to combine it with the
incoming layer — see the footgun below) and are gated on the `toml`,
`json`, or `yaml` feature (any one; the format only matters for `.file()`,
since `.mapping()` never touches a file extension).

## 4. The `#[serde(skip)]` footgun

The builder's file/mapping merge round-trips the running value through
`serde_value`: `serde_value::to_value(base)` then, after merging in the
incoming layer, `T::deserialize(base_value)`
(`src/file.rs:135-138`, `:147-150`). That round-trip is driven entirely by
your `#[derive(Serialize, Deserialize)]` impl — which means **any field
marked `#[serde(skip)]` is silently reset to `Default::default()` on every
single `.file()`/`.mapping()`/`.overlay_file()`/`.overlay_mapping()` call**,
because `serde`'s generated `Serialize` never writes a skipped field out,
so the merge has nothing to read back for it.

This matters because `#[serde(skip)]` is exactly the annotation you reach
for on a nested sub-config you're resolving from its own env vars instead
of the parent file — and that is precisely the field this footgun destroys.
Verified with a minimal repro (scratch crate, `#[setting(nested,
override_prefix = "DB")] #[serde(skip)] database: Database`, `DB_HOST`
set in the environment):

```text
after override_with, before second file layer: database.host = "real-db-host"
after a second .file() call:                    database.host = ""
```

The value survives the first builder chain (injected via `.override_with`
after the file layer), then a **later, separate** `.file()` call
re-serializes the whole struct, drops the skipped field (never written),
and deserializes it back as `Database::default()` — an empty string, not
the real value that was there a line earlier.

**The workaround**, and it is exactly what `examples/http` does — build the
skipped sub-configs from their own env vars, and inject them with
`.override_with()` as the **last** step in the chain, after every
file/mapping layer has already run:

```rust
# use conflaguration::{Settings, Validate};
#
# #[derive(Default, Settings, Validate, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "DB")]
# struct DatabaseConfig {
#     #[setting(envs = "DATABASE_HOST", r#override, default = "localhost")]
#     host: String,
# }
#
# #[derive(Default, Settings, Validate, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "LOG")]
# struct LoggingConfig {
#     #[setting(default = "info")]
#     level: String,
# }
#
# #[derive(Settings, Validate, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "HTTP")]
# struct HttpConfig {
// examples/http/src/main.rs:28-34
#[serde(skip)]
#[setting(nested, override_prefix = "DB")]
database: DatabaseConfig,

#[serde(skip)]
#[setting(nested, override_prefix = "LOG")]
logging: LoggingConfig,
# }
```

The sub-config types are elided above, in two ways worth knowing before you
copy the snippet.

**They are not bare.** `DatabaseConfig` carries `#[settings(prefix = "DB")]`
and its `host` field carries
`#[setting(envs = "DATABASE_HOST", r#override, default = "localhost")]`
(`examples/database/src/lib.rs:5-8`). That `envs` + `r#override` pair is
where the `DATABASE_HOST` key in the sample output below comes from — not
from the embedding field's `override_prefix = "DB"`, which only applies once
`DatabaseConfig` is embedded, never when `DatabaseConfig::from_env()` is
called directly. Copy the snippet without those attributes and your root key
is a bare `HOST`.

**They are shortened.** The real `DatabaseConfig` has eight fields — `host`,
`port`, `name`, `user`, `password`, `pool_max`, `connect_timeout_secs`,
`ssl` — of which the first five carry their own `envs = "DATABASE_*"` plus
`r#override` (`examples/database/src/lib.rs:5-29`). Only `host` is kept
above, to keep the `#[serde(skip)]` mechanism visible. That is why the
sample output below lists `DATABASE_PORT` and other keys the snippet does
not declare: the output is from running the real example, not this
reduction.

```rust,no_run
# // no_run: config.toml isn't present in a doctest's working directory.
# use conflaguration::{Settings, Validate};
#
# #[derive(Default, Settings, Validate, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "DB")]
# struct DatabaseConfig {
#     #[setting(envs = "DATABASE_HOST", r#override, default = "localhost")]
#     host: String,
# }
#
# #[derive(Default, Settings, Validate, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "LOG")]
# struct LoggingConfig {
#     #[setting(default = "info")]
#     level: String,
# }
#
# #[derive(Settings, Validate, serde::Serialize, serde::Deserialize)]
# #[settings(prefix = "HTTP")]
# struct HttpConfig {
#     #[serde(skip)]
#     #[setting(nested, override_prefix = "DB")]
#     database: DatabaseConfig,
#     #[serde(skip)]
#     #[setting(nested, override_prefix = "LOG")]
#     logging: LoggingConfig,
# }
#
# fn main() -> conflaguration::Result<()> {
// examples/http/src/main.rs:61-75
// the sub-configs are env-only (serde-skipped in the file), so build them
// from the environment up front — this applies their own #[setting(default)]s.
let database = DatabaseConfig::from_env()?;
let logging = LoggingConfig::from_env()?;

let config: HttpConfig = conflaguration::builder()
    .file("config.toml") // http top-level fields
    .env()                // env overrides those
    .override_with(move |cfg: &mut HttpConfig| {
        // inject the env-built sub-configs explicitly
        cfg.database = database;
        cfg.logging = logging;
    })
    .validate()
    .build()?;
#   let _ = config;
#   Ok(())
# }
```

Run for real (`cargo run -p example-http` from `examples/http/`, no env
overrides set):

```text
host = "0.0.0.0" (HTTP_HOST)
port = 3000 (PORT)
request_timeout_secs = 30 (HTTP_REQUEST_TIMEOUT_SECS)
body_limit = "10mb" (HTTP_BODY_LIMIT)
feature_cors = false (HTTP_FEATURE_CORS)
feature_compression = false (HTTP_FEATURE_COMPRESSION)
database:
  host = "localhost" (DATABASE_HOST)
  port = 5432 (DATABASE_PORT)
  ...
logging:
  level = "info" (LOG_LEVEL)
  ...
```

Both sub-configs are fully populated because `.override_with()` runs after
every file/env layer, not before it — there is no later `.file()` call left
to reset them.

The general rule: if a field is `#[serde(skip)]`, treat it as **write-once,
last** in the chain. Never call `.file()`/`.mapping()`/`.overlay_*()`
*after* you've set it, or it reverts.

## 5. Validation

`Validate` is a plain trait — `fn validate(&self) -> conflaguration::Result<()>`
— and `#[derive(Validate)]` gives you a cascading implementation: it
generates a no-op `Ok(())` unless the struct has `nested`/`flatten` fields,
in which case it calls `validate()` on each of those and collects every
failure, prefixing each [`ValidationMessage`]'s path with the field name
(`crates/conflaguration-derive/src/lib.rs:531-593`,
`src/lib.rs`'s `ValidationMessage::prepend_path`).

For your own rules — the derive macro can't know that `port == 0` is
invalid — implement it by hand and merge in the errors:

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

[`init()`] calls `from_env()` then `validate()` and returns whichever fails
first. The builder's `.validate()` method does the same thing mid-chain,
converting an `Ok` state to an `Err` state
(`src/builder.rs:124-135`) — everything after it in the chain is then
skipped, same as any other error.

## 6. Build-time constants

The two-tier pattern: resolve a `Settings` struct **at build time**, bake
the result into `pub const`s a build script writes into `OUT_DIR`, and get
a runtime override surface on the *same* struct for free. This is the
single most valuable thing this crate does for a caller on a hot path who
wants both a compile-time-foldable constant (to size an array, pick a SIMD
width, whatever the optimizer needs a `const` for) *and* the ability to
override it without a recompile.

`#[derive(ConfigCodegen)]` supports flat structs of scalar fields only —
`nested`/`flatten` fields are a compile error (`"ConfigCodegen does not
support nested or flatten fields"`,
`crates/conflaguration-derive/src/lib.rs:771`, exercised by
`config_codegen_rejects_nested`, `crates/conflaguration-derive/src/lib.rs:923-933`)
— and `skip` fields are simply omitted from the generated module
(`crates/conflaguration-derive/src/lib.rs:773-775`).

`examples/codegen` is the derive-based worked example
(`examples/codegen/build.rs`):

```rust,no_run
# // no_run: write_consts() calls std::env::var_os("OUT_DIR"), which cargo
# // only sets for a real build.rs invocation, never in a doctest process.
// build.rs
use conflaguration::{ConfigCodegen, Settings, codegen};

#[derive(Settings, ConfigCodegen)]
#[settings(prefix = "APP")]
struct Build {
    #[setting(default = 8)]
    pool_threads: u32,
    #[setting(default = false)]
    tracing: bool,
    #[setting(default = "info")]
    log_level: String,
}

fn main() -> conflaguration::Result<()> {
    let build = Build::from_env()?;                // defaults + APP_* at build time
    codegen::write_consts(&build, "build_config.rs")?;
    build.emit_cfg("app");                          // cargo:rustc-cfg=app_* directives
    codegen::rerun_for::<Build>();                  // rerun-if-env-changed, derived from Build
    Ok(())
}
```

`examples/codegen/src/main.rs` then just `include!`s the generated file and
reads the consts. Built and run (`cargo run -p example-codegen`, no env
overrides):

```text
pool_threads = 8 (compile-time const)
tracing      = false
log_level    = "info"
```

`codegen::rerun_for::<Build>()` (`src/lib.rs:624-630`) walks `Build`'s own
`env_keys()` — derived from its `#[setting(...)]` attributes at macro
expansion time — and emits one `cargo:rerun-if-env-changed` per key. There
is no hand-kept env-var list to drift out of sync with the struct.

`examples/sizing` is the same two-tier shape hand-rolled from a plain TOML
file instead of a derived struct — useful when the build-time values need
validation the derive can't express (`examples/sizing/build.rs:34-42`
enforces `ring.capacity` is a power of two, and every value additionally
takes a `SIZING_<SECTION>_<KEY>` build-time env override,
`examples/sizing/build.rs:24-32`). Built and run (`cargo run -p
example-sizing`, no env overrides):

```text
ring capacity  = 1024 (compile-time const)
flush interval = 500us (runtime override, default = const)
```

`main.rs` (`examples/sizing/src/main.rs:12-24`) uses `sized::RING_CAPACITY`
as a `const` array length — genuinely impossible with a pure runtime
value — while `flush_micros` still has a live runtime override
(`SIZING_PUMP_FLUSH_INTERVAL_MICROS`) layered on top of the same
build-time default via `conflaguration::resolve_or`.

**Reach for this instead of plain runtime config when:** the value sizes a
type or controls a compile-time decision (array length, const generic,
`cfg`-gated code path) *and* you still want it overridable without
recompiling for the common case (tuning a running system). If you never
need the `const`, skip this entirely and just use `Settings::from_env()`.

## 7. Errors

Every variant of [`Error`] (`src/lib.rs`, `#[non_exhaustive]`), what fires
it, and what to do:

| Variant | Fires when | What to do |
| --- | --- | --- |
| `Error::Env(environs::Error)` | any env lookup or parse fails — missing key, or a value that doesn't parse into the field's type | match the inner `environs::Error`: `NotFound { keys, location }`, `Parse { key, expected, got, source, location }`, `DotenvLoad { path, source }`, `DotenvParse { path, line, message }` (`environs` crate, `src/error.rs:26-44`, not part of this crate — cited for completeness since you'll see it in a match) |
| `Error::Validation { errors: Vec<ValidationMessage> }` | your (or the derived) `Validate::validate()` returned failures | iterate `errors`; each has a dotted `path` and a `message` |
| `Error::Io(std::io::Error)` | reading a config file from disk failed | usual `io::Error` handling — permissions, missing file, etc. |
| `Error::UnsupportedFormat(String)` | a file extension isn't `.toml`/`.json`/`.yaml`/`.yml` (case-sensitive — lowercase only), or the matching feature isn't enabled | check the extension is lowercase and the right Cargo feature (`toml`/`json`/`yaml`) is on |
| `Error::NoSource` | `.build()` was called on a builder with no source configured at all | add at least one of `.value()`/`.file()`/`.mapping()`/`.env()` before `.build()` |
| `Error::Toml(toml::de::Error)` (feature `toml`) | TOML deserialization failed | malformed TOML — the inner error has the line/column |
| `Error::Json(serde_json::Error)` (feature `json`) | JSON deserialization failed | same, for JSON |
| `Error::Yaml(serde_yaml::Error)` (feature `yaml`) | YAML deserialization failed | same, for YAML |
| `Error::ValueSerialize(serde_value::SerializerError)` / `Error::ValueDeserialize(serde_value::DeserializerError)` (any file format feature) | the builder's internal `serde_value` merge round-trip failed | almost always means your `Serialize`/`Deserialize` impls don't round-trip cleanly through a generic value — check for custom (de)serialize impls that assume a specific format |

`Error` is `#[non_exhaustive]` (`src/lib.rs:338`) — a `match` needs a
wildcard arm, since new variants can be added without a breaking change.

## Cost

Numbers below are cited, not re-measured. Feature-flag and binary-size
numbers come from `linkweight-rows.md`, a five-consumer-binary measurement
pass with full provenance (snapshot hash, `git diff --stat`, a reproduce
command per row); runtime costs come from `docs/discipline.md`. Nothing
here is a benchmark dump for its own sake; each number is here because it
should change a decision you're about to make.

**Read this before the size table, not after — it changes what the table
means.** A crate that declares a dependency on conflaguration in
`Cargo.toml` but never calls into it (`degenerate`, below) linked to a
**smaller** binary than a control program with no dependency on
conflaguration at all — 433,936 B vs 434,368 B, a 434-byte difference, with
a byte-for-byte identical `__TEXT` section (278,528 B both) and a symbol
count difference (866 vs 861) explained by embedded panic-location strings
scaling with the binary's own crate-name length, not by any linked
conflaguration code (`linkweight-rows.md:397-411`). The linker dead-code-
eliminates anything your program never reaches. **The binary-size numbers
below measure what your code actually calls, not what your `Cargo.toml`
lists as a dependency.** Read the "what to act on" bullets under the table
before treating any of these bytes as "the cost of depending on
conflaguration."

### Feature-flag cost, measured with five standalone consumer binaries

Five minimal `[workspace]`-rooted binaries, one per feature combination
(plus the adversarial `degenerate` control), each built with `cargo build
--release`, measured with `ls -l` / `size` / `nm | wc -l`
(`linkweight-rows.md:98-105` describes what each `main` does;
`linkweight-rows.md:176-182` is the measurement table):

| variant | dependency | binary bytes | `nm` symbols |
| --- | --- | --- | --- |
| `control` | none (`std::env::var` + manual `u16` parse) | 434,368 | 866 |
| `empty` | `conflaguration`, `default-features = false` | 455,104 | 901 |
| `derive` | `+ features = ["derive"]` | 455,824 | 907 |
| `derive-toml` | `+ features = ["derive", "toml"]` | 912,832 | 1,961 |
| `degenerate` | same dependency as `empty`, never called | 433,936 | 861 |

Every env-reading variant was smoke-tested with `APP_PORT=1234`, and
`derive-toml` (no env var set) read `port = 9090` from a real
`config.toml`, proving its file-parse path actually executed rather than
merely linking (`linkweight-rows.md:106-109`).

**What to act on:**

- `empty` over `control` costs 20,736 bytes for calling
  `resolve_with_or_str` once — call this the floor cost of using
  conflaguration's env-only path at all, not a per-feature tax
  (`linkweight-rows.md:382-388`).
- `derive` over `empty` is **720 bytes**. The `#[derive(Settings)]` macro
  itself is nearly free at the linked-binary level — if you only need
  typed env vars, enabling `derive` is not the expensive part.
- `toml` roughly **doubles** both binary size and symbol count over
  `derive` alone (455,824 -> 912,832 bytes, 907 -> 1,961 symbols). `serde`
  + `toml` + `serde-value`'s merge machinery is what costs, not the derive
  macro. Do not enable `toml` "just in case" if every value you read comes
  from an env var.
- Compile-graph cost is separate from binary size and is paid even for
  code you never call: `empty` pulls in 14 crates over `control`'s 0,
  `derive` adds 1 more (the proc-macro crate itself), and `toml` adds 16
  more on top of `derive` — `serde`, `serde_derive`, `toml`, `toml_edit`,
  `winnow`, `indexmap`, and others (`linkweight-rows.md:126-153`). That's
  compile time and dependency-audit surface, independent of whether the
  linker keeps any of it.

### `tracing` is unconditional on the published release; the opt-out exists upstream but is not yet wired

At `features = []`, conflaguration's own `Cargo.toml` never names
`tracing` (verified directly, `Cargo.toml:16-22`) — it arrives entirely
through `environs = "1.2.0"`, a plain, non-optional dependency
(`Cargo.toml:16`), which itself pulls `tracing`, `tracing-core`,
`tracing-attributes`, `pin-project-lite`, and `once_cell` unconditionally
(`cargo tree -i tracing` traces `tracing -> environs -> conflaguration`,
`linkweight-rows.md:268-273`). "Zero features" is not "zero dependencies":
5 of the 14 crates a zero-feature consumer gains are `tracing`'s own
subtree (`linkweight-rows.md:306-315`). Measured again against the current
tree: 5 of the 22 crates at `--no-default-features` are that subtree, and
`environs` is the only crate pulling any of them.

**This is fixed upstream and not yet reachable from here.** `environs`
1.3.0 makes `tracing` optional behind a `tracing` feature that is on by
default, so the dependency is dropped with `default-features = false`; the
proof is that `cargo tree --no-default-features` in `environs` contains
only `falsehoods` and `thiserror`, while a default `cargo tree` still shows
`tracing v0.1.44`. See `verify.md` for the nine gates and both tree
outputs. conflaguration still depends on `environs = "1.2.0"` from
crates.io, so nothing changes for a consumer until 1.3.0 is published and
that dependency is bumped. Do not read the crate-count drop as a
binary-size or runtime win without measuring it: the C2 row in
`discipline.md` measured a declared-but-unused dependency at exactly zero
shipped bytes, because the linker already strips what is unreachable.

Whether that subtree's code actually runs on the path this crate uses is a
separate question, and the evidence — not proof — says no: `environs`'s
only `tracing::debug!` calls live in its `dotenv.rs` (dotenv-file loading),
not in `resolve.rs` (the function `resolve_with_or_str` actually calls),
and a symbol-table grep of the `empty` binary for `tracing` returns zero
matches (`linkweight-rows.md:327-338`). That's evidence, not proof —
release-mode inlining can fold a callee's body into its caller without
leaving a symbol carrying the callee's crate name, so a true zero can't be
certified this way.

### Build time, and a caveat on the noise in two of these numbers

Clean-build wall time, sccache disabled, fresh `CARGO_TARGET_DIR` per run,
3 runs (`linkweight-rows.md:213-219`):

| variant | median clean build | spread (max - min) |
| --- | --- | --- |
| `control` | 0.18 s | 0.04 s |
| `empty` | 3.66 s | 0.13 s |
| `derive` | 4.09 s | 0.26 s |
| `derive-toml` | 8.47 s | **2.54 s** |
| `degenerate` | 4.55 s | 1.52 s |

**Take the spread seriously before the median for the last two rows.**
`derive-toml`'s 2.54 s spread is roughly 30% of its own median and
`degenerate`'s 1.52 s is about 33%, both far noisier than `control`'s
near-zero spread. This isn't unexplained flakiness: every noisy run's log
contains `Blocking waiting for file lock on package cache` — cargo's
registry-index lock, held process-wide — and two other agents were
building and editing this same repository concurrently while these numbers
were captured (`linkweight-rows.md:221-232`). The binary-size and
crate-count numbers above don't depend on wall time and aren't affected by
this; treat `derive-toml`'s and `degenerate`'s clean-build numbers as
noisy-but-real, not as a baseline to gate your own CI on.

Incremental rebuilds (touch `src/main.rs`, persistent target dir) are
cheaper and steadier — `control` 0.13 s, `empty` 0.21 s, `derive` 0.25 s,
`derive-toml` 0.55-0.68 s (`linkweight-rows.md:245-251`) — and are what you
actually pay while iterating, not the clean-build numbers above.

### Runtime cost of using the crate

**`from_env()` scaling.** From the C1 downstream-effect rows
(`docs/discipline.md:70-77`, post-fix numbers, median of 3 runs): a 5-field
flat struct resolves in ~833–875 ns; a 20-field flat struct in ~3.48–3.6
µs; a 3-level-deep nested struct in ~773–798 ns. Cost scales with field
count (more fields, more resolves) and with nesting depth (each nesting
level adds a `join_key` call to build the child prefix) — roughly linear in
both, not measured beyond these three shapes.

**Builder merge cost is dominated by disk I/O, not by `serde_value`.**
Each additional `.file()`/`.mapping()`/`.overlay_*()` layer in a
file-based chain costs, at 2/3/4/6/8 layers respectively: ~15.5-16.6 µs (54
allocs), ~29.4-33.7 µs (93 allocs), ~44.8-52.7 µs (135 allocs, CoV exceeds
the 5% report-as-range bar at this depth), ~70.3-85.4 µs (219 allocs), and
~96.3-121.7 µs median across 3 runs (CoV ~12.7%, over the report-as-range
bar) — this is today's shipped behavior, unchanged by anything in this
section (`docs/discipline.md`'s C3 row). The original diagnosis of this
cost attributed nearly all of it to the `serde_value` round-trip; a later
pass isolated the round-trip cost via `builder_merge_8_layer_in_memory`
(`.mapping()`/`.overlay_mapping()` instead of `.file()`, zero filesystem
syscalls) and reported a ~90%/10% disk-I/O/round-trip split. **That
isolation arm was itself miscounted and has been corrected.** The file arm
seeds with `.file()` (a one-shot deserialize, no merge — `src/builder.rs:
220-221`) then performs 6 genuine merge round trips
(`merge_file`, `src/builder.rs:223-224`); the in-memory arm previously
seeded with `.value()` (a bare move, zero serialize/deserialize) and
performed only 5 round trips — one short, and missing the first layer's
one-shot deserialize entirely. It now seeds with `.mapping()` (the same
one-shot-deserialize shape as `.file()`'s first call, `src/builder.rs:
232-233`) and carries a 7th layer, so both arms run 1 one-shot deserialize
+ 6 genuine `merge_mapping`/`merge_file` round trips
(`src/builder.rs:223-224,235-236`), differing only in disk I/O.

Re-measured (3 runs): the in-memory arm now totals **10.99-11.48 µs median**
(dividing by its 6 round trips gives **~1.8-1.9 µs/round trip**, matching the
original per-round-trip estimate, though that division still folds the first
layer's one-shot-deserialize cost in as noise rather than isolating it). Per
run, the disk-I/O share (file-arm total minus in-memory total, over the
file-arm total) is 88.6% / 89.0% / 90.6%. **Corrected split: ~89-91% disk
I/O** (`std::fs::read_to_string` plus TOML parsing per layer,
`src/file.rs:29,58`) **, ~9-11% the `serde_value` round trip** — close to
the originally reported 90%/10%, but that number was backed by a mismatched
6-vs-5-round-trip comparison; it is now backed by a matched 6-vs-6 one.

That reframes the guidance: if you're stacking `.file()` layers at process
startup, the cost you're paying is mostly opening and reading files off
disk — unavoidable, and paid once. If you're merging in-memory sources
(`.value()`/`.mapping()`) or re-merging in a hot-reload loop, the
`serde_value` round trip itself is the relevant cost, at roughly 1.8-1.9 µs
per round trip, and caching file *contents* (not the merge result) is the
lever if you reload frequently.

**A caching fix for the round-trip was tried and rolled back — this is not
"in progress."** The obvious next step — cache the merged
`serde_value::Value` on `ConfigBuilder<T>` so a chained layer reuses it
instead of re-deriving it from `T` via `serde_value::to_value` — was
implemented and benched at 2/3/4/6/8 layers. Result: every "after" median
landed inside the "before" noise band at every depth (no time win), and
allocations rose by a flat **+11 per chain** at every depth with at least
one round trip (`c3-discipline-rows.md:105-117`). Mechanism: `serde_value`
ships only a by-value `Deserializer` impl, no `&Value` impl
(`c3-discipline-rows.md:86-90`), so keeping the cached value alive across
the consuming deserialize required a `Value::clone()` per layer — and
cloning the same `BTreeMap<Value, Value>` tree costs the same as building
it fresh via `to_value`, so the tweak paid for a clone without recouping
anything (`c3-discipline-rows.md:119-134`). `src/builder.rs` and
`src/file.rs` are byte-identical to their pre-attempt state
(`c3-discipline-rows.md:136-143,225-231`) — "cloning is cheaper than
serializing" was a design premise, and the bench falsified it, not an
argument. The only path that would actually cut this cost is a
non-consuming `Deserializer` over `&Value`, which doesn't exist in
`serde_value` today and is a nontrivial, error-identity-risky change to
write by hand — named as the correctly-scoped next step, not attempted
this session (`c3-discipline-rows.md:241-251`).

**`join_key` (the per-field, per-nesting-level string join underneath
every `from_env()`)** costs one allocation, ~13 ns, after the fix landed
this session (`docs/discipline.md:52-101`, C1) — down from two allocator
operations (~57 ns) before. This is already folded into the `from_env()`
numbers above; called out separately because it's the one hot-path
primitive in the crate with a stated zero-allocation-above-one budget
(`docs/discipline.md:23-26`).

**When to reach for build-time `codegen` instead of runtime config:** when
the value needs to be a compile-time `const` for the optimizer to act on
(array length, const generic, SIMD width) — see [section 6](#6-build-time-constants).
The runtime resolution cost above doesn't apply to a build-time const at
all; you pay the `from_env()` cost once, in `build.rs`, not on every
process start.

[`Settings`]: ../src/lib.rs
[`Validate`]: ../src/lib.rs
[`init()`]: ../src/lib.rs
[`ConfigBuilder`]: ../src/builder.rs
[`Error`]: ../src/lib.rs
[`ValidationMessage`]: ../src/lib.rs
