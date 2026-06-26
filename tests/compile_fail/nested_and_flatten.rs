use conflaguration::Settings;

#[derive(Settings)]
struct Inner {
    value: String,
}

#[derive(Settings)]
struct BadConfig {
    #[setting(nested, flatten)]
    inner: Inner,
}

fn main() {}
