# conflaguration discipline log

Host: Apple Silicon (darwin), quiet except where noted. Build profile named
per row (`cargo bench` uses Cargo's `bench` profile — optimized, not debug;
`cargo build --release` for the consumer measurements in Task 3). `RUSTC_WRAPPER=sccache`
and `CARGO_TARGET_DIR=/tmp/cargo_target` are ambient in this shell for every
row below; sccache was explicitly disabled (`env -u RUSTC_WRAPPER -u
CARGO_BUILD_RUSTC_WRAPPER`) for every Task 3 clean/incremental build timing so
those numbers are not cache-assisted.

Bench harness: `benches/bench_conflaguration.rs` (divan, `AllocProfiler` global
allocator). Re-prove any row below with:

```
cargo bench --bench bench_conflaguration --features derive,toml,json
```

20 bench cases execute per run (asserted from the divan summary line count in
every log under `/private/tmp/claude-.../scratchpad/bench_*.log` — never 0). A
flagless `cargo bench` runs neither `bench_conflaguration` (skipped for
missing `derive`/`toml`/`json`) nor any real case; `benches/bench_guard.rs`
carries no `required-features` and exits non-zero in that case so the skip
cannot read as a pass.

## Allocation budget (stated before optimizing, principle 18/§11)

- `join_key` (hot path — called once per field, per nesting level, on every
  `from_env()`): budget is **exactly one allocation**, sized to fit. Measured
  baseline was two allocator operations (one undersized `alloc` + one `grow`)
  — a violation of the stated budget, not a design choice.
- `from_file`/builder merge (setup path, called once per process typically):
  no stated zero-allocation budget; bounded is acceptable, but allocator
  *operation count* (not just byte count) should scale linearly with layer
  count, not super-linearly.
- template `render`/`render_lenient` (cold-to-warm path depending on caller):
  zero allocations on the no-match arm (borrowed return) is the stated
  contract — confirmed by the bench (`template_1kb_no_match` shows no
  alloc/dealloc rows in the divan table, i.e. 0 allocator calls over 1600
  iters).
- derive-generated per-field key-list buffers (`dynamic_key_tokens`,
  `crates/conflaguration-derive/src/lib.rs:390-401`, called once per field per
  `from_env()`): budget is **zero heap allocations** for the `__keys` and
  `__key_refs` buffers themselves. The key `String` each buffer holds (one
  per `join_key` call, budgeted above at exactly one allocation) is a
  separate line item, not part of this budget. Met as of C4.
- per-field default-fallback path (`environs::resolve_or` and siblings,
  `environs` crate — a dependency, not this repo): budget is **zero
  allocations for error construction**. An `Error::NotFound` built only to
  be discarded unread by the `Err(Error::NotFound { .. }) => Ok(default)`
  arm is a budget violation, not a design choice — the same standard
  `join_key`'s two-allocator-op baseline was held to above. Met in environs
  1.3.0 (C5, `scan` returns `None` on exhaustion, never constructs the
  error). Not met by the environs 1.2.0 this crate currently resolves
  (`Cargo.toml:16`) — every default-fallback field on the shipped
  dependency still pays that allocation today.

## Opt-sweep axes (§11, adapted — this crate is not sans-IO, most axes N/A)

| axis | status | rationale |
|---|---|---|
| state machine | N/A | config resolution is a one-shot pipeline, not a protocol state machine |
| bytes-first | PARTIAL | `template.rs` is bytes-first by design; `file.rs`/derive path is text (TOML/JSON/env strings), which is the correct boundary — config values are text at their source |
| borrowed views | DONE where possible | `template::render` returns `Cow`, borrows on the no-match path (confirmed 0-alloc in bench) |
| zero-copy | N/A | config values are owned by design (outlive the source file/env snapshot) |
| copy-over-clone | DONE | `join_key` after this session's fix does exactly one allocation, no intermediate clones |
| SIMD | N/A | no byte-scan hot loop large enough to matter; `template::find_closing` is a linear byte scan over typically-small config text, not a wire hot path |
| stack-over-heap | N/A-with-rationale | key strings are inherently heap (`String`, returned to the caller); no fixed upper bound on prefix/segment length to justify a stack buffer |
| branchless inner loop | N/A | no hot inner loop at this scale (config resolution runs once to a few thousand times per process, not per-packet) |
| no dynamic dispatch | DONE | `Resolver` trait is used via static generics (`impl Resolver`/tuple impls), never `dyn Resolver`, in the crate's own call sites |
| O(1) per token | N/A | `join_key` is O(prefix+segment) by necessity (must copy both into a new buffer); no per-token loop exists |

## 16-point gate audit (C1-C5)

This table audits all five rows below (C1-C5) against the disciplined-component
skill's 16 gates. Standing rule: a row with blank cells is not done, and per
gate 13 a row with no `design-favors: incumbent` arm has not been benched yet.

| # | gate | C1 | C2 | C3 | C4 | C5 | notes |
|---|---|---|---|---|---|---|---|
| 1 | spin out behind compile-time default-off gate | FAIL (exception) | N/A | N/A | FAIL (exception) | FAIL (exception) | shipped default path changed with no feature gate and no e2e bench behind it (C2 is a dependency deletion, C3 was rolled back and never landed, so neither applies). Behaviour-preserving, 243 tests unchanged, and C4 passing an independent adversarial review is an argument, not the gate — recorded as a **named deliberate exception**, not N/A |
| 2 | build clean at every claimed tier | PASS-vacuous | PASS-vacuous | PASS-vacuous | PASS-vacuous | PASS-vacuous | no `#![no_std]` anywhere in `src/` or `crates/`, no tier claim in `README.md` or `docs/guide.md` — the crate claims no restricted tier, so there is nothing to verify; a vacuous pass is not evidence of tier support |
| 3 | tests pass with N asserted | PASS | PASS | PASS | PASS | PASS | conflaguration: 243 passed, 0 skipped. environs (C5): 175 default-features, 175 no-default-features, 208 all-features |
| 4 | lint clean at strictest baseline | PASS | PASS | PASS | PASS | PASS | `cargo clippy --workspace --all-targets -- -D warnings` exit 0, `unwrap_used` and `expect_used` denied crate-wide |
| 5 | micro-bench exists | PASS | PASS | PASS | PASS | PASS | `benches/bench_conflaguration.rs`, 20 divan cases |
| 6 | compare-bench vs explicit named alternatives | PASS (C6) | FAIL | PASS (C6) | PASS (C6) | PASS (C6) | `envy`, `config`, `figment` dev-dependencies (`Cargo.toml:33,35,36`) and six named-incumbent bench arms now exist (C6) covering the env-resolution path (C1 join_key, C4 derive buffers, C5 environs scan) and the layered-merge path (C3). C2 (unused-dependency removal) has no comparable incumbent operation and stays FAIL |
| 7 | E2E bench | PASS (C7) | PASS (C7) | PASS (C7) | PASS (C7) | PASS (C7) | `e2e_service_boot`/`e2e_service_boot_minimal` (`benches/bench_conflaguration.rs:750,773`) now compose file-merge + env-resolution + template-render + validate + build into one measured boot (C7). Unlike gates 6/13 this gate is a crate-level property (does any e2e bench exist), not a per-row incumbent comparison — the original FAIL used one identical rationale across all five columns rather than a row-specific one — so C7 seals it uniformly across all five columns, C2 included |
| 8 | optimisation sweep recorded | PARTIAL | PARTIAL | PARTIAL | PARTIAL | PARTIAL | the `## Opt-sweep axes` table (`docs/discipline.md:57`) exists but marks most axes N/A for this crate's domain; the external-library sweep gate 8 actually asks for is not recorded per row |
| 9 | re-bench every tweak, including rollbacks | PASS | PASS | PASS | PASS | PASS | C3 is a recorded rollback with its falsified premise; C1 records a degenerate control that was itself invalid and was fixed |
| 10 | SIMD / state-machine / no-dyn pass | PARTIAL | PARTIAL | PARTIAL | PARTIAL | PARTIAL | declared N/A in the opt-sweep table; no disassembly was read to confirm what the compiler emitted, which is what gate 10 actually asks for |
| 11 | strict O(1) steady-state | PASS | PASS | PASS | PASS | PASS | no hashing on the resolve path; `join_key` is O(prefix+segment) by necessity |
| 12 | config + API parity fixture | PARTIAL/FAIL | PARTIAL/FAIL | PARTIAL/FAIL | PARTIAL/FAIL | PARTIAL/FAIL | the only candidate is `tests/builder.rs:359-363` (`default_builder_constructs_same_as_new`), which asserts both construction paths return `Err(NoSource)`. Two errors matching is not identical state — no fixture builds a populated config both ways and asserts equivalence |
| 13 | home-turf incumbent arm (`design-favors: incumbent`) | PASS (C6) | FAIL | PASS (C6) | PASS (C6) | PASS (C6) | C6 adds `design-favors: incumbent` arms against envy (env resolution) and config-rs/figment (layered merge); conflaguration meets-or-beats on all of them. C2 has no incumbent counterpart and stays unsealed |
| 14 | internal-primitive audit | PASS | PASS | PASS | PASS | PASS | each row carries a "Binary question (principle 1)" subsection (`docs/discipline.md:157,210,298,398,507`) answering whether an existing primitive expresses it |
| 15 | no magic numbers | PASS | PASS | PASS | PASS | PASS | a grep for tunable constants, `with_capacity(N)` literals, and common power-of-two literals across `src/` and `crates/` returns nothing |
| 16 | re-provable now from the artifact | PASS | PASS | PASS | PASS | PASS | every row carries a re-prove command. C5 additionally records the `cargo update -p environs` gotcha without which the re-prove silently measures the unpatched crate |

