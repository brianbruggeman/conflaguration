use conflaguration::Settings;

fn noop(value: &str) -> Result<String, std::convert::Infallible> {
    Ok(value.to_string())
}

#[derive(Settings)]
struct BadConfig {
    #[setting(nested, resolve_with = "noop")]
    inner: String,
}

fn main() {}
