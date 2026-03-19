use conflaguration::Settings;

#[derive(Settings)]
struct BadConfig {
    #[setting(default = 42, default_str = "99")]
    port: u16,
}

fn main() {}
