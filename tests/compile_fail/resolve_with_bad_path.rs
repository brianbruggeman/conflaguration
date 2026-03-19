use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(resolve_with = "not a valid path!!!")]
    field: String,
}

fn main() {}
