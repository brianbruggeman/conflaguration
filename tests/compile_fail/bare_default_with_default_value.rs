use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(default, default = 42)]
    port: u16,
}

fn main() {}
