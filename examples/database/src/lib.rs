use conflaguration::ConfigDisplay;
use conflaguration::Settings;

#[derive(Default, Settings, ConfigDisplay)]
#[settings(prefix = "DB")]
pub struct DatabaseConfig {
    #[setting(envs = "DATABASE_HOST", r#override, default = "localhost")]
    pub host: String,

    #[setting(envs = "DATABASE_PORT", r#override, default = 5432)]
    pub port: u16,

    #[setting(envs = "DATABASE_NAME", r#override, default = "app")]
    pub name: String,

    #[setting(envs = "DATABASE_USER", r#override, default = "postgres")]
    pub user: String,

    #[setting(envs = "DATABASE_PASSWORD", r#override, sensitive)]
    pub password: Option<String>,

    #[setting(default = 10)]
    pub pool_max: u32,

    #[setting(default = 30)]
    pub connect_timeout_secs: u64,

    #[setting(default = false)]
    pub ssl: bool,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        let auth = match &self.password {
            Some(pass) => format!("{}:{}", self.user, pass),
            None => self.user.clone(),
        };
        let ssl = if self.ssl { "?sslmode=require" } else { "" };
        format!("postgres://{}@{}:{}/{}{}", auth, self.host, self.port, self.name, ssl)
    }
}