### Project memory bootstrap: no index exists

A check for an agent-facing docs index found none: no `ai_docs/`, no
`AGENTS.md`/`AGENT.md`, no `index.jsonl`/`task-routes.jsonl`/`invariants.jsonl`
anywhere in the repo; `.claude/` contains only `worktrees/`. No registry
record was added — the repo has no such convention to add one to, and
inventing one was not this task.

### The four originally-named gaps — now closed, and what is still open

This section originally named four blanks: gates 6, 13, 7, and 12. All four
are closed as of this work session:

1. **Gate 12 — config + API parity fixture.** Closed by `tests/builder.rs:437-571`
   (`expected_parity_config`/`parity_env_vars` plus four `assert_eq!` checks
   asserting a populated config field-for-field across the derive, fluent,
   value, and file construction paths), 245 tests total.
2. **Gates 6 and 13 — compare-bench vs named alternatives / home-turf
   incumbent arm.** Closed together by C6 (`docs/discipline.md:588`), which
   added `envy`, `config`, and `figment` dev-dependencies and six named-
   incumbent bench arms covering the env-resolution path and the
   layered-merge path.
3. **Gate 7 — E2E bench.** Closed by C7 (`docs/discipline.md:706`), which
   added `e2e_service_boot`/`e2e_service_boot_minimal` — two bench functions
   composing the crate's file-merge, env-resolution, template-render, and
   validated-build steps into one measured boot.

All three closures happened in this same session. What the gate-audit table
(`docs/discipline.md:72`) still marks open is not among these four: gate 8
(PARTIAL — the external-library sweep gate 8 actually asks for is not
recorded per row) and gate 10 (PARTIAL — no disassembly was read to confirm
what the compiler emitted), plus the gate-1 named exception (shipped default
path changed with no feature gate and no e2e-first firewall, carried by C1,
C4, and C5). Those three are the genuinely open items left.

### What the closed gates mean for the original question

Gates 6, 13, and 7 are now closed, and conflaguration meets-or-beats the
named incumbents on env-resolution and layered-merge at the roughly-once-
per-process call scale C6 measured (`docs/discipline.md:662-673`). This
still does not explain the 31% link-time regression that caused a
downstream consumer to exclude the crate. C6 measured per-operation speed,
not link footprint; the regression sits on a link-time / binary-footprint
axis, and C2 (`docs/discipline.md:200`) measured binary size directly and
found exactly zero delta. C7 does not touch this axis either — it measures
wall-clock composition cost at runtime, not link behaviour or binary size.
The original motivating problem remains unexplained and unreproduced in
this log. Stated plainly, per C7's own proportion result
(`docs/discipline.md:763`): the whole C1-C7 body improved a once-per-process
cost by a sub-2% slice, while the thing that got the crate excluded — a
link-time regression — is untouched by any row in this log.

## C1 — `join_key` (src/lib.rs:238)

**Tweak:** replace `format!("{prefix}_{segment}")` with
`String::with_capacity(prefix.len() + 1 + segment.len())` +
`push_str`/`push`/`push_str`. `format!`'s capacity estimation under-sized the
buffer for this shape, forcing a realloc (`grow`) on every non-empty-prefix
call.

**Binary question (principle 1, rust.md):** can this be expressed with what
already exists? Yes — `std::String::with_capacity` + `push_str` is the
existing primitive; no new type, no new dependency.

| metric | before | after | delta |
|---|---|---|---|
| `join_key_with_prefix` median (3 runs) | 56.85 / 57.26 / 57.64 ns | 12.92 / 12.56 / 12.64 ns | **-77.8% to -78.1%** |
| CoV (fastest-slowest spread, 100 samples) | <1% | <2% | both well under the 5% report-as-range bar |
| allocator ops/call | 1 alloc (12 B) + 1 grow (12 B) = 2 ops, peak 24 B | 1 alloc (17 B), 0 grow = 1 op, peak 17 B | **-50% allocator ops, -29% peak bytes** |

**Downstream effect** (same field count, no other change):

| bench | before median (3 runs) | after median (3 runs) | delta | alloc-count delta |
|---|---|---|---|---|
| `from_env_flat20` | 4.249 / 4.714 / 4.345 µs | 3.6 / 3.48 / 3.6 µs | **-15% to -26%** | alloc count unchanged (85); 20 `grow` ops removed |
| `from_env_nested3` | 1.045 / 1.109 / 1.121 µs | 797.6 / 772.8 / 776.8 ns | **-24% to -31%** | grow ops removed on the nested-prefix path |
| `from_env_flat5` | 1.042 / 1.148 / 1.024 µs | 833 / 849.7 / 874.6 ns | **-15% to -24%** | single-level prefix, smaller effect (1 join_key call/field vs nested's 2+) |

**CoV / runs:** 3 runs before, 3 runs after (`bench_run2/3/4.log`,
`bench_post1/2/3.log` in the scratchpad); before/after bands do not overlap on
any of the four benches above — this is outside noise, not a rounding
artifact.

**Behaviour is the oracle:** `cargo nextest run --all-features` — **231
tests, 231 passed, 0 skipped** before and after (unchanged N).

**Mechanism:** the win is allocator *operation count*, not byte count — total
bytes allocated across `from_env_flat20`'s 85 calls rose slightly (1.465 KB
-> 1.645 KB, since we now allocate the true final size instead of an
undersized buffer that gets doubled), but the 20 `grow` calls (each a
realloc: allocate new block, memcpy old contents, free old block) are
eliminated entirely. Fewer allocator round-trips, not fewer bytes moved, is
what the wall-clock delta traces to.

