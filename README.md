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

| Attribute | Effect |
|---|---|
| `prefix = "APP"` | Prepend `APP_` to all env var keys |
| `resolve_with = "my_fn"` | Default custom parser for fields without typed defaults |

### Field-level `#[setting(...)]`

| Attribute | Effect |
|---|---|
| `default` | Use `T::default()` as fallback when env var is missing |
| `default = value` | Typed fallback when env var is missing |
| `default_str = "str"` | String fallback, parsed at resolution time |
| `envs = "KEY"` | Override the auto-generated env var name |
| `envs = ["K1", "K2"]` | Cascade — first set key wins |
| `override` | Use exact key names, ignoring prefix |
| `resolve_with = "fn"` | Custom `fn(&str) -> Result<T, E>` parser |
| `nested` | Delegate to inner struct's `Settings` impl |
| `skip` | Use `Default::default()`, ignore env |
| `sensitive` | Mask value in `ConfigDisplay` output |
| `override_prefix` | Accumulate parent prefix for nested structs |
| `override_prefix = "X"` | Use explicit prefix for nested struct |

Conflicting combinations are rejected at compile time:
- `default` + `default_str`
- `skip` + any other attribute
- `nested` + `default`/`default_str`/`resolve_with`/`envs`/`override`/`sensitive`
- `override_prefix` without `nested`

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

Layer sources with explicit ordering — later sources override earlier ones:

```rust
let config: Config = conflaguration::builder()
    .defaults()
    .file("config.toml")
    .env()
    .apply(|c| c.port = 9090)
    .validate()
    .build()?;
```

## File loading

Requires a format feature: `toml`, `json`, or `yaml`.

```rust
let config: Config = conflaguration::from_file("config.toml")?;
let config: Config = conflaguration::from_file_then_env("config.toml")?;
```

Format is detected by lowercase file extension (`.toml`, `.json`, `.yaml`, `.yml`).
Uppercase or mixed-case extensions are rejected.

## Validation

Derive `Validate` for automatic cascading into nested fields, or implement manually:

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

## Features

| Feature | Effect |
|---|---|
| `derive` | Enable `#[derive(Settings, Validate, ConfigDisplay)]` |
| `toml` | TOML file parsing |
| `json` | JSON file parsing |
| `yaml` | YAML file parsing |
