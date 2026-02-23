use conflaguration::ConfigDisplay;
use conflaguration::Settings;
use conflaguration::Validate;
use example_database::DatabaseConfig;
use example_logging::LoggingConfig;

#[derive(serde::Deserialize, Settings, ConfigDisplay)]
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
    #[setting(nested)]
    database: DatabaseConfig,

    #[serde(skip)]
    #[setting(nested)]
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

    let config: HttpConfig = conflaguration::builder().file("config.toml").env().validate().build()?;

    println!("{config}");

    Ok(())
}
