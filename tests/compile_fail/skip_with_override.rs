use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(skip, r#override)]
    field: u16,
}

fn main() {}
