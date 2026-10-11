struct LiteralSource;

impl SettingsSource for LiteralSource {
    fn get(&self, key: &str) -> Option<&str> {
        if key == "COLOR" {
            Some("always")
        } else {
            None
        }
    }
}

pub fn read_literal() -> Option<&'static str> {
    LiteralSource.get("COLOR")
}
