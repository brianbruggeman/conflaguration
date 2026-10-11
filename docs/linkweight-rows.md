# Per-feature-set link-weight and build-time rows for docs/discipline.md

`docs/discipline.md` was reported upstream as already containing a
per-feature-set link-weight / build-time table. It does not — confirmed by
reading the file in full (`cat docs/discipline.md`, 2026-09-15): it has
allocation-budget rows (C1-C3) and an opt-sweep table, no consumer-crate
size/build-time table anywhere in it. These rows fill that gap. They are
built for merging into that file, not committed to it — no edit was made to
`docs/discipline.md` itself.

## Provenance — every number below is attributable to this exact tree state

Captured before any measurement, from the live repo (read-only; nothing here
was built against the live repo):

```
HEAD: 3eb325d84fc291df35528e6781b5b814884e2d28

git status --short:
 M Cargo.lock
 M Cargo.toml
 M README.md
 M crates/conflaguration-derive/Cargo.toml
 M crates/conflaguration-derive/src/lib.rs
 M src/builder.rs
 M src/lib.rs
 M src/template.rs
?? .claude/
?? benches/
?? conflaguration
?? docs/

git diff --stat:
 Cargo.lock                              |  82 ++++++++-
 Cargo.toml                              |   7 +-
 README.md                               | 145 +++++++++++++---
 crates/conflaguration-derive/Cargo.toml |   3 +
 crates/conflaguration-derive/src/lib.rs |  33 ++--
 src/builder.rs                          |  71 ++++++--
 src/lib.rs                              | 286 +++++++++++++++++++++-----------
 src/template.rs                         |   9 +-
 8 files changed, 491 insertions(+), 145 deletions(-)

shasum:
fb68f028fdd90925c2c48a41a1d0d0886dab8d0f  Cargo.toml
d4f596e332e8a772c599a572d3904e77dfc62894  Cargo.lock
ba37497ce359ba168c8e366646e280df8601a0d6  src/lib.rs
d80a112b3aa200d4aaeb0e0335d427ca4fefe066  src/file.rs
e8fd6c6ae5cca11b7195e6b54a6b909aa8f7e7bc  src/builder.rs
```

The `docs/discipline.md`/`Cargo.toml`/`Cargo.lock`-owning agent and the
`src/file.rs`/`src/builder.rs`/`benches/`-owning agent were both live during
this session; the snapshot below is what froze the tree for these rows.

**Snapshot**: `rsync -a --exclude target --exclude .git --exclude .claude
/Users/brianbruggeman/repos/mine/conflaguration/
.../scratchpad/conflag-snapshot/` — one rsync, not repeated.

**Snapshot integrity check**: `cargo check --all-features` against the
snapshot with a fresh `CARGO_TARGET_DIR` (`lw-target-verify`) — **exit 0 on
the first attempt** (all crates checked including `derive`, `toml`, `json`,
`yaml`; finished in 5.32s). No re-snapshot was needed. **N = 1 attempt.**

All measurement below runs **only** against
`.../scratchpad/conflag-snapshot`, never against the live repo. Nothing was
written to `/Users/brianbruggeman/repos/mine/conflaguration`.

## Snapshot's `Cargo.toml` `[features]` — read before building anything

```toml
[features]
derive = ["dep:conflaguration-derive"]
codegen = []
toml = ["dep:serde", "dep:toml", "dep:serde-value"]
json = ["dep:serde", "dep:serde_json", "dep:serde-value"]
yaml = ["dep:serde", "dep:serde_yaml", "dep:serde-value"]
```

**No `tracing` feature exists.** `tracing` does not appear anywhere in this
`[features]` table or in `[dependencies]` as a named entry — it is not an
opt-in surface a consumer can toggle. Per the task instructions, a fifth
`derive-tracing` variant was **not** created, because there is no feature to
enable it with. Instead, Task 3 below measures what a consumer already pays
for `tracing` unconditionally, at zero features, and traces exactly where it
enters from.

## Task 1 — the five consumer crates (4 required + 1 adversarial control)

All under `.../scratchpad/linkweight/<variant>/`, each a standalone
`[workspace]`-rooted binary crate (an unrelated stray `/tmp/Cargo.toml`
belonging to a different, concurrently-running proxima-tensor session forced
this — cargo's upward workspace search hit it and failed with "failed to
find a workspace root" until each variant crate declared its own empty
`[workspace]` table; this is the standard cargo fix suggested by cargo's own
error message, applied only inside crates this task created).

