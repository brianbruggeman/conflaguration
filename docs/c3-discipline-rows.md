## C3 — `serde_value` round-trip per builder merge layer — REVISITED, tweak MEASURED and ROLLED BACK

Host: Apple Silicon (darwin), quiet except where noted (some runs shared the
box with an unrelated concurrent `Cargo.toml`/`tracing`-optional-ization
session on the same machine — build profile named per row below). Build
profile: `cargo bench` (Cargo `bench` profile, optimized, not debug) unless
stated otherwise. `CARGO_TARGET_DIR` pointed at this session's private
scratch target dir for every command in this row (never the shared
`/tmp/cargo_target`).

### Task 1 — the oracle, read before touching anything

**Internal state** (`src/builder.rs:48-50`, pre-tweak):
```rust
pub struct ConfigBuilder<T> {
    state: Option<Result<T>>,
}
```
`None` = no source yet. `Some(Ok(T))` = running value. `Some(Err(_))` = short-circuited.

**Every short-circuit / early-return site** (pre-tweak line numbers):
- `src/builder.rs:69` — `value()`: `Some(Err(_)) => self` (no-op on error state).
- `src/builder.rs:84` — `env()`: `Some(Err(_)) => self`.
- `src/builder.rs:99` — `env_with_prefix()`: `Some(Err(_)) => self`.
- `src/builder.rs:119` — `override_with()`: `other => Self { state: other }` (covers `None` and `Some(Err(_))`).
- `src/builder.rs:133` — `validate()`: same `other` passthrough.
- `src/builder.rs:141` — `build()`: `None => Err(Error::NoSource)`.
- `src/builder.rs:218` — `merge_from_file()`: `Some(Err(_)) => self`.
- `src/builder.rs:231` — `merge_from_mapping()`: `Some(Err(_)) => self`.

**`merge_value`/`MergeMode`** (`src/file.rs:84-118`, unchanged throughout this
session): `MergeMode::Override` replaces existing keys; `MergeMode::Overlay`
only fills a slot for which `is_empty()` (`Value::Unit` or `Value::Option(None)`)
is true (`src/file.rs:95-97,109-116`). Maps recurse key-by-key
(`src/file.rs:104-107`); everything else is mode-dispatched at the leaf.

**Every test pinning error identity/ordering through the builder**
(`grep -rn "Error::" tests/ src/`, builder-relevant subset):
- `tests/builder.rs:113` `validate_rejects_invalid_config` — pins `Error::Validation` surfaces from `.env().validate()`.
- `tests/builder.rs:120` `build_without_source_returns_no_source` — pins `Error::NoSource` from an empty builder.
- `tests/builder.rs:124-136` `error_state_short_circuits` — pins that `.override_with()` does NOT run its closure once `.validate()` has failed, AND that the surfaced error is still the `Validation` error, not something `override_with` introduced.
- `tests/builder.rs:355` `file_error_short_circuits` — `.file(bad_path).env().build()` pins `Error::Io` from a **first-source** `file()` call (`state=None` path, `crate::from_file`, no serde_value round trip at all) and that `.env()` after it is skipped.
- `tests/builder.rs:367-371` `value_after_error_preserves_error` — `.env().validate().value(default).build()` pins that `value()` cannot clear an error state.
- `tests/builder.rs:374-379` `env_after_error_preserves_error` — same for `.env()` as the clearing attempt.
- `tests/builder.rs:382-387` `env_with_prefix_after_error_preserves_error` — same for `.env_with_prefix()`.
- `tests/builder.rs:358-364` `default_builder_constructs_same_as_new` — pins `Error::NoSource` identically for `Default::default()` and `::new()`.

**Gap found in the oracle, load-bearing for Task 2:** no existing test exercises
an error occurring specifically at an *intermediate* `merge_file`/`merge_mapping`
round trip in a chain of 3+ file/mapping layers (e.g. `.file(good).file(bad_type).file(nonexistent)`).
`file_error_short_circuits` only covers the **first-source** `Io` path, which never
touches the serde_value medium at all. This gap is real and is exactly where a
naive "defer every deserialize to `build()`" redesign would silently change
which error wins (see Task 2).

