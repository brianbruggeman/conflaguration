use conflaguration::Settings;

#[derive(Settings)]
#[settings(bogus = "nope")]
struct BadConfig {
    field: u16,
}

fn main() {}
