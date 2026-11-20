# xml-lenient -- slices

Each slice: one commit, one behaviour change, one validation command. Update the checkbox and the
note IN THE SAME COMMIT as the slice. Every cargo command runs as `timeout -k 30s 25m git wt cargo
...`, `CARGO_BUILD_JOBS=2`.

| # | slice | discharges | validation command | expected | done | note |
|---|---|---|---|---|---|---|
| 0a | baseline at `68fd339` (CONFLAG-1) | AC6 | `git wt cargo val --workspace --all-features --output plain` | 296 passed, 2 failed | [x] | 2026-11-20; both failures reproduce with `--features derive` and the default target dir: trybuild 19/19 "cannot find derive macro `Settings`", `tests/no_std_graph.rs:125` E0463 |
| 0b | fix the two failing tests (CONFLAG-2), its own commit first | AC6 | the same | 298 passed, 0 failed | [ ] | |
| 1 | `xml`, `ron`, `json5` features: `from_*_str`, `from_file` arms, the six fixture twins | AC1, AC5 (first two controls) | `git wt cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_from_file_) or test(ctl_formats_)'` | 6 + controls passed | [ ] | |
| 2 | builder layers for the three formats: grouped XML tree, lenient deserialize | AC2, AC3, AC5 (third control) | `... -E 'test(format_layered_) or test(format_lenient_)'` | 8 passed | [ ] | |
| 3 | `Tree` free-form value | AC4, AC5 (fourth control) | `git wt cargo val --features xml,json,toml --test formats -E 'test(format_tree_)'` | 2 passed | [ ] | |
| 4 | full suite + default features | AC6, AC7 | AC6 and AC7 | post-0b count + 20, 0 failed; no `default` diff | [ ] | |

## resume

Last landed slice: 0a (baseline measured)
Next action: CONFLAG-2 result into 0b; then audit this spec
Open question, if any: none

## struck

-