**Re-prove command:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json
```
Compare `join_key_with_prefix` and `from_env_flat20`/`from_env_nested3` rows;
alloc/grow counts are printed directly under each bench in the divan table.

**Status: LANDED.**

## C2 — unused `memchr` dependency (Cargo.toml:23)

**Finding:** `grep -rni memchr --include='*.rs' .` (excluding `target/`)
returns **zero matches** in the crate's own source. `memchr` was declared as
a non-optional dependency and never imported anywhere — the byte-scan in
`template.rs::find_closing`/`has_template` is hand-rolled, not calling into
the `memchr` crate at all.

**Tweak:** `cargo remove memchr`.

**Binary question:** N/A — this is deletion of dead weight, not a new
primitive.

| metric | before | after | delta |
|---|---|---|---|
| `consumer-empty` (features=[]) binary size, release | 435,600 B | 435,600 B | **0 — no change** |
| `consumer-empty` symbol count (`nm \| wc -l`) | 877 | 877 | **0 — no change** |
| `cargo tree` line count, `consumer-empty` | 27 | 26 | **-1 crate node** |

**Mechanism:** the linker already dead-code-eliminated `memchr`'s unused
symbols from the final binary (LTO/garbage-collection at link time strips
unreferenced `rlib` code), so removing the declared dependency has **zero**
effect on shipped binary size or symbol count for this consumer. It does
remove one crate from the *compile* graph — one fewer crate to download,
compile, and audit from a cold cache — which is a real but small saving that
does not show up in the linked-binary numbers. Reported honestly: this is a
hygiene/compile-graph win, not a runtime or binary-size win.

**Behaviour is the oracle:** `cargo nextest run --all-features` — **231
tests, 231 passed, 0 skipped**, unchanged.

**Re-prove command:**
```
cargo tree | wc -l          # in Task 3's consumer-empty scratch crate, before vs after
nm target/release/consumer-empty | wc -l
```

**Status: LANDED** (compile-graph win confirmed measured; binary-size claim
explicitly NOT made — measured flat).

## C3 — per-layer builder merge cost (src/file.rs:126,135,147-148) — ATTEMPTED, ROLLED BACK; original mechanism was misattributed

**Measurement (Task 2 baseline, 3 runs, CoV noted):**

| bench | median (3 runs) | CoV | allocs |
|---|---|---|---|
| `builder_merge_2_layer` | 15.12 / 14.83 / 15.44 µs | ~2% | 54 |
| `builder_merge_3_layer` | 30.06 / 28.86 / 30.1 µs | ~2% | 93 |
| `builder_merge_4_layer` | 43.97 / 45.81 / 44.38 µs | ~2% | 135 |

Cost scales ~linearly at ~14-15 µs and ~40 allocs per additional layer.

**The mechanism above was wrong — corrected.** This row originally attributed
that ~14-15 µs to the `serde_value::to_value(base)` round-trip at
`src/file.rs:135,147`. An isolation arm added later
(`builder_merge_8_layer_in_memory`, no file reads) was compared against
`builder_merge_8_layer` to attribute cost, but a second adversarial pass found
the two arms were not doing the same amount of work: `.file()`/`.mapping()`
called first from empty state take a one-shot deserialize path
(`crate::from_file` / `crate::file::from_mapping`, `src/builder.rs:220-221,
232-233`) with no merge, and only later calls hit `merge_file`/`merge_mapping`
(`src/builder.rs:223-224,235-236`) for a genuine `serde_value` round trip. The
file arm seeded with `.file()` (a one-shot deserialize) and then alternated
`overlay_file`/`file` six more times — **6 round trips**. The in-memory arm
seeded with `.value(...)` (a bare move, zero serialize/deserialize) and only
alternated five more times — **5 round trips** — one layer short of the file
arm and missing the first layer's one-shot deserialize entirely. Comparing
95.25 µs against that undercounted 8.699 µs manufactured a 91%/9% split from
an apples-to-six-apples-and-five-oranges pair.

**Fix:** `builder_merge_8_layer_in_memory` now seeds with `.mapping(...)`
(the same one-shot-deserialize shape as `.file()`'s first call) and carries a
7th layer, so both arms perform 1 one-shot deserialize + 6 genuine merge
round trips, differing only in disk I/O. Re-measured (3 runs, medians):

| bench | median (3 runs) | mean (3 runs) |
|---|---|---|
| `builder_merge_8_layer` (file, 7 layers) | 96.26 / 101.1 / 121.7 µs | 97.42 / 106.2 / 126.6 µs |
| `builder_merge_8_layer_in_memory` (mapping, 7 layers) | 10.99 / 11.12 / 11.48 µs | 11.03 / 11.21 / 11.53 µs |

File-arm CoV across the 3 runs is ~12.7%, over the 5% report-as-range bar —
reported as a range, not a point value. Per-run split (file total minus
in-memory total, over file total): 88.6% / 89.0% / 90.6% disk I/O, 11.4% /
11.0% / 9.4% round trip. **Corrected split: ~89-91% disk I/O
(`std::fs::read_to_string` + TOML parse per layer), ~9-11% the `serde_value`
round trip** — close to the originally reported 90%/10%, but that number was
previously backed by a mismatched 6-vs-5-round-trip comparison; it is now
backed by a matched 6-vs-6 one. Dividing the in-memory total by its 6 round
trips gives **~1.8-1.9 µs/round-trip**, consistent with the original
per-layer estimate, though this division still folds the first layer's
one-shot-deserialize cost in as noise rather than isolating it. The per-layer
totals in the original table (2/3/4-layer rows above) are unaffected by this
correction — only the 8-layer file-vs-memory comparison was mismatched. This
propagated into `README.md` and `docs/guide.md` before being caught.
Recorded here because a measurement paired with the wrong mechanism is not a
result, and this row is the reason two other documents had to be corrected
twice now.

**Binary question (principle 1):** can this be expressed with what already
exists? For the *structural* fix (merge once at the end instead of per layer,
or merge in place on a single `serde_value::Value` and only materialize `T`
once at `.build()`), the answer is genuinely "not without a type/state
change" — `ConfigBuilder<T>`'s state is `Option<Result<T>>`; carrying a
half-merged `Value` alongside or instead of `T` through the chain is a real
internal representation change (still `Value`, an existing type — no new type
needed — but a different field in `ConfigBuilder`, and `override_with`'s
closure-over-`T` signature would need to force materialization at that point
in the chain, which changes when errors surface).

**Why deferred, not landed:** the change touches the builder's core state
machine and its error-ordering contract (`override_with`, `validate`,
short-circuit-on-error), which several existing tests
(`tests/builder.rs::value_after_error_preserves_error`,
`override_with_mutates_value`, etc.) pin exactly. Correctly reworking it
within this session's remaining scope risked landing an unmeasured behavior
change on the error-path semantics — against principle 15 (the foundational
treatment, not a rushed one) and principle 16 (every row re-provable, not "it
should still pass"). Recorded here as the **second-highest-value target**
after `join_key`, with its own measured cost, for the next session.

**Status: ATTEMPTED AND ROLLED BACK.** The fix described above was
implemented in a later pass: `ConfigBuilder<T>` gained a
`cache: Option<serde_value::Value>` so chained layers reuse the last merged
`Value` instead of re-deriving it from `T`. No new type — an existing
`Option` over an existing `Value`. The per-layer `T::deserialize` was kept
untouched so error identity and ordering were preserved, verified against
every error-pinning test in `tests/builder.rs`.

Measured at 2/3/4/6/8 layers, 3-4 runs each, before and after:

- **time: no improvement at any depth.** Every post-change median sat inside
  the pre-change variance band. Marked `unmeasured`, not `improved`.
- **allocations: regressed, flat +11 per chain** at every depth with at least
  one merge round-trip.

**Mechanism for the failure, traced to source:** `serde_value` ships only a
by-value `Deserializer` impl — there is no `impl Deserializer for &Value` in
the vendored crate. So a cached accumulator must `Value::clone()`, and a
clone rebuilds the same `BTreeMap<Value, Value>` with freshly heap-allocated
`Value::String` leaves, costing what `to_value()` cost. The premise "clone is
cheaper than serialize" was falsified by measurement, not argued down.

Rolled back; `src/builder.rs` and `src/file.rs` are byte-identical to their
pre-attempt state. Two additions were kept: the extended bench arms
(6-layer, 8-layer, the in-memory isolation arm, and a clone-only degenerate
control measuring 62 ns / 0 allocs against the real bench's 102.8 µs / 303
allocs — a 1654x gap confirming the harness measures real work), and
`tests/merge_parity.rs`, a permanent correctness test covering a
`#[serde(skip)]` field and a nested substruct through a multi-layer chain.