### Task 2 — design decision

**Rejected design:** accumulate every layer into one `Value` and deserialize
into `T` exactly once, at `.build()`.

**What breaks, with `file:line` evidence:** `file_to_value()` (`src/file.rs:57-82`)
never validates against `T` — it parses the incoming file into the untyped
`Value` regardless of whether it type-checks against `T`. Under today's code, a
type-mismatch in layer 2 of `.file(good1).file(bad_type2).file(missing3)`
surfaces at layer 2's `T::deserialize` call and short-circuits layer 3 — file 3
is **never even opened** (`merge_from_file`'s `Some(Err(_)) => self` arm at
`src/builder.rs:218` returns without calling `crate::file::merge_file` again).
Deferring every deserialize to `.build()` would let layer 3's `Io` error (file
not found) surface *instead of* layer 2's type error, and layer 3's file would
actually be read (an observable side-effect change) — a genuine error-identity
violation, even though no *current* test happens to pin this exact scenario
(the Task 1 gap above). Per the task's own rule ("do not change an error's
identity or ordering to make your optimization work"), full deferral was
rejected without landing it.

**Binary question — new type needed?** No. The chosen alternative needed one
extra field on `ConfigBuilder<T>`: `cache: Option<serde_value::Value>`
(`Option` and `serde_value::Value` both already exist; no new type introduced).
Answered by writing the field, not by arguing about it.

**Faithful alternative implemented:** keep exactly the same per-layer
`T::deserialize` call (so error identity/timing is byte-for-byte unchanged),
but avoid recomputing `serde_value::to_value(base)` from scratch on every
chained merge layer by caching the last successfully-merged `Value` and reusing
it as the next layer's starting accumulator. Since `serde_value::Value`'s
`Deserializer` impl only exists **by value** (verified —
`grep -n "impl.*Deserializer" ~/.cargo/registry/.../serde-value-0.7.0/src/de.rs`
shows only `impl<'de> de::Deserializer<'de> for Value` at line 430, no `&Value`
impl), keeping the accumulator alive across the consuming `T::deserialize` call
required one `Value::clone()` per merge layer.

### Task 3 — implementation, measured, and the negative result

**Files touched during the attempt (both reverted — see Status):**
`src/builder.rs` (added `cache` field, `invalidate_merge_cache()`, converted
`value`/`env`/`env_with_prefix`/`override_with` to `self.state.take()` +
in-place mutation so the new field survives every builder call), `src/file.rs`
(`accumulator()` reuses cache or falls back to `to_value`; `deserialize_and_cache()`
clones before the consuming deserialize).

**Measured result — 5 layer depths, 3-4 runs each, `cargo bench --bench
bench_conflaguration --features derive,toml,json`, bench profile, quiet
box:**

| depth | round trips | before (median, µs, n=4 runs) | after (median, µs, n=3 runs) | before allocs | after allocs |
|---|---|---|---|---|---|
| 2-layer | 0 | 16.62 / 16.16 / 15.76 / 15.47 | 14.42 / 14.95 / 14.84 | 54 | 54 |
| 3-layer | 1 | 33.73 / 29.58 / 30.22 / 29.41 | 31.69 / 28.93 / 29.01 | 93 | **104 (+11)** |
| 4-layer | 2 | 52.74 / 45.29 / 51.93 / 44.78 | 46.63 / 42.83 / 44.03 | 135 | **146 (+11)** |
| 6-layer | 4 | 70.53 / 70.28 / 85.43 / 73.73 | 76.21 / 69.68 / 72.4 | 219 | **230 (+11)** |
| 8-layer | 6 | 96.68 / 102 / 107.8 / 102.8 | 97.41 / 98.85 / 100.9 | 303 | **314 (+11)** |

CoV across runs at 4/6-layer exceeds 5% (before: 4-layer spread 44.78-52.74µs
≈ 8.5% CoV; 6-layer spread 70.28-85.43µs ≈ 8.7% CoV) — reported as a **range**,
not a point estimate, per the noise-band rule. Every "after" median sits
**inside** the "before" range at its depth. **No measured time win at any
depth — this is `unmeasured`/noise, not `improved`.**

