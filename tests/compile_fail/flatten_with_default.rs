use conflaguration::Settings;

#[derive(Settings)]
struct Inner {
    value: String,
}

#[derive(Settings)]
struct BadConfig {
    #[setting(flatten, default)]
    inner: Inner,
}

fn main() {}
