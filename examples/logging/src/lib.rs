use conflaguration::ConfigDisplay;
use conflaguration::Settings;

#[derive(Default, Settings, ConfigDisplay)]
#[settings(prefix = "LOG")]
pub struct LoggingConfig {
    #[setting(default = "info")]
    pub level: String,

    #[setting(default = "json")]
    pub format: String,

    #[setting(default = false)]
    pub pretty: bool,

    #[setting(default = true)]
    pub ansi: bool,

    pub file: Option<String>,

    #[setting(default = false)]
    pub span_events: bool,
}