**The correctly-scoped next step, if anyone returns to this:** not the cache.
A real reduction needs a hand-written non-consuming
`Deserializer<'de> for &'a serde_value::Value` so the accumulator can be
mutated in place across a chain. That is real new code with real risk to the
error-identity contract. It is also lower value than it looks, because the
isolation arm shows the round-trip is only ~9-11% of the cost (corrected
figure above) — the other ~89-91% is disk I/O that a config crate cannot
optimize away and that a caller pays once at startup.

## C4 — derive-generated key-list buffers: `Vec` -> stack array (crates/conflaguration-derive/src/lib.rs:390-401)

**Tweak:** the `Settings` derive macro's `dynamic_key_tokens` function
(`crates/conflaguration-derive/src/lib.rs:390-401`) emitted, per scalar
field:

```rust
let __keys: Vec<String> = vec![conflaguration::join_key(__prefix, "FIELD_NAME")];
let __key_refs: Vec<&str> = __keys.iter().map(|s| s.as_str()).collect();
```

three heap allocations per field: the `join_key` `String` (necessary), the
`Vec<String>` backing buffer, and a second `Vec<&str>` re-collected from the
first. Changed to:

```rust
let __keys: [::std::string::String; N] = [ /* N join_key(...) or N .to_string() for envs_override */ ];
let __key_refs: [&str; N] = __keys.each_ref().map(|s| s.as_str());
```

`N` is fixed at macro-expansion time by `field_segment_names`
(`crates/conflaguration-derive/src/lib.rs:380-386`), which reads the parsed
attribute AST — `attrs.envs`, populated from the `envs = "KEY"` /
`envs = ["K1","K2"]` attribute, or the single field-name-derived key when
`envs` is empty. This covers the plain field (1 key), `envs = "KEY"` (1),
the `envs = [...]` cascade (N), and `envs_override` (N, via `.to_string()`
instead of `join_key`) — every form on this path has a key count known at
compile time, so nothing was left on the `Vec` path for `dynamic_key_tokens`.
`each_ref()` is a zero-allocation array method (`&[String; N] ->
[&String; N]`, then `.map` to `[&str; N]`); `&[&str; N]` coerces to `&[&str]`
at every call site (`gen_field_construct`, `gen_override_guard`), so no
caller needed to change.

`gen_display_keys_expr` (`crates/conflaguration-derive/src/lib.rs:628-641`)
was deliberately left on the `Vec` path — a scope decision, not an
oversight. It is a cold path used only to build `ConfigDisplay`'s debug
string and its result is immediately consumed by `.join(", ")`, so there is
no per-field-resolution cost to recover there.

**Binary question (principle 1, rust.md):** can this be expressed with what
already exists? Yes — `[T; N]` and `each_ref()` are existing `std`
primitives; no new type, no new dependency. The only reason this wasn't
already an array is that `N` has to be threaded through as a `proc_macro2`
`TokenStream` const rather than a runtime value, which the macro already had
available (`key_count = names.len()`, computed at expansion time) and simply
wasn't using.

| bench | median before (3 runs) | median after (3 runs) | allocs before | allocs after | alloc delta |
|---|---|---|---|---|---|
| `from_env_flat5` | 890.1 / 832 / 886.3 ns | 730.3 / 751.2 / 753.3 ns | 21 | 11 | **-10 (-48%)** |
| `from_env_flat20` | 3.599 / 3.596 / 3.605 µs | 3.023 / 3.106 / 3.094 µs | 85 | 45 | **-40 (-47%)** |
| `from_env_nested3` | 813.1 / 775.8 / 809.2 ns | 692.8 / 713.5 / 720.6 ns | 21 | 13 | **-8 (-38%)** |

Alloc counts were identical across all 3 runs on each side, before and
after. Time deltas: roughly -14% to -18% on `from_env_flat5`/
`from_env_flat20`, roughly -8% to -11% on `from_env_nested3`.

A prediction of "85 -> about 45" for `from_env_flat20` was written down
before measuring. It landed at exactly 45. Recorded here because that is
what makes this a test rather than a rationalisation after the fact — same
standard C1's harness held itself to in the degenerate-control section
below.

**Mechanism:** contrast with C1. C1 (`join_key`, `src/lib.rs:238`) removed
`grow` operations while total bytes allocated across `from_env_flat20`
slightly *rose* (undersized buffer replaced by a correctly-sized one) — so
only allocator *operation count* moved, and the wall-clock delta traced to
fewer allocator round-trips, not fewer bytes. C4 removes two heap
allocations per field outright (the `Vec<String>` backing buffer and the
`Vec<&str>` re-collect) — both the operation count and the bytes allocated
fall together, which is why both metrics move together here and did not in
C1.

C4 does not reach zero allocations per field, and that floor is outside
this macro. What remains at `from_env_flat20`'s 45 is: 20 fields x 1
necessary `join_key` `String`, plus 20 x 1 `String` that `environs`
allocates internally to build an `Error::NotFound { keys: keys.join(", ") }`
(`environs/src/parse.rs:12`) on every field that falls through to a default
— that error is constructed, then discarded unread by the `Err(Error::NotFound
{ .. }) => Ok(default)` arm in `environs/src/resolve.rs` — plus 5 kept
`String` defaults for the 5 fields with no matching env var:
20 + 20 + 5 = 45. A fix for the discarded-error allocation is in progress
in the `environs` crate itself and cannot reach conflaguration until
environs 1.3.0 is published and the dependency bumped, since this crate
currently depends on `environs = "1.2.0"` from crates.io (`Cargo.toml:16`).

