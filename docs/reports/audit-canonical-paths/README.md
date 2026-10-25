# AUDIT-CANONICAL-PATHS: unit selection by canonical path

Card of the pluggable-cards plan ([seam F, "Canonical paths"](../../plans/pluggable-cards/README.md#f-measurement-as-a-tool);
user decision 7 of 2026-10-24: canonicalise paths in the audit's unit
selection). Base `fb5412930`; implementation `26a1acd2d`, measured as TIP.
It changes no verdict.

## Problem

The checkout moved to `/media/Data/GitHub/quirin-lang`, and
`/home/cosmo/Documents/GitHub/qcore-c` became a symlink to it. The pinned
compile databases still spell the old prefix. Every route resolves its declared
root to the real path, then selected units by comparing path strings. So no
unit matched, and every corpus, system and slice route failed before analysis
(`PINNED-INPUTS-PATH-AFTER-MIGRATION`; run 4 of
[W48-BU-SINGLE-LEAF-BODIES](../w48-bu-single-leaf-bodies/README.md)). Cards
worked around it with a re-spelled private mirror (`mirror_inputs.py`).

## What changed

`crates/qcore-audit/src/input_paths.rs` holds one comparison for every
selection site. It compares **located** paths:

- A path that resolves becomes its canonical form. Symlinks are resolved, and
  `.` and `..` apply as the kernel applies them. So a database `directory`
  that is itself a symlink, and a relative `file` with `..`, resolve as Clang
  opens them.
- A path that does not resolve is located through its deepest existing
  ancestor, canonicalised, with the unresolved tail kept component by
  component. It is never treated as resolving. If it is selected, it stays in
  the denominator, and its unit is refused when it is read, as before.

The located form is for **matching only**. Each selected `CompileEntry` keeps
its spelled `directory`, `file` and `command`. Clang reads the recorded
spelling (constraint 11 is untouched), and every published path, compile-task
identity and digest is the one the route published before. A row's relative
path is the spelled one when the spelling lies under the root, and the
canonical one otherwise. Pinned-byte checks and contract pins (C06.2) still
compare content digests and are unchanged.

Fail-closed additions:

- **Outside the root.** A record spelled under a source root that resolves
  outside it is refused with the route's existing outside-root diagnostic. It
  is never admitted and never silently dropped. "Spelled under" includes a
  record spelled through a symlinked checkout whose last component, or a
  directory below the root, leads out.
- **Two spellings of one file.** Two selected records that spell one source
  file two ways, or one configuration's directory two ways, are refused with
  the named diagnostic `input-path-alias`. They are never merged by a dedup and
  never counted twice. The same holds for two selected files that locate to one
  source. One spelling repeated is not an alias: each route keeps its own rule
  for that.

### Selection sites

Every string comparison of input paths in `qcore-audit`:

| Site | Before | Now |
|---|---|---|
| `pinned_route.rs` `plan_library` (top-level, recursive, exact) | `parent() ==`, `starts_with`, `==` on spelled paths | `select_under` / `select_exact`, outside and alias refusals |
| `pinned_route.rs` unit file and cross-unit home key | `strip_prefix(source_root)` | `relative_within` (existing diagnostics) |
| `pinned_route.rs` Monkey plan | raw dedup | alias refusal before the audit |
| `current.rs` `select_configurations` | canonical, with a raw fallback for a dangling path | `select_under` / `select_exact` |
| `current.rs` `export_cross_unit_home` | `strip_prefix(checkout)` on the spelled path (the configured routes' failure) | `relative_within` |
| `current_route.rs` `select_file_configurations` (explicit selections) | canonical with raw fallback | `select_exact`, outside refusal, cross-file alias refusal |
| `current_route.rs` `is_monkey_unit`, Monkey expected source, `verify_monkey_source_copy` | `strip_prefix` on spelled paths | `place` / `relative_within` |
| `lib.rs` `select_profile_units` (configuration route) | canonical with raw fallback, conflicts keyed by spelling | `select_under` / `select_exact`, alias refusal |
| `compile_db.rs` `select_unique_configuration` (observed-wait inputs) | `entry.file == file` | `select_exact` (alias refused, never merged) |
| `outer_selection.rs` `compute_reach` | canonical with raw fallback | located match, alias refusal |
| `member_prepass.rs` own-input test | `==` | `same_existing_path`, as `compiler_options` already does |

Reviewed and left unchanged:

- `evaluate_entry` and `checkpoint_run`, which already canonicalise;
- the frame-helper import, which already matches canonically and refuses
  ambiguity;
- `native_platform_matrix.rs`, whose spelled paths are the recorded native
  evidence's identity, so it compares them exactly by design;
- `quirin-audit-plugin-libevent`, a separate crate that selects from a database
  it generates itself against the canonical checkout.

No standing constraint describes the audit's selection by path, so no rule
text changes. Constraint 11 ("do not normalize the spelled input path") holds:
the spelling given to Clang and recorded in identities is untouched.

### Tests

- `input_paths` unit tests: a symlinked spelling locating to the real root;
  escape; alias versus repetition; a relative file under a symlinked directory;
  no lexical normalisation past a missing component.
- `pinned_route` `canonical_selection_tests`: the old spelling selects the
  units of the real root, in the old order and with the old spelling kept, and
  publishes the real relative path; escaping, dangling and aliased records.
- `tests/canonical_paths.rs`, end to end through `quirin-audit current` with a
  cross-unit link map:
  - a database spelled through a symlinked checkout gives the same verdicts as
    the real spelling (before the card it failed with `cross-unit home lies
    outside checkout`), and its records keep the old spelling;
  - `{"directory": OLD/build, "file": "../src/X.c"}`;
  - outside-root refusal;
  - dangling refusal;
  - `input-path-alias` refusal, and one spelling repeated still counted once
    per command.

IDENTITY_SECTION

GATE_SECTION

## Boundaries and follow-ons

- A compile-task identity binds the record's spelled `directory`, `file` and
  `command`, so the same database spelled two ways gives two identities and
  two sets of identity digests. That is constraint 11's intent: the spelling
  reaches `__FILE__`. A checkpoint (`.qo`) sealed under one spelling therefore
  does not match a run under another. This card does not change that.
- The scheduler card's `mirror_inputs.py` and `measure.py` still work (TIP
  mirror side) and are no longer needed. Retiring the copies is the
  `MEASURE-TOOL` card's.
- `quirin-audit-plugin-libevent` compares a self-generated database's paths by
  string. It is unaffected by the move, and it is outside this crate.
