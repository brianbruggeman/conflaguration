use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(bogus)]
    field: u16,
}

fn main() {}
