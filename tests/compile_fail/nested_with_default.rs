use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(nested, default = 42)]
    inner: u16,
}

fn main() {}
