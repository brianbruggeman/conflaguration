# xml-lenient -- slices

Each slice: one commit, one behaviour change, one validation command. Update the checkbox and the
note IN THE SAME COMMIT as the slice. Every cargo command runs as `timeout -k 30s 25m git wt cargo
...`, `CARGO_BUILD_JOBS=2`. `F6` = `toml,json,yaml,xml,ron,json5`.

| # | slice | discharges | validation command | expected | done | note |
|---|---|---|---|---|---|---|
| 0a | baseline at `68fd339` (CONFLAG-1) | AC6 | `cargo val --workspace --all-features --output plain` | 296 passed, 2 failed | [x] | 2026-11-20; both failures reproduce with `--features derive` and the default target dir |
| 0b | fix the two failing tests (CONFLAG-2), their own commits first | AC6 | `cargo val --workspace --all-features --output plain` | 298 passed, 0 failed | [x] | 2026-11-20 CONFLAG-2, on local main `beded1a`: trybuild 1.0.116 read features from `target/debug/.fingerprint`, which cargo 1.100's build-dir layout no longer writes, so fixtures lost `derive` (`50a89b0` bumps to 1.0.121); `no_std_graph` hard-coded `deps/` search paths, now taken from cargo's artifact records (`beded1a`) |
| 1 | `xml`, `ron`, `json5` features: `from_*_str`, `from_file` arms and re-export cfg, the six fixture twins, `check_fixture` | AC1, AC9, AC5 (`check_fixture` Ok and Err) | `cargo val --features F6 --test formats -E 'test(format_from_file_) or test(ctl_formats_)'`, then AC9 (a), (b), (c) | 8 passed (AC1 6 + 2 controls); AC9 1, 1, 1 | [ ] | |
| 2 | `Tree` (grouping, `$text` collapse, typed scalars, JSON conversion), `check_json` | AC4, AC5 (`check_json` Ok and Err, the two quick-xml controls) | `cargo val --features F6 --test formats -E 'test(format_tree_) or test(ctl_formats_)'` | 12 passed (AC4 6 + the 6 controls written so far) | [ ] | |
| 3 | builder layers for the three formats through `Tree`, the full lenient deserializer, `check_error` | AC2, AC8, AC3, AC5 (`check_error` Ok and Err) | `cargo val --features F6 --test formats -E 'test(format_layered_) or test(format_overlay_) or test(format_lenient_) or test(ctl_formats_)'` | 24 passed (AC2 6 + AC8 6 + AC3 4 + AC5 8) | [ ] | |
| 4 | public-surface check, full suite, defaults | AC6, AC7 | `cargo val --features F6,derive --test api_surface`; `cargo val --features derive --test compile_fail`; `grep -c '^default = \["std"\]$' Cargo.toml`; `cargo val --workspace --all-features --output plain` | api_surface 1 passed; compile_fail 1 passed (20 trybuild cases); default line 1; full suite 337 passed, 0 failed | [ ] | |

## resume

Last landed slice: 0b (green baseline on local main `beded1a`)
Next action: audit round 3
Open question, if any: none

## struck

-
