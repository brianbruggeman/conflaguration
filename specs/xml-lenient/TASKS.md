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
| 3 | builder layers for the three formats through `Tree`, the full lenient deserializer (`src/file.rs:138,150`), `check_error`, the overlay, F4-family and `Deep` fixtures (CONFLAG-5) | AC2, AC8, AC3, AC5 (`check_error` Ok and Err) | `cargo val --features F6 --test formats -E 'test(format_layered_) or test(format_overlay_) or test(format_lenient_) or test(ctl_formats_)'` | 28 passed (AC2 6 + AC8 6 + AC3 8 + AC5 8) | [x] | 2026-11-20 CONFLAG-5: 28 passed (worker `slice3.log`, independent `rerun-slice3.log`, after review `slice3-fix.log`); full suite 345 passed 0 failed 2 ignored (`full-fix.log`); clippy clean with all features and `std,xml` alone; slice 1's `cfg_attr(allow(unused_variables))` deleted; removed lines in `src/` are the 2 `T::deserialize(base_value)` calls of L13 plus that attribute's 3 slice-1 lines; review moved an inline `std::iter` path to an import and replaced two `ref` guards |
| 4 | `from_tree`, `lenient::{deserialize, option}`, the `Holder` and `Helped` fixtures (CONFLAG-6) | AC10, AC11, AC12 | `cargo val --features F6 --test formats -E 'test(format_from_tree_) or test(format_limit_) or test(format_helper_)'` | 12 passed (AC10 8 + AC11 1 + AC12 3) | [x] | 2026-11-20 CONFLAG-6: 12 passed (worker `slice4.log`, after review `slice4-fix.log`); full suite 358 passed 0 failed 2 ignored (`full-fix.log`); clippy clean with all features and `std,xml` alone; 0 removed lines in `src/`; review moved two inline `crate::` paths to imports. The `std,json`-only clippy run exposed slice 1's ungated `tests/formats_single.rs` items, fixed in its own commit before this one |
| 5 | incumbent checks, full suite | AC6, AC7 | `grep -c '^default = \["std"\]$' Cargo.toml`; `git diff -U0 beded1a HEAD -- src crates \| grep '^-' \| grep -v '^--- '`; `cargo val --workspace --all-features --output plain` | default line 1; exactly L13's 13 lines; full suite 358 passed, 0 failed | [x] | 2026-11-20 at `8a29d45`: AC7 (a) 1; AC7 (b) 13 lines, `diff` against the L13 block empty; AC6 358 passed 0 failed 2 ignored (`<tickets>/CONFLAG-6/evidence/ac6-full.log`, the run filtered only model families this crate lacks, 358 PASS lines, 0 FAIL); clippy clean under default, `std,json`, `std,toml`, `std,yaml`, `std,xml` and all features |

## resume

Last landed slice: 5 (spec complete; full suite 358 passed, 0 failed, 2 ignored)
Next action: release 2.1.0 to crates.io (owner's go; push of main first)
Open question, if any: none

## struck

- slice 3's first form (40 tests: builder layers, lenient deserializer, `from_tree`, R8 helpers in
  one commit), split 2026-11-20 into slices 3 (CONFLAG-5, 28) and 4 (CONFLAG-6, 12): too large for
  one 20-minute card. Old slice 4 is now slice 5; ACs and totals unchanged.

- R6's first form, "every existing public function keeps its exact signature", checked by
  function-pointer typing (`tests/api_surface.rs`) and a trybuild drift case: struck in round 4.
  A pointer with fixed type arguments still compiles after a new bound those arguments satisfy, so
  the check could not see the change it claimed to catch; replaced by the removed-line list L13.
