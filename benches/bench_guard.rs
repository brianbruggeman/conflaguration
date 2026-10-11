//! Guards against a flagless `cargo bench` reporting success while measuring
//! nothing. `bench_conflaguration` requires `derive,toml,json`; cargo skips a
//! bench target whose required features are unmet and exits 0, so a plain
//! `cargo bench` used to print `0 measured` and still succeed. This target
//! carries no `required-features`, so cargo never skips it: it fails loudly
//! when the real suite would have been skipped, and is a silent no-op when
//! the real suite actually ran.

const NEEDED_FEATURES: [(&str, bool); 3] = [("derive", cfg!(feature = "derive")), ("toml", cfg!(feature = "toml")), ("json", cfg!(feature = "json"))];

fn main() {
    let missing: Vec<&str> = NEEDED_FEATURES
        .into_iter()
        .filter_map(|(name, enabled)| (!enabled).then_some(name))
        .collect();

    if !missing.is_empty() {
        eprintln!(
            "bench_conflaguration was skipped: missing feature(s) [{}]. \
             run `cargo bench --features derive,toml,json` (or --all-features).",
            missing.join(", ")
        );
        std::process::exit(1);
    }
}
