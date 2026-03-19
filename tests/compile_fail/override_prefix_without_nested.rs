use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(override_prefix = "CUSTOM")]
    field: u16,
}

fn main() {}
