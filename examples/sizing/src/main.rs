//! Two-tier sizing pattern.
//!
//! `build.rs` reads `sizing.toml`, validates it, and bakes the values into
//! compile-time constants. Those constants do double duty: they size types that
//! must be `const` (here, a ring buffer), and they serve as the defaults for the
//! runtime override surface resolved through conflaguration.

mod sized {
    include!(concat!(env!("OUT_DIR"), "/sizing_consts.rs"));
}

// compile-time type control: the length is fixed and build-time validated.
type RingBuffer = [u8; sized::RING_CAPACITY];

fn main() -> conflaguration::Result<()> {
    let ring: RingBuffer = [0; sized::RING_CAPACITY];

    // runtime override: SIZING_PUMP_FLUSH_INTERVAL_MICROS wins if set, otherwise
    // fall back to the compile-time default baked in by build.rs.
    let flush_micros: u64 = conflaguration::resolve_or(&["SIZING_PUMP_FLUSH_INTERVAL_MICROS"], sized::FLUSH_INTERVAL_MICROS)?;

    println!("ring capacity  = {} (compile-time const)", ring.len());
    println!("flush interval = {flush_micros}us (runtime override, default = const)");
    Ok(())
}