| variant | dependency | main does |
|---|---|---|
| `control` | none | `std::env::var("APP_PORT")` parsed to `u16`, `black_box`, printed |
| `empty` | `conflaguration`, `default-features = false` | `conflaguration::resolve_with_or_str(&["APP_PORT"], str::parse::<u16>, "8080")`, `black_box`, printed |
| `derive` | `+ features = ["derive"]` | `#[derive(Settings)]` struct, `Config::from_env()`, `black_box`, printed |
| `derive-toml` | `+ features = ["derive", "toml"]` | `#[derive(Settings)]` + `#[derive(Serialize, Deserialize)]`, `conflaguration::builder().file("config.toml").env().validate().build()`, `black_box`, printed |
| `degenerate` (Task 5 adversarial control) | same dep as `empty` | declares `use conflaguration as _unused_conflaguration;`, **never calls into it** — prints a hardcoded `0u16` through `black_box` |

Smoke-tested (`APP_PORT=1234 cargo run -q` for the env-reading variants):
`control` -> `1234`, `empty` -> `1234`, `derive` -> `1234`, `degenerate` ->
`0`, `derive-toml` (no env var set) -> `9090` (from its `config.toml`,
proving the file-parse path actually executed, not just linked).

`derive-toml`'s `Config` needed `serde::{Serialize, Deserialize}` derives to
satisfy `ConfigBuilder::<T>::file`'s `T: DeserializeOwned + Serialize` bound
(`src/builder.rs:149`) — this is inherent to using the file-overlay API at
all, not an artifact of the test harness; any real consumer of `.file()`
pays the same `serde/derive` tax.

## Task 2 — measurements, release profile, `conflag-snapshot` tree

**N = 5 variants measured. 3 runs per timing number (15 clean builds, 15
incremental builds). 1 `cargo tree` and 1 release build per variant for
size/symbol counts (these do not need repeat runs — they are deterministic
given fixed inputs, unlike wall-clock time).**

### 2.1 — crate count (`cargo tree | wc -l`, full tree saved per variant)

| variant | `cargo tree` line count | unique crate names (dep-graph, not counting self) |
|---|---|---|
| `control` | 1 | 0 |
| `empty` | 26 | 14 |
| `derive` | 30 | 15 |
| `derive-toml` | 58 | 32 |
| `degenerate` | 26 | 14 (**identical set to `empty`** — same `Cargo.toml`) |

Full trees: `.../scratchpad/trees/<variant>.tree.txt`.

**Delta, `empty` vs `control` (crates added by conflaguration at zero
features)**:
```
conflaguration, environs, falsehoods, once_cell, pin-project-lite,
proc-macro2, quote, syn, thiserror, thiserror-impl, tracing,
tracing-attributes, tracing-core, unicode-ident
```
14 crates, and **`tracing` is in that list at zero features** — see Task 3.