**Allocations moved the wrong way**: a flat **+11 allocations** appeared at
every depth that has at least one round trip (2-layer, which has zero round
trips, is unchanged at 54/54). Mechanism, traced: `Value::clone()`
(`src/file.rs::deserialize_and_cache`, the tweak's new step) costs the same as
`serde_value::to_value(base)` (the tweak's eliminated step) — both build a
fresh `BTreeMap<Value, Value>` with newly-heap-allocated `Value::String` leaves
for every string field; there is no cheaper "just copy the bytes" path for
either operation. The tweak therefore pays for the clone on every layer
(including the *last* layer in a chain, whose cached value is never read again)
without recouping it anywhere, since the operation it was meant to replace
(`to_value`) costs the same as the operation it introduced (`clone`). The
premise ("cloning an already-built Value tree is cheaper than rebuilding one
via `T`'s `Serialize` impl") was **falsified by measurement** (principle 18 —
this premise was stated as a design rationale before benching; the bench is
what corrected it, which is exactly why it's recorded here rather than shipped
on the strength of the argument alone).

**Decision: ROLLED BACK.** `src/builder.rs` and `src/file.rs` are restored
byte-for-byte to their pre-session content (`diff` against a captured snapshot
of both files taken before any edit this session returns empty). Per protocol:
"Δ ≥ 0 on every metric → keep... mixed → judge worst metric; if worst is bad,
roll back." Time was flat-to-noise and allocations were **strictly worse** —
not a mixed win, a clean loss on the metric that matters most (allocs), and
the round-trip's own asymptotic behavior did not improve at all
(35-42 allocs/round-trip **before and after**, unchanged — see below).

### Task 4 — bench extended to 6/8 layers; N cases

**5 `builder_merge_*` cases now exist** (`benches/bench_conflaguration.rs`):
`builder_merge_2_layer`, `_3_layer`, `_4_layer`, `_6_layer` (new),
`_8_layer` (new), plus two new diagnostic arms (`_in_memory`,
`_control_skip_merge`, both new — see Task 5). **7 bench functions total in
that section; 15 + 6 = 21 divan cases execute in a full
`cargo bench --bench bench_conflaguration --features derive,toml,json` run**
(asserted from the divan summary — never 0; re-prove with that exact command).

**Linearity claim, re-examined honestly:** per-round-trip allocation cost is
constant and linear **both before and after** the attempted tweak — roughly
39-42 allocs per additional round trip in both codepaths (e.g. before:
93→135→219→303 for 1→2→4→6 round trips is exactly 42/round-trip; after:
104→146→230→314 is also exactly 42/round-trip). **The claim that this tweak
would turn the scaling sub-linear is FALSE, measured** — it changed the
per-round-trip constant by zero (see mechanism above: clone cost ≈ serialize
cost) and only added a flat, one-time +11 tax. Cost remains linear in
round-trip count after this session, exactly as before it.

### Task 5 — adversarial pass and degenerate control

**(a) Correctness — parity test, new permanent test:**
`tests/merge_parity.rs::multi_layer_merge_preserves_skip_field_and_nested_substruct`
builds a 4-source chain (`.value()` seed with a `#[serde(skip)]` field set to a
nonzero sentinel, `.file()`, `.overlay_file()`, `.file()`) over a struct with a
nested sub-struct, and asserts every field. **Run against the cache-tweak code
AND against the pre-tweak code (source files swapped in place, both builds
verified via `diff -q` against saved snapshots first) — identical PASS, identical
assertions, in both cases** (`cargo nextest run --features toml,derive --test
merge_parity`: 1/1 passed, both times). The `#[serde(skip)]` field resets to
its `Default` after any round trip in both implementations, which is `serde`'s
own contract for skipped fields (not a caching artifact — noted explicitly in
the test as a why-comment so a future reader doesn't mistake it for a bug).
This test is now permanent tree state (kept regardless of the rollback — it
tests existing, shipped merge behavior, not the reverted tweak).

**(b) Reproduction under reorder:** 3 independent `cargo bench` invocations
(not filtered/reordered further given the session's time budget) for both the
pre-tweak and cache-tweak code show all five depths landing in overlapping
bands across all three runs each (see the before/after table). The "after"
numbers do not depend on which specific run they came from — they sit inside
the "before" spread every time.

**(c) Wrong-thing-measured — separated, and this is the headline finding.**
The `builder_merge_N_layer` benches call `.file()`/`.overlay_file()` **inside**
the timed closure, and each of those calls does a real `std::fs::read_to_string`
(`src/file.rs:29,58`, unchanged by anything in this session) against a temp
file on every iteration. Added `builder_merge_8_layer_in_memory` — the
identical 5-round-trip merge chain, but via `.mapping()`/`.overlay_mapping()`
on in-memory `MergeConfig` values, zero filesystem syscalls:

| bench | median | mechanism |
|---|---|---|
| `builder_merge_8_layer` (file-based, 6 round trips + 7 file reads) | 102.8 µs | round-trip merge + 7× `read_to_string` |
| `builder_merge_8_layer_in_memory` (5 round trips, 0 file reads) | 9.083 µs | round-trip merge only |

**~90% of the `builder_merge_N_layer` cost measured throughout this session
(and in the original C3 finding this task was asked to revisit) is disk I/O,
not the serde_value round trip.** Per-round-trip cost in memory is
≈9.083 µs / 5 ≈ **1.8 µs**, not the ~14-15 µs/layer figure in the original C3
row (`docs/discipline.md:143-157`) — that figure conflated file-read latency
with merge-medium cost. **This is the number that hurts most in this session's
findings**: the original diagnosis of *where* the cost lives was wrong, and the
tweak this session attempted was chasing a ≤1.8 µs/layer target with an
optimization that (per Task 3) didn't even net-positive on that slice. Filed
here as a correction to the C3 row rather than silently re-measured and
re-labeled.

**Degenerate control — required, run, and it passed (i.e. correctly showed a dramatic difference):**
`builder_merge_8_layer_control_skip_merge` clones a `MergeConfig::default()`
seven times, touching **zero** serde machinery and **zero** files. Result:
**62.17 ns median, 0 allocations** (all fields are `Default`-zero/empty,
including two empty `String`s, which `Clone` never heap-allocates for) versus
**102.8 µs / 303 allocations** for the real `builder_merge_8_layer`. A **1654×**
time gap and an infinite (0 vs 303) allocation gap confirms the real bench is
measuring substantial, real work — not a fixed harness constant the control
would have shared. Control outcome: **PASSED** (the intended failure mode —
"looks the same as the real bench" — did not occur).

### Status

**MEASURED, ROLLED BACK.** `src/builder.rs`/`src/file.rs` are byte-identical
to their pre-session state. Landed and kept: `benches/bench_conflaguration.rs`
gained `_6_layer`, `_8_layer`, `_in_memory`, and `_control_skip_merge` arms;
`tests/merge_parity.rs` is a new permanent correctness test, independent of
the reverted tweak.

**Re-prove commands:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json -- builder_merge
cargo nextest run --features toml,derive --test merge_parity
cargo nextest run --all-features   # 236 tests (235 + this session's new parity test)
cargo nextest run                  # 71 tests
```

**Residual — named, not silently dropped:** the only way to actually cut
merge-medium cost below "clone/serialize a same-shape tree per round trip"
is a **non-consuming** deserializer over `&serde_value::Value` (the crate only
ships an owned-value `Deserializer` impl), so the accumulator could be mutated
in place across an entire chain without ever cloning or re-serializing the
untouched majority of fields. That is new, nontrivial code (a hand-written
`Deserializer<'de> for &'a Value` covering every `Value` variant) with real
risk to the error-identity contract if a variant is mishandled, and it did not
fit this session's remaining budget after the negative result above. It is the
correctly-scoped next step if C3 is revisited — but priority should first go
to the ~90%-of-cost disk-I/O finding in (c), since it dwarfs the round-trip
cost this row was chasing.