**Behaviour is the oracle:** `cargo nextest run --workspace` — **241
tests, 241 passed, 0 skipped**, unchanged before and after. `cargo clippy
--workspace --all-targets -- -D warnings` — exit 0, unchanged. `cargo test
--doc -p conflaguration --features derive,toml,codegen` — **26 doctests,
exit 0**, unchanged. `cargo build --workspace --all-targets` — exit 0.

**Re-prove command:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json
```
Compare `from_env_flat5`/`from_env_flat20`/`from_env_nested3` rows; alloc
counts are printed directly under each bench in the divan table.

**Status: LANDED.**

## C5 — `environs::resolve` not-found allocation (`environs` crate, not this repo — `environs/src/resolve.rs`, `environs/src/parse.rs:12`)

**Tweak:** `environs/src/resolve.rs` previously had `resolve::<T>(keys)` loop
`env::var(key)`, and on exhaustion call `T::on_not_found(keys)`, which
allocates a `String` via `keys.join(", ")` (`environs/src/parse.rs:12`) to
populate `Error::NotFound`. `resolve_or` then matched
`Err(Error::NotFound { .. }) => Ok(default)` — discarding that String
unread. Every field falling back to its default paid one allocation for an
error message nobody sees. This is the same discarded-error allocation
named as C4's remaining floor (`crates/conflaguration-derive/src/lib.rs`
row above, "20 x 1 `String` that `environs` allocates internally").

The fix introduces a shared private `scan`:

```rust
fn scan<'keys>(keys: &[&'keys str]) -> Option<(&'keys str, String)> {
    keys.iter().find_map(|key| env::var(key).ok().map(|raw| (*key, raw)))
}
```

`resolve`, `resolve_or`, `resolve_or_parse` and `resolve_or_else` all match
on it. The `_or` family returns the default directly on `None`, never
constructing `Error::NotFound`. `resolve` still calls `T::on_not_found` on
exhaustion, unchanged. A `parse_found` helper does the `T::from_env_str` +
`Error::Parse` construction once.

**The intermediate defect, kept because it is the instructive part.** A
first version of this fix had `resolve_or` call a `lookup()` helper
returning `Option<String>` purely as a presence test, discard the value,
then delegate to `resolve::<T>(keys)`, which rescanned and allocated the
same String a second time. That traded a wasted allocation on the
not-found path for a wasted allocation on the *found* path — strictly
worse for anyone who actually sets their env vars. Caught by reading the
diff, not by any test. The current `scan` shape returns the matched pair
so nothing is re-read.

**On the test that does not cover this.** A test was written to catch
that double-scan regression, then empirically checked against the
reverted buggy code — it passed on *both* versions. `env::var` is
idempotent, so rescanning an unmutated environment cannot change which key
wins or what `Error::Parse` carries. The defect is provable only by
reading the code; no `Result`-observable assertion can catch it without an
allocation- or call-count harness, which `environs` does not have. The
test was kept (it pins key-attribution order, a genuinely different bug
class) but it does not cover the allocation defect, and claiming otherwise
would be false coverage.

**Binary question (principle 1):** can this be expressed with what
already exists? Yes — `scan` returns `Option<(&str, String)>` built from
`Iterator::find_map` over the existing key slice; no new type, no new
dependency. The fix is a reshaping of control flow (return the matched
pair instead of a presence bool, then delegate on it) inside the existing
function shapes, not new machinery.

Measured by temporarily pointing conflaguration at a snapshot of the
local `environs` 1.3.0 working tree via `[patch.crates-io]`, benching,
then reverting. Harness `benches/bench_conflaguration.rs` (divan,
`AllocProfiler`), medians of 3 runs per side, 20 divan cases asserted on
all 6 runs (never 0).

| bench | before time | before allocs | after time | after allocs |
|---|---|---|---|---|
| `from_env_flat5` | 725.2 ns | 11 | 647.1 ns | 6 |
| `from_env_flat20` | 2.961 µs | 45 | 2.649 µs | 25 |
| `from_env_nested3` | 682.5 ns | 13 | 624.3 ns | 9 |

`from_env_nested3`'s before-median carries CoV ~4.6%, just under the 5%
report-as-range bar — noted, not rounded away.

A prediction of "45 -> about 25" for `from_env_flat20` was written down
before measuring. It landed at exactly 25. Recorded here for the same
reason C4 recorded its own pre-registered prediction: that is what makes
this a test rather than a rationalisation after the fact.

**Cumulative, C4 + C5, `from_env_flat20`:** 85 (pre-C4 baseline) -> 45
(C4, derive `Vec` -> stack array) -> 25 (C5, environs `scan`). That is
**-70.6% allocations** (85 -> 25) and **3.60 µs -> 2.65 µs, about -26%
time**, from the 85-allocation baseline. Stated explicitly here because
neither row alone shows the combined effect.

**Mechanism, traced to source:** before the fix, every field that falls
through to its default runs the full key list to exhaustion, then
`T::on_not_found(keys)` builds `Error::NotFound { keys: keys.join(", ") }`
— one heap `String` — which `resolve_or` immediately discards in its
`Err(Error::NotFound { .. }) => Ok(default)` arm. `scan` removes the
allocation from that path entirely: on exhaustion it returns `None`, and
`resolve_or` returns the default with no `Error` value ever constructed.
The found-path cost is unchanged (`parse_found` does exactly the
`T::from_env_str` + `Error::Parse` construction the old code did). The
allocation removed here is the same one C4 named and could not reach —
C4's `from_env_flat20` mechanism paragraph totals 45 as "20 necessary
`join_key` Strings + 20 discarded `Error::NotFound` Strings + 5 kept
default Strings"; C5 removes the middle term for the 20 fields that fall
back to a default, landing at 20 + 0 + 5 = 25.

**The reproduction gotcha — recorded because it cost real time.** Adding
`[patch.crates-io] environs = { path = ... }` alone did *not* take
effect: cargo warned `patch was not used in the crate graph` and kept
resolving `environs v1.2.0`. It required `cargo update -p environs` to
force re-resolution onto the patched path source. Anyone re-proving this
row will hit the same thing, and a silently-unpatched run would have
produced a "no improvement" result that looked like a real negative
rather than an inert patch.

**Behaviour is the oracle.** After revert (patch removed, `environs`
back to crates.io 1.2.0): `cargo nextest run --workspace` — **243 tests,
243 passed, 0 skipped**; `cargo clippy --workspace --all-targets -- -D
warnings` — exit 0; `cargo test --doc -p conflaguration --features
derive,toml,codegen` — **26 doctests, exit 0**. In `environs` itself:
175 / 175 / 208 tests across default / no-default-features /
all-features feature sets, 8 clippy+build gates, all exit 0.

**Re-prove command:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json
```
with `[patch.crates-io] environs = { path = "<local environs 1.3.0
worktree>" }` in `Cargo.toml` plus `cargo update -p environs` (see the
reproduction gotcha above — the patch alone does not take effect), then
revert both before comparing to C4's shipped numbers.

**Status: LANDED IN `environs`, NOT YET REACHABLE FROM THIS CRATE.**
conflaguration depends on `environs = "1.2.0"` from crates.io
(`Cargo.toml:16`). The measured improvement requires environs 1.3.0 to be
published and that dependency bumped. Until then conflaguration's
real-world number is C4's 45, not 25 — the 25 above is measured-under-
patch, not shipped.

