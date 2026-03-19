use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(skip, default = 42)]
    port: u16,
}

fn main() {}
