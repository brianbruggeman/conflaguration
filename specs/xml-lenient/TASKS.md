# xml-lenient -- slices

Each slice: one commit, one behaviour change, one validation command. Update the checkbox and the
note IN THE SAME COMMIT as the slice. Every cargo command runs as `timeout -k 30s 25m git wt cargo
...`, `CARGO_BUILD_JOBS=2`. `F6` = `toml,json,yaml,xml,ron,json5`.

| # | slice | discharges | validation command | expected | done | note |
|---|---|---|---|---|---|---|
| 0a | baseline at `68fd339` (CONFLAG-1) | AC6 | `cargo val --workspace --all-features --output plain` | 296 passed, 2 failed | [x] | 2026-11-20; both failures reproduce with `--features derive` and the default target dir |
| 0b | fix the two failing tests (CONFLAG-2), their own commits first | AC6 | `cargo val --workspace --all-features --output plain` | 298 passed, 0 failed | [x] | 2026-11-20 CONFLAG-2, on local main `beded1a`: trybuild 1.0.116 read features from `target/debug/.fingerprint`, which cargo 1.100's build-dir layout no longer writes, so fixtures lost `derive` (`50a89b0` bumps to 1.0.121); `no_std_graph` hard-coded `deps/` search paths, now taken from cargo's artifact records (`beded1a`) |
| 1 | `xml`, `ron`, `json5` features: `from_*_str`, `from_file` arms, the 8 widened gates of L13, `tests/formats_single.rs`, the six fixture twins, `check_fixture` | AC1, AC9, AC5 (`check_fixture` Ok and Err) | `cargo val --features F6 --test formats -E 'test(format_from_file_) or test(ctl_formats_)'`, then AC9 (a), (b), (c) | 8 passed (AC1 6 + 2 controls); AC9 1, 1, 1 | [x] | 2026-11-20 CONFLAG-3: 8 passed; AC9 1, 1, 1 (worker and independent re-run, `<tickets>/CONFLAG-3/evidence/{slice1,rerun-slice1,ac9-*,rerun-ac9-*}.log`); `from_xml_str` doc-test passes; removed lines are L13's 11 slice-1 lines. A build with only `xml` / `ron` / `json5` has no `file_to_value` arm reading `contents` until slice 3, so `file_to_value` carries a `cfg_attr(..., allow(unused_variables))`; slice 3 removes it |
| 2 | `Tree` (grouping, `$text` and TOML-datetime collapse, typed scalars, JSON conversion with non-finite floats as null), `check_json` | AC4, AC5 (`check_json` Ok and Err, the two quick-xml controls) | `cargo val --features F6 --test formats -E 'test(format_tree_) or test(ctl_formats_)'` | 14 passed (AC4 8 + the 6 controls written so far) | [x] | 2026-11-20 CONFLAG-4: 14 passed (worker `slice2.log`, after the review fix `slice2-fix.log`, independent `rerun-slice2.log`); F3J, F3X, TD, TI hold as written; clippy clean with all features and with `std,xml` alone; 0 removed lines in `src/` / `crates/`; review moved an in-function `use` in `src/tree.rs` to module top |
| 3 | builder layers for the three formats through `Tree`, the full lenient deserializer, `from_tree`, `lenient::{deserialize, option}`, `check_error`, the `Deep`, `Holder` and `Helped` fixtures | AC2, AC8, AC3, AC10, AC11, AC12, AC5 (`check_error` Ok and Err) | `cargo val --features F6 --test formats -E 'test(format_layered_) or test(format_overlay_) or test(format_lenient_) or test(format_from_tree_) or test(format_limit_) or test(format_helper_) or test(ctl_formats_)'` | 40 passed (AC2 6 + AC8 6 + AC3 8 + AC10 8 + AC11 1 + AC12 3 + AC5 8) | [ ] | must delete slice 1's `cfg_attr(..., allow(unused_variables))` on `file_to_value` once the three arms read `contents` |
| 4 | incumbent checks, full suite | AC6, AC7 | `grep -c '^default = \["std"\]$' Cargo.toml`; `git diff -U0 beded1a HEAD -- src crates \| grep '^-' \| grep -v '^--- '`; `cargo val --workspace --all-features --output plain` | default line 1; exactly L13's 13 lines; full suite 358 passed, 0 failed | [ ] | |

## resume

Last landed slice: 2 (CONFLAG-4; full suite 323 passed, 0 failed, 2 ignored)
Next action: slice 3 (CONFLAG-5)
Open question, if any: none

## struck

- R6's first form, "every existing public function keeps its exact signature", checked by
  function-pointer typing (`tests/api_surface.rs`) and a trybuild drift case: struck in round 4.
  A pointer with fixed type arguments still compiles after a new bound those arguments satisfy, so
  the check could not see the change it claimed to catch; replaced by the removed-line list L13.