## C6 — named-incumbent comparison: envy, config-rs, figment (gates 6, 13)

**What this row is.** C1-C5 all measured conflaguration against conflaguration
— none of them carried a `design-favors: incumbent` arm, which gates 6 and 13
require. This row is not an optimisation tweak of conflaguration's own code;
it is that missing arm, added for the two domains C1-C5 actually cover — env
resolution (C1, C4, C5) and layered file merge (C3) — so those rows finally
have something to be compared against besides their own earlier selves.

**Added:** three home-turf incumbent dev-dependencies via `cargo add --dev`:
`envy 0.4.2`, `config 0.15.25` (`toml` feature), `figment 0.10.19` (`toml`
feature) (`Cargo.toml:35,33,36`). Six new bench functions in
`benches/bench_conflaguration.rs`, 20 -> 26 divan cases (`grep -c
'#\[divan::bench\]' benches/bench_conflaguration.rs` returns 26). Each
incumbent's real API was read from the registry cache before writing its arm,
not assumed from memory: `envy-0.4.2/src/lib.rs`,
`config-0.15.25/src/{env,file/mod,builder,config}.rs`,
`figment-0.10.19/src/{lib,providers/data}.rs`.

**Why the env arm is a fresh pair, not a reuse of `from_env_flat20`/`flat5`.**
Those existing arms exercise conflaguration's default-fallback path with no
env vars set — envy cannot do that without extra `serde(default)` plumbing,
so reusing them would compare an unfair pair. A matched pair on prefix
`BENCH_INCUMBENT20`/`BENCH_INCUMBENT5` was built instead, where both sides
read the same real, present env vars: conflaguration's `join_key(prefix,
FIELD_NAME)` and envy's `prefixed("BENCH_INCUMBENT20_")` produce
byte-identical env var names, so neither side is handicapped
(`benches/bench_conflaguration.rs:133-306`).

**Group A — env into struct, ~1 call/process. 3 runs each, median of
per-run medians:**

| arm | crate | median (3 runs) | range | allocs | peak | design-favors |
|---|---|---|---|---|---|---|
| `from_env_flat20_incumbent_conflaguration` | conflaguration | 6.794 µs | 6.755–7.199 µs, CoV ~2.9% | 48 alloc / 43 dealloc | 41 B | baseline pair |
| `from_env_flat20_incumbent_envy` | envy | 9.923 µs | 9.723–10.17 µs, CoV ~1.8% | 295 alloc / 290 dealloc | 18.11 KB | **incumbent (load-bearing)** |
| `from_env_flat5_incumbent_conflaguration` | conflaguration | 0.9898 µs | 0.9894–1.017 µs, CoV ~1% | 12 alloc / 11 dealloc | 26 B | baseline pair |
| `from_env_flat5_incumbent_envy` | envy | 6.529 µs | 6.439–6.834 µs, CoV ~2.6% | 225 alloc / 224 dealloc | 11.41 KB | **incumbent** |

At 20 fields conflaguration is 1.46x faster, 6.1x fewer allocations, 452x
lower peak bytes. At 5 fields, 6.6x faster, 18.75x fewer allocations, 439x
lower peak.

**Group B — layered merge, ~1 call/process, 2 TOML layers, identical file
content across all three, concurrent host load present (a proxima `cargo
check` was running, pid 24164):**

| arm | crate | median (3 runs) | range | allocs | peak | design-favors |
|---|---|---|---|---|---|---|
| `builder_merge_3_layer` (reused) | conflaguration | 29.45 µs | 29.2–40.16 µs, CoV ~19% | 73 / 73 | 2.609 KB | baseline |
| `builder_merge_2_toml_layers_incumbent_config` | config-rs | 51.46 µs | 45.95–66.4 µs, CoV ~15.8% | 67 / 67 | 3.435 KB | **incumbent** |
| `builder_merge_2_toml_layers_incumbent_figment` | figment | 33.07 µs | 31.53–41.44 µs, CoV ~12.3% | 76 / 76 | 4.767 KB | **incumbent** |

conflaguration is fastest of the three: ~12% faster than figment, ~75%
faster than config-rs — and conflaguration's arm includes an extra `.env()`
scan layer the incumbents do not. All three Group B CoV figures exceed the
5% report-as-range bar, so Group B is reported as ranges, not point values;
the concurrent host load noted above is the likely cause, per this file's
existing benching-is-taxed discipline.

**Mechanism, traced to source.** `envy::from_env` / `Prefixed::from_env`
(`envy-0.4.2/src/lib.rs:292-339`) always calls `std::env::vars()` first and
filters afterward — it enumerates the entire process environment and
allocates a `String` pair for every variable before the prefix check ever
runs. conflaguration's `environs::scan` (C5 row) does a targeted
`env::var(key)` per specific candidate key, with no full-environment
enumeration. The allocation gap (6-19x) and the peak-bytes gap (439-452x)
trace to that architectural difference — full-enumeration-then-filter vs
targeted-lookup — not to implementation quality. That is also why the gap
widens at 5 fields (6.6x) versus 20 (1.46x): envy pays a near-fixed cost to
enumerate the whole environment regardless of how many fields the target
struct has, so with fewer wanted fields the wasted enumeration dominates
more.

**The frequency reframe — the honest ceiling on this whole row.** Verified
by reading the API: `envy::from_env`/`Prefixed::from_env` and the layered
`ConfigBuilder` (both conflaguration's and the incumbents') resolve a
process's configuration once at boot. No per-request or per-event call site
exists anywhere in `src/` or the benches. Every arm above sits in the same
band: ~1 call per process. Weighting cost by that band, conflaguration is a
net win on every arm built here, but the absolute impact of any single win
is single-digit-to-low-double-digit microseconds, once, per process start —
noise against typical process startup (exec + dynamic link + main entry,
order of milliseconds). A per-request caller would never observe it. This is
a measurement with a traced mechanism (a result), not a decision; whether it
matters enough to act on is the owner's call, not this row's.

**What this seals and what it does not.**

- Seals gates 6 and 13 for the env-resolution path (C1, C4, C5) and the
  layered-merge path (C3): a `design-favors: incumbent` arm now exists,
  conflaguration meets-or-beats on it, and the mechanism is traced. C2
  (unused-dependency removal) has no comparable incumbent operation and
  remains unsealed on both gates.
- Does **not** seal gate 7 (E2E bench). These are still micro-benches, now
  with incumbent arms alongside conflaguration's own — nothing composes any
  of the three crates into a realistic application workload.
- Does **not** explain the original 31% link-time regression (`docs/discipline.md:132-147`).
  A per-operation speed win is a different axis from link/binary footprint;
  this row says nothing about why linking the crate regressed a downstream
  forward pass. C8 (`docs/discipline.md:809`) is the row that measures the
  binary axis directly.

**Behaviour is the oracle:** not applicable to this row — C6 adds only
dev-dependencies and bench functions, no `src/` or `crates/` change, so
there is no behaviour to re-verify against the existing test suite.

**Re-prove command:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json -- incumbent
```
Runs the six arms in this row in isolation; alloc/dealloc/peak rows are
printed directly under each bench in the divan table, same as every other
row in this log.

**Status: MEASURED — SEALS GATES 6 AND 13 FOR THE PATHS COVERED; NOT A
DECISION.**

