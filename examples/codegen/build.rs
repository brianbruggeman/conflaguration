//! Resolve a config at build time and bake it into compile-time constants.
//!
//! The same `#[derive(Settings)]` struct that resolves at runtime also derives
//! `ConfigCodegen`, so the build script reuses it: `from_env()` applies the
//! `#[setting(default)]`s and any `APP_*` vars present at build time, then the
//! `codegen` helpers emit the consts, the `cfg` directives, and the rerun
//! tracking — no hand-written `format!` template, no hand-kept env-var list.

use conflaguration::ConfigCodegen;
use conflaguration::Settings;
use conflaguration::codegen;

#[derive(Settings, conflaguration::ConfigCodegen)]
#[settings(prefix = "APP")]
struct Build {
    #[setting(default = 8)]
    pool_threads: u32,

    #[setting(default = false)]
    tracing: bool,

    #[setting(default = "info")]
    log_level: String,
}

fn main() -> conflaguration::Result<()> {
    let build = Build::from_env()?;
    codegen::write_consts(&build, "build_config.rs")?;
    build.emit_cfg("app");
    codegen::rerun_for::<Build>();
    Ok(())
}
