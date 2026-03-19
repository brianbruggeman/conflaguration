use conflaguration::Settings;

#[derive(Settings)]
struct Inner {
    value: String,
}

#[derive(Settings)]
struct BadConfig {
    #[setting(nested, r#override)]
    inner: Inner,
}

fn main() {}
