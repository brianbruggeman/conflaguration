//! Consumes the constants baked by `build.rs`. These are compile-time values;
//! a runtime `Settings` struct could layer `APP_*` overrides on top of them.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/build_config.rs"));
}

fn main() {
    println!("pool_threads = {} (compile-time const)", generated::POOL_THREADS);
    println!("tracing      = {}", generated::TRACING);
    println!("log_level    = {:?}", generated::LOG_LEVEL);
}