## C7 — end-to-end service-boot bench (gate 7)

**What this row is.** C1-C6 all measured conflaguration against itself or
against a named incumbent, one function at a time — none of them measured
what a real service pays once, at boot, when every one of those functions
runs together. This row is not an optimisation of conflaguration's own code
either, same as C6: it is the missing e2e arm gate 7 asked for, so the
micro-wins in C1-C6 can be read in proportion to the cost they sit inside,
not in isolation.

**Added:** two new bench functions in `benches/bench_conflaguration.rs`,
section "6. end-to-end service boot" (`benches/bench_conflaguration.rs:670`),
26 -> 28 divan cases (`grep -c '#\[divan::bench\]' benches/bench_conflaguration.rs`
returns 28).

- `e2e_service_boot` (`benches/bench_conflaguration.rs:750`): untimed setup
  (`with_inputs`) writes a base TOML layer and an override TOML layer. The
  timed closure renders a `postgres://{{ env::.. }}:{{ env::.. }}@{{ env::.. }}:5432/{{ env::.. }}`
  database-URL template via `template::render(&EnvResolver)`, then chains
  `.file(base).file(override).env().override_with(|c| c.database.url =
  rendered).validate().build()` into a validated `ServiceConfig` (bind_host,
  bind_port, a nested `DatabaseConfig { url, pool_size }`, tls_enabled,
  timeout_seconds, worker_count).
- `e2e_service_boot_minimal` (`benches/bench_conflaguration.rs:773`): one
  file layer, no `.env()` pass, a single-token template render, the same
  validated build.

**Host load, noted per this file's benching-is-taxed discipline.** Both arms
ran under concurrent host load throughout: a proxima `cargo check`, pid
34329, load average 11.03 on a 10-core box. Medians stayed stable across the
3 runs despite a wide per-run fastest/slowest spread — the range column
below is wide, the median column is not.

| arm | median (3 runs) | range | CoV of medians | allocs | peak |
|---|---|---|---|---|---|
| `e2e_service_boot` | 114.6 / 116.5 / 128.3 µs | 98.07-314.7 µs | ~5.1% | 114 | 16.75 KB |
| `e2e_service_boot_minimal` | 71.12 / 78.31 / 75.41 µs | 51.6-253.6 µs | ~3.9% | 55 | 8.505 KB |