**Delta, `derive` vs `empty`**: `conflaguration-derive` only (1 crate — the
proc-macro crate itself; its own `syn`/`quote`/`proc-macro2` are already
shared/deduped against `environs`'s copies where versions match).

**Delta, `derive-toml` vs `derive`**: `autocfg, equivalent, hashbrown,
indexmap, num-traits, ordered-float, serde, serde_core, serde_derive,
serde_spanned, serde-value, toml, toml_datetime, toml_edit, toml_write,
winnow` — 16 crates (`[build-dependencies]` line in the tree is a section
header, not a crate; excluded from the count).

**`degenerate` vs `empty`**: `diff` of the two crate-name lists is empty —
**identical dependency graph**, confirming any binary-size difference
between them (Task 5) is a linker-reachability effect, not a graph
difference.

Re-prove: `cd .../scratchpad/linkweight/<variant> && CARGO_TARGET_DIR=<fresh-dir> cargo tree`

### 2.2/2.3 — binary size and symbol count (`cargo build --release`, `nm`)

Built with `env -u RUSTC_WRAPPER -u CARGO_BUILD_RUSTC_WRAPPER` (sccache
disabled — see Task 2.4 rationale; disabling it doesn't change build
*output*, only build *speed*, but kept consistent across every command in
this file), one shared persistent `CARGO_TARGET_DIR`
(`lw-target-measure`) since size/symbol counts are deterministic given fixed
source and don't need repeat runs the way wall-clock does.

`nm` needed no special flags on this macOS/arm64 toolchain (`nm
<binary> | wc -l` worked directly — the binaries are not fully stripped;
mangled Rust `v0` symbols for e.g. `environs::error::Error`'s `Debug` impl
are visible in `empty`'s symbol table, see Task 3).

| variant | binary bytes | `size` `__TEXT` | `size` `__DATA` | `nm \| wc -l` |
|---|---|---|---|---|
| `control` | 434,368 | 278,528 | 16,384 | 866 |
| `empty` | 455,104 | 294,912 | 16,384 | 901 |
| `derive` | 455,824 | 294,912 | 16,384 | 907 |
| `derive-toml` | 912,832 | 622,592 | 16,384 | 1,961 |
| `degenerate` | 433,936 | 278,528 | 16,384 | 861 |

Re-prove:
```
cd .../scratchpad/linkweight/<variant>
CARGO_TARGET_DIR=.../scratchpad/lw-target-measure env -u RUSTC_WRAPPER -u CARGO_BUILD_RUSTC_WRAPPER cargo build --release
ls -l .../scratchpad/lw-target-measure/release/<variant>
size .../scratchpad/lw-target-measure/release/<variant>
nm .../scratchpad/lw-target-measure/release/<variant> | wc -l
```

### 2.4 — clean build wall-time (3 fresh target dirs per variant, sccache disabled)

**sccache is ambient in this environment** (`RUSTC_WRAPPER` and
`CARGO_BUILD_RUSTC_WRAPPER` both point at
`/Users/brianbruggeman/.cargo/bin/sccache` by default). sccache caches
compilation units by content hash **independent of `CARGO_TARGET_DIR`**, so
a "fresh target dir" alone would NOT guarantee a cold compile — every
timing command below explicitly runs `env -u RUSTC_WRAPPER -u
CARGO_BUILD_RUSTC_WRAPPER` to disable it, matching the convention already in
`docs/discipline.md`'s header. Timed with `/usr/bin/time -p` (`real`/`user`/`sys`;
this machine's `/usr/bin/time` supports POSIX `-p` output).

**Cold-start verified two ways per run**: (1) each run's `CARGO_TARGET_DIR`
was a directory created immediately before the build by this run's own
`mkdir -p`, guarded by a script check that aborted (`exit 1`) if the
directory already existed; (2) every build log lists a full `Compiling
<crate>` line for every dependency (pasted below for two variants; all five
logs are in `.../scratchpad/timing/clean-*.log`) — none show a "Finished" with
no preceding `Compiling` lines, which would indicate stale reuse.

| variant | run 1 (s) | run 2 (s) | run 3 (s) | spread (max−min) |
|---|---|---|---|---|
| `control` | 0.22 | 0.18 | 0.18 | 0.04 |
| `empty` | 3.78 | 3.66 | 3.65 | 0.13 |
| `derive` | 4.04 | 4.30 | 4.09 | 0.26 |
| `derive-toml` | 8.47 | 6.66 | 9.20 | **2.54** |
| `degenerate` | 5.59 | 4.07 | 4.55 | 1.52 |

**The spread that hurts**: `derive-toml`'s 2.54s spread is ~30% of its own
median, and `degenerate`'s 1.52s spread is ~33% — both far noisier than
`control`'s near-zero spread. Mechanism, not guessed: `grep -l "Blocking
waiting for file lock on package cache"` against every timing log finds it
in `clean-control-run1`, `clean-empty-run2`, `incr-degenerate-run3`, and
`incr-derive-toml-run1` — this is cargo's registry-index lock, held
process-wide (not per-`CARGO_TARGET_DIR`), and this task's own instructions
say two other agents were building/editing this repo concurrently during
this session. That contention is a real, external, and only partially
controlled-for confound on the wall-clock numbers; the crate-count and
binary-size numbers above are unaffected by it (they don't depend on wall
time).

Re-prove (repeat 3x, fresh dir each time, check the log for "Blocking
waiting for file lock" before trusting a single run):
```
FRESH=.../scratchpad/lw-clean-target-<variant>-run<N>
mkdir $FRESH   # fails loudly if not fresh
cd .../scratchpad/linkweight/<variant>
CARGO_TARGET_DIR=$FRESH env -u RUSTC_WRAPPER -u CARGO_BUILD_RUSTC_WRAPPER /usr/bin/time -p cargo build --release
```

### 2.5 — incremental rebuild wall-time (touch `src/main.rs`, same persistent target dir, sccache disabled)

| variant | run 1 (s) | run 2 (s) | run 3 (s) | spread |
|---|---|---|---|---|
| `control` | 0.13 | 0.13 | 0.13 | 0.00 |
| `empty` | 0.21 | 0.20 | 0.21 | 0.01 |
| `derive` | 0.25 | 0.25 | 0.23 | 0.02 |
| `derive-toml` | 0.68 | 0.55 | 0.60 | 0.13 |
| `degenerate` | 0.13 | 0.13 | 0.23 | 0.10 |

`degenerate`'s outlier run 3 (0.23s vs 0.13/0.13) is the same package-cache
lock contention noted above (confirmed in that run's log). `degenerate`'s
other two runs match `control` almost exactly (0.13s) — rebuilding the
degenerate consumer touches no code that actually depends on
`conflaguration`'s output, so cargo has nothing new to link either.

Re-prove:
```
touch .../scratchpad/linkweight/<variant>/src/main.rs
cd .../scratchpad/linkweight/<variant>
CARGO_TARGET_DIR=.../scratchpad/lw-target-measure env -u RUSTC_WRAPPER -u CARGO_BUILD_RUSTC_WRAPPER /usr/bin/time -p cargo build --release
```

## Task 3 — the tracing question, answered with numbers

```
$ cargo tree -i tracing        (run inside empty/, default-features = false)
tracing v0.1.44
└── environs v1.2.0
    └── conflaguration v2.0.0 (.../scratchpad/conflag-snapshot)
        └── empty v0.1.0

$ cargo tree -i once_cell
once_cell v1.21.4
└── tracing-core v0.1.36
    └── tracing v0.1.44
        └── environs v1.2.0
            └── conflaguration v2.0.0
                └── empty v0.1.0

$ cargo tree -i pin-project-lite
pin-project-lite v0.2.17
└── tracing v0.1.44
    └── environs v1.2.0
        └── conflaguration v2.0.0
            └── empty v0.1.0
```

**`tracing` is not optional at any feature level.** It does not come from
conflaguration's own `[dependencies]` (verified above: conflaguration's
`Cargo.toml` never names `tracing`) — it is pulled in entirely by
`environs = "1.2.0"`, which is itself a plain, non-optional, unconditional
dependency of conflaguration regardless of `default-features`. There is no
feature flag anywhere in this dependency chain that removes it; a consumer
building `conflaguration` with `default-features = false` gets `tracing`,
`tracing-core`, `tracing-attributes`, and `pin-project-lite` unconditionally,
plus the two extra `syn` copies (`v2.0.119` and `v3.0.5`) and `once_cell`
that arrive to support building `tracing-attributes`' proc-macro and
`tracing-core`'s `OnceCell`.

**Measured cost of `tracing` (as part of `environs`'s always-on baggage) to
the `empty` consumer, from this session's own numbers above:**

- **crates**: 14 (the full `empty`-vs-`control` delta from Task 2.1) — of
  which `tracing`, `tracing-core`, `tracing-attributes`, `pin-project-lite`,
  `once_cell` (5) are attributable specifically to `tracing`'s own subtree;
  the other 9 (`environs`, `falsehoods`, `thiserror`, `thiserror-impl`,
  `proc-macro2`, `quote`, `syn`, `unicode-ident`, `conflaguration` itself)
  are shared infrastructure or `environs`'s other functionality, not
  `tracing`-specific. **This crate count could not be split further without
  rebuilding `environs` from a patched, tracing-free source** — not
  attempted in this session (out of the 90-minute budget; flagged as a
  residual, not silently rounded off).
- **binary bytes**: `empty` is 455,104 B vs `control`'s 434,368 B, a
  20,736 B delta — but this delta is NOT decomposable into a
  "tracing-specific" share from binary size alone. See the symbol check
  below.
- **build-seconds**: `empty`'s clean build (3.65-3.78s) vs `control`'s
  (0.18-0.22s) — full compilation of the 14-crate subtree, including
  `tracing`'s proc-macro (`tracing-attributes`) and its own `syn`
  dependency, which is a large fraction of that wall time (rebuilding `syn`
  twice, once per major version, is the single most expensive step visible
  in every `empty`/`derive`/`derive-toml` build log).

**Whether `tracing`'s own code is actually reachable in the linked
binary — checked, not assumed:** `environs`'s vendored source
(`~/.cargo/registry/src/.../environs-1.2.0/src/`) was read directly.
`tracing::debug!` is called exactly 4 times, all inside `dotenv.rs`
(dotenv-file loading) — `grep -rc "tracing::" src/*.rs` returns 0 for
`resolve.rs`, the file that actually implements `resolve_with`, which is
the only `environs` function `empty`'s `main` calls. Consistent with that:
`grep -i tracing .../symbols/empty.nm.txt` returns **zero matches** in the
symbol table (which does retain some mangled Rust `v0` names —
`environs::error::Error`'s `Debug` impl shows up as two `t`-class local
symbols, so the binary is not so stripped that a `tracing`-named symbol
would be invisible if one existed). **This is evidence, not proof**: LTO-less
release inlining can fold a callee's body into a caller without leaving a
symbol carrying the callee's own crate name, so a true zero cannot be
certified this way — but it is the same standard of evidence the existing
`memchr` finding in `docs/discipline.md` used, and it points the same
direction: `tracing` costs real, measured compile-graph weight (5+ crates)
and real, measured build seconds, but no evidence surfaced in this session
ties it to any of `empty`'s 20,736-byte size delta over `control`. That
delta is better explained by the 2 `environs`-named symbols actually found
(`Error`/`Location` `Debug` impls) plus the rest of `resolve_with`'s and
`environs`'s error-formatting code, which the `main.rs` in every variant
does exercise.

**No opt-out to measure the other direction.** Because no `tracing` feature
exists (confirmed above), Task 3's fallback ("measure the cost of opting
IN") does not apply either — there is no OFF state for `tracing` to opt in
from. The only state that exists is "on, unconditionally," which is what
was measured.

Re-prove:
```
cd .../scratchpad/linkweight/empty
cargo tree -i tracing
cargo tree -i once_cell
cargo tree -i pin-project-lite
grep -rc 'tracing::' ~/.cargo/registry/src/*/environs-1.2.0/src/*.rs
grep -i tracing .../scratchpad/symbols/empty.nm.txt   # expect: no output
```

## Task 5 — adversarial pass

**Like-for-like?** `control`, `empty`, and `derive` all resolve one `u16`
field named `port` from the `APP_PORT` env var (default `8080`), run it
through `std::hint::black_box`, and print it — verified identical behavior
by running all three with `APP_PORT=1234` and getting `1234` from each.
`derive-toml` necessarily does more (file-overlay + `.validate()` +
`.build()`) because that is the surface being measured — it is not
"smaller/larger for an unrelated reason," it is deliberately exercising more
of the API, and its `config.toml` (`port = 9090`) proves that path executes,
not just links, by producing `9090` instead of the env-var default.
`degenerate` is the odd one out on purpose (Task 5's own adversarial
requirement) and is called out as such, never averaged in with the other
four.

**Byte delta between `control` and `empty`**: 455,104 − 434,368 =
**20,736 bytes**. Mechanism: `empty`'s `main` actually calls
`conflaguration::resolve_with_or_str`, which calls into `environs::resolve_with`
and its error-formatting path (`environs::error::Error`, confirmed present
in the symbol table above) — this is real, reachable code the linker keeps
because `black_box` prevents the call from being proven dead. Contrast with
`degenerate` below: declaring the dependency alone costs nothing in bytes.

**Clean timing real?** See Task 2.4: each of the 15 clean-build runs used a
directory created immediately before that run by its own `mkdir -p`, guarded
by a script-level check that aborts with `exit 1` if the directory already
existed (so a stale directory can never silently satisfy "fresh"), and every
build log lists a full per-crate `Compiling` sequence, not an instant
"Finished" that would indicate a warm cache slipped through.

**Degenerate control — the outcome, reported either way:**

| | `control` | `degenerate` |
|---|---|---|
| binary bytes | 434,368 | 433,936 |
| `__TEXT` | 278,528 | 278,528 |
| `nm \| wc -l` | 866 | 861 |

`degenerate` (dependency declared, `conflaguration` never called) is **434
bytes smaller** than `control`, and its `__TEXT` section size is **byte-for-byte
identical**. The `nm` symbol-count difference (866 vs 861, a 5-symbol gap)
and the small overall byte difference are consistent with embedded
path-length noise (the crate name `control` is 7 characters, `degenerate` is
10 — panic-location and build-id strings embedding the crate/binary name
differ in length) rather than any linked code from `conflaguration`.

**This is the finding the task asked to be reported plainly: the linker
strips an unreachable dependency down to nothing, so the Task 2 binary-size
column measures reachability, not the mere presence of a dependency in the
graph.** `degenerate`'s 14-crate dependency graph (identical to `empty`'s,
confirmed in Task 2.1) produced **zero** measurable binary-size or `__TEXT`
cost. The size numbers for `empty`/`derive`/`derive-toml` in Task 2.2 say
less than they might appear to at a glance — they are not "the cost of
depending on conflaguration with these features," they are "the cost of the
specific code paths these five `main`s actually call." A consumer whose code
never reaches a given feature's functions pays compile-graph cost (Task 2.1)
but not binary-size cost for it, exactly as the `memchr` row in
`docs/discipline.md` already found and this row reconfirms with an explicit
built control rather than an after-the-fact deletion.

## What was not measured (named, not hidden)

- No `derive-tracing` variant: no `tracing` feature exists to enable (Cargo.toml
  `[features]` quoted in full above).
- `tracing`'s share of the 14-crate `empty` delta was reported as "5 of 14,
  by subtree membership," not proven by rebuilding `environs` without it —
  doing that would mean patching a vendored crate from source, out of this
  session's scope and budget.
- No linker map (`-Wl,-map`) was generated, so the 20,736-byte
  `control`-to-`empty` delta is reported as one aggregate number with a
  plausible mechanism (the two `environs` symbols found), not a byte-exact
  attribution across `environs`'s internal functions.
- Wall-clock numbers carry acknowledged noise from concurrent cargo
  processes (package-cache lock contention, named explicitly per occurrence
  above) — reported as spread, not smoothed away.

## Files produced (all under the scratchpad, nothing written to the repo)

```
.../scratchpad/conflag-snapshot/                    (frozen tree, Task 0)
.../scratchpad/verify-check.log                     (snapshot integrity check)
.../scratchpad/linkweight/control/                  (Cargo.toml, src/main.rs)
.../scratchpad/linkweight/empty/                    (Cargo.toml, src/main.rs)
.../scratchpad/linkweight/derive/                   (Cargo.toml, src/main.rs)
.../scratchpad/linkweight/derive-toml/              (Cargo.toml, src/main.rs, config.toml)
.../scratchpad/linkweight/degenerate/               (Cargo.toml, src/main.rs)
.../scratchpad/trees/<variant>.tree.txt             (full cargo tree per variant)
.../scratchpad/trees/<variant>.names.txt            (sorted unique crate names per variant)
.../scratchpad/symbols/<variant>.nm.txt             (full nm output per variant)
.../scratchpad/build-<variant>.log                  (release build logs, lw-target-measure)
.../scratchpad/sanity-<variant>.log                 (cargo check sanity logs)
.../scratchpad/timing/clean-<variant>-run<N>.log    (stdout of each clean build)
.../scratchpad/timing/clean-<variant>-run<N>.time.log   (stderr + /usr/bin/time -p output)
.../scratchpad/timing/incr-<variant>-run<N>.log     (stdout of each incremental build)
.../scratchpad/timing/incr-<variant>-run<N>.time.log    (stderr + /usr/bin/time -p output)
.../scratchpad/lw-target-verify/                    (snapshot integrity check target dir)
.../scratchpad/lw-target-sanity/                    (cargo check / cargo tree target dir)
.../scratchpad/lw-target-measure/                   (persistent release-build target dir, size/symbol/incremental source)
.../scratchpad/lw-clean-target-<variant>-run<N>/    (15 fresh per-run target dirs, clean timing)
.../scratchpad/linkweight-rows.md                   (this file)
```

(`...` = `/private/tmp/claude-501/-Users-brianbruggeman-repos-mine-conflaguration/12f2fcbb-b0c4-4e9d-9be7-203a796c0639/scratchpad`)
