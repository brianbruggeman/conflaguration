use conflaguration::ConfigDisplay;
use conflaguration::Settings;
use conflaguration::Validate;
use example_database::DatabaseConfig;
use example_logging::LoggingConfig;

#[derive(serde::Deserialize, serde::Serialize, Settings, ConfigDisplay)]
#[settings(prefix = "HTTP")]
struct HttpConfig {
    #[setting(default = "0.0.0.0")]
    host: String,

    #[setting(envs = "PORT", r#override, default = 3000)]
    port: u16,

    #[setting(default = 30)]
    request_timeout_secs: u64,

    #[setting(default = "10mb")]
    body_limit: String,

    #[setting(default = false)]
    feature_cors: bool,

    #[setting(default = false)]
    feature_compression: bool,

    #[serde(skip)]
    #[setting(nested, override_prefix = "DB")]
    database: DatabaseConfig,

    #[serde(skip)]
    #[setting(nested, override_prefix = "LOG")]
    logging: LoggingConfig,
}

impl Validate for HttpConfig {
    fn validate(&self) -> conflaguration::Result<()> {
        use conflaguration::ValidationMessage;
        let mut errors = vec![];
        if self.port == 0 {
            errors.push(ValidationMessage::new("port", "must be > 0"));
        }
        if self.database.host.is_empty() {
            errors.push(ValidationMessage::new("database.host", "must not be empty"));
        }
        if self.database.pool_max == 0 {
            errors.push(ValidationMessage::new("database.pool_max", "must be > 0"));
        }
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            errors.push(ValidationMessage::new("logging.level", format!("must be one of: {}", valid_levels.join(", "))));
        }
        if errors.is_empty() { Ok(()) } else { Err(conflaguration::Error::Validation { errors }) }
    }
}

fn main() -> conflaguration::Result<()> {
    conflaguration::load()?;

    // the sub-configs are env-only (serde-skipped in the file), so build them
    // from the environment up front — this applies their own #[setting(default)]s.
    let database = DatabaseConfig::from_env()?;
    let logging = LoggingConfig::from_env()?;

    let config: HttpConfig = conflaguration::builder()
        .file("config.toml") // http top-level fields
        .env() // env overrides those
        .override_with(move |cfg: &mut HttpConfig| {
            // inject the env-built sub-configs explicitly
            cfg.database = database;
            cfg.logging = logging;
        })
        .validate()
        .build()?;

    println!("{config}");

    Ok(())
}
