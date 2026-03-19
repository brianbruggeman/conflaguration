use conflaguration::Settings;

struct NotSettings {
    value: String,
}

#[derive(Settings)]
#[settings(prefix = "BAD")]
struct BadConfig {
    #[setting(nested)]
    inner: NotSettings,
}

fn main() {}