`e2e_service_boot`'s CoV sits just over the 5% report-as-range bar this file
uses elsewhere (e.g. C6's Group B, `docs/discipline.md:643`) — reported as a
range, not a point value. `e2e_service_boot_minimal`'s CoV sits under it.
Allocation counts were deterministic across all 3 runs on both arms: 114 and
55, no variance.

**Mechanism.** The two arms differ in exactly what they compose:
`e2e_service_boot` merges two file layers (`.file().file()`, each a
one-shot-deserialize-then-merge round trip per C3's mechanism), then an
`.env()` pass over the struct's fields (the C1/C4/C5 path), then one
multi-token template render (four `env::` substitutions), then
`.override_with()` + `.validate()` + `.build()`. `e2e_service_boot_minimal`
drops the second file layer, the `.env()` pass, and renders a single-token
template instead of a four-token one — fewer merge round trips, fewer env
lookups, fewer template substitutions, which is why its median (71-78 µs) and
allocation count (55) sit below the full arm's (114-128 µs, 114 allocs). The
gap between the two arms is the composed cost of exactly the layers dropped,
not a different code path.

**The proportion — this is the whole point of this row, stated as the honest
read.** The largest *shipped* micro-win in this log is C1's `from_env_flat20`
(`docs/discipline.md:149,171`): ~4.25-4.71 µs before, ~3.48-3.6 µs after,
roughly 0.65-1.2 µs saved per call. C4's shipped win on the same bench is
smaller, roughly 0.5 µs (`docs/discipline.md:359,409`). C5's roughly 0.3 µs
win is measured (`docs/discipline.md:460,523`) but explicitly not yet
reachable from this crate — it lives in an unpublished environs version this
crate does not yet depend on. Against the e2e boot's ~115-130 µs median,
C1's shipped win is roughly 0.6-1% of one boot. The scales differ in kind,
not only in size: a boot happens once per process, and the micro-win being
compared against it happens once per env-resolution call *inside* that same
boot. Summed, the five optimisation rows above (C1, C3 attempted-then-rolled-
back, C4, C5 not-yet-reachable) move a sub-2% slice of a cost that itself
happens once per process.

**What this seals and what it does not.**

- Seals gate 7: an e2e bench now exists and composes the crate — layered
  file merge, env override, template render, validated build — into one
  measured boot number, instead of leaving the crate's cost story as 28
  isolated per-function micro-benches.
- Does **not** produce a speedup. No `src/` or `crates/` code changed for
  this row; it adds measurement, not optimisation. The status line below
  says this explicitly so it cannot be misread as a seventh win.
- Does **not** explain the original 31% link-time regression
  (`docs/discipline.md:132-147`). This row measures wall-clock composition
  cost at runtime; the regression that excluded this crate is a link-time /
  binary-footprint axis, which C2 (`docs/discipline.md:200`) measured
  directly and found flat.

**Behaviour is the oracle:** not applicable to this row — C7 adds only
bench-local structs (`DatabaseConfig`, `ServiceConfig`) and two bench
functions to `benches/bench_conflaguration.rs`, no `src/` or `crates/`
change, so there is no behaviour to re-verify against the existing test
suite.

**Re-prove command:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json -- e2e_service_boot
```
Runs both arms in this row in isolation; alloc/dealloc/peak rows are printed
directly under each bench in the divan table, same as every other row in
this log.

**Status: MEASURED — SEALS GATE 7; IT IS THE DENOMINATOR, NOT A SPEEDUP.**

## C8 — link-footprint measurement: does dropping the environs tracing subtree move the linked binary

**What this row is.** C1-C7 measured per-operation speed (C1, C4, C5) or
crate-level properties (C2's binary-size check, on an unrelated unused
dependency, found zero delta; C6/C7 measure wall-clock). None of them
measured the axis the original 31% regression actually lives on: what a
downstream consumer's *linked binary* looks like with conflaguration's
`environs` dependency carrying its `tracing` feature versus not. This row
is that measurement.

**Setup.** A standalone consumer crate deriving a `Settings` struct and
calling `from_env()` through `std::hint::black_box`, depending on
conflaguration by path with `features=["derive"]`, built `cargo build
--release`. Four configurations, measured via `ls -l` (bytes), `size`
(`__TEXT`), `nm | wc -l` (symbols), and `cargo tree -e normal | sort -u |
wc -l` (unique tree lines):

| config | binary bytes | __TEXT | nm symbols | tree lines | tracing in tree |
|---|---|---|---|---|---|
| A. baseline (environs 1.2.0 from crates.io) | 513,824 | 311,296 | 943 | 28 | yes (3 hits) |
| B. environs 1.3.0 patched, tracing ON (default) | 497,264 | 294,912 | 942 | 29 | yes (3 hits) |
| C. environs 1.3.0 patched, tracing OFF | 497,264 | 294,912 | 942 | 22 | no (0 hits) |
| D. control, no conflaguration dep at all | 470,432 | 278,528 | 869 | 1 | n/a |

**Finding 1 — null result on the binary axis, stated as such.** B vs C (same
environs 1.3.0 source, only the `tracing` feature toggled — the clean
single-variable comparison) are byte-for-byte identical on all three
linked-binary metrics: 497,264 bytes / 294,912 `__TEXT` / 942 symbols. `cmp`
shows the raw files differ, but a sorted symbol-name diff (118 of 942
lines) shows every difference is a Rust SVH/metadata-hash suffix from the
two consumer crates living in different scratch dirs — not a code-size
change. Only the crate-graph count moves: the environs subtree drops 5
crates (`tracing`, `tracing-core`, `tracing-attributes`, `pin-project-lite`,
`once_cell`), tree lines 29 -> 22. This reproduces C2's pattern exactly
(`docs/discipline.md:200`): the dependency-count axis moves, the
linked-binary axis does not — the linker strips the unreachable tracing
code regardless of whether it is in the graph.

Confound to record honestly: A vs B is **not** a clean isolation — the
environs 1.3.0 snapshot also carries ~300 lines of unrelated
`resolve.rs`/`dotenv.rs`/`builder.rs` changes vs 1.2.0, so the A -> B delta
(16,560 bytes) reflects those too, not just the feature default. B vs C is
the clean comparison, and it is zero.

**Finding 2 — a real defect, and it is fixable.** A downstream consumer
cannot reach config C without editing conflaguration's own manifest. Proven
empirically: `Cargo.toml:16` declares `environs = "1.2.0"` with no
`default-features = false`, and conflaguration exposes no passthrough
feature in its `[features]` table (`derive`/`codegen`/`toml`/`json`/`yaml`
only). Cargo feature unification is additive/monotonic across the whole
graph — a downstream `environs = { default-features = false }` still
resolved `environs features=default,tracing`, because conflaguration's own
manifest requests environs' default features and no downstream manifest can
subtract a feature another crate in the graph requested. Config C was
reachable only by editing `Cargo.toml:16` directly (done and reverted
during measurement).

The fix, fully specified but **blocked on publish order**: change
`Cargo.toml` to `environs = { version = "1.3.0", default-features = false
}` and add a passthrough feature `tracing = ["environs/tracing"]`, off by
default. Then the tracing subtree leaves conflaguration's default graph and
a consumer opts in with `features=["tracing"]`. This cannot land until
environs 1.3.0 is published to crates.io — pointing conflaguration at an
unpublished version breaks the build today. Recorded as the next step
gated on the publish, not as done.

**The honest read for C8.** The 31% regression lives on the linked-binary /
runtime axis, and C8 shows that dropping the tracing subtree does **not**
move the linked binary at all (Finding 1) — so even once the passthrough
defect (Finding 2) is fixed, the reachable win is a 5-crate compile-graph
and audit-surface reduction, not a binary-size or runtime win. The 31%
regression was conflaguration+serde+clap together regressing a forward
pass; serde_derive and clap monomorphisation are heavier suspects than
conflaguration, and none of it is reproduced here. C8 does not explain the
regression; it rules out the tracing subtree as the mechanism for any
binary-size component of it, and surfaces a real but separate packaging
defect.

**Behaviour is the oracle:** not applicable to this row — C8 measures a
standalone external consumer crate and a temporary manifest edit that was
reverted; no `src/` or `crates/` change landed.

**Re-prove command:**
```
# build all four configs release, then per binary:
ls -l target/release/<consumer-bin>
size target/release/<consumer-bin>
nm target/release/<consumer-bin> | wc -l
cargo tree -e normal | sort -u | wc -l
```
Config C requires the manifest edit named in Finding 2 (`Cargo.toml:16` ->
`default-features = false`), applied and reverted for measurement.

**Status: MEASURED — null result on binary axis (Finding 1); real
reachability defect (Finding 2), fix blocked on environs 1.3.0 publish.
Does NOT explain the 31% regression.**

## Adversarial pass (Task 5)

**(a) Correctness.** Added four direct unit tests for `join_key`
(`src/lib.rs`, `join_key_empty_prefix_returns_bare_segment`,
`join_key_joins_with_underscore`, `join_key_empty_segment_still_underscores`,
`join_key_sizes_the_buffer_by_bytes_not_chars`) — the last asserts byte-exact
output on a multi-byte UTF-8 prefix (`"CAFÉ"`, é = 2 bytes), which would
expose a char-count-vs-byte-count capacity bug if one existed.
`cargo nextest run --all-features`: **235 tests, 235 passed** (231 -> 235,
the +4 new tests; N asserted, not assumed). No behavior changed.

**(b) Reproduction under reorder.** Re-ran the two moved benches in
isolation, filtered and reordered (`cargo bench ... -- from_env_flat20
join_key_with_prefix`, dropping every other bench from the run):
`from_env_flat20` median 3.478 µs, `join_key_with_prefix` median 12.96 ns —
both inside the three-run bands already recorded above. The win does not
depend on run order or on which other benches share the process.

**(c) Wrong-thing-measured check.** `from_env_flat20`/`from_env_nested3`
rows are explicitly labeled "downstream effect", not a `join_key`-only
claim — they measure the full `from_env()` cascade (env lookups + parsing +
`join_key` calls combined), which is what they say they measure. The
`join_key_with_prefix` row is the only row backing the isolated allocator
-op claim.

## Degenerate control (Task 5) — REQUIRED, run and reported

First attempt at the control was itself a bug, kept here rather than
deleted (principle: negative results are the valuable ones). Naively
writing `format!("{}_{}", "APP_DATABASE", "HOST")` inline with string
*literals* at the bench call site measured **1 alloc, 17 B, no grow** —
indistinguishable from the fixed `join_key`. That did NOT mean the fix was
fake; it meant the control was invalid: literal arguments in the same scope
let the compiler const-propagate their lengths, which a real `&str`
function parameter (opaque length) cannot get. Fixed by reproducing the
pre-fix body as its own function, `join_key_pre_fix(prefix: &str, segment:
&str) -> String { format!("{prefix}_{segment}") }`, called through
`std::hint::black_box` on both arguments (same treatment now applied to the
two real `join_key_*` benches for symmetry) — `benches/bench_conflaguration.rs`,
`join_key_control_old_format_shape`.

**Control result:** `join_key_control_old_format_shape` measured **1 alloc
(12 B) + 1 grow (12 B) = 24 B peak** — exactly the pre-fix two-allocator-op
pattern recorded in C1's baseline row, reproduced on demand. This confirms
the `AllocProfiler` instrumentation genuinely distinguishes the two code
shapes rather than reporting a constant; the C1 win is not a harness
artifact.

**Re-prove:**
```
cargo bench --bench bench_conflaguration --features derive,toml,json -- join_key
```

## Task 0 gate self-correction (not a perf tweak, recorded for completeness)

Adding `benches/bench_conflaguration.rs` initially broke `cargo clippy
--all-targets --all-features` (`unwrap_used`/`expect_used` denied
crate-wide, per this repo's `[lints.clippy]`, apply to `--all-targets`
including benches). Fixed with a scoped `#![allow(clippy::unwrap_used,
clippy::expect_used)]` at the top of the bench file plus a one-line why
comment, matching the existing pattern in `src/template.rs`'s and
`tests/*.rs`'s `#[cfg(test)] mod tests` blocks. Re-ran `cargo clippy
--all-targets --all-features` after the fix: **exit 0, 0 warnings.**
`cargo nextest run --all-features` unaffected throughout: **231/231.**
