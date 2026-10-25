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

## Identity run

[attempts.json](attempts.json) lists both attempts. Attempt 2 is the evidence.
Its drivers are in [scripts/](scripts/), and every comparison is in
[identity/](identity/).

### Sides

| Side | Binary | Inputs | Driver |
|---|---|---|---|
| base-mirror | BASE `fb5412930` | the re-spelled mirror (`mirror_inputs.py`) | the scheduler card's `measure.py`, unchanged |
| tip-mirror | TIP `26a1acd2d` | the same mirror | the same |
| tip-original | TIP | the ORIGINAL pinned inputs, unmirrored: the real `build/cycle19` and both prepared databases in place, all spelling `/home/cosmo/Documents/GitHub/qcore-c` | `scripts/measure.py --inputs original` |
| base-namespace | BASE | the same original inputs, where that spelling is a read-only bind mount of the checkout rather than a symlink (`pin_run.sh OLD=yes`). This is the environment the inputs were prepared in, so BASE needs no mirror. | `scripts/measure.py --inputs namespace` |

Each side has its own `git archive` snapshot, which carries the pinned libevent
submodule. Every side is built and run in a private mount namespace, with its
snapshot at `private/acp/snap` and its output at `private/acp/out`. So all
sides spell analyzer, profiles, contracts and outputs the same way, and they
differ only in how the inputs are spelled. The fixed-path builds are
reproducible: both BASE snapshots give binary `74c984eb…` and both TIP
snapshots `3b3606ab…`. Each route ran in a `gate_slot.py audit` slot with a
7 GiB budget and `--jobs 3`; every artifact is the same at any job count.
Corpus, system and slice use the scheduler card's route and link map.

**BASE on the original inputs fails as run 4 did** (attempt 1,
`base-original`):

- corpus: `compile database contains no selected cmetrics sources`;
- slice and system: `current audit rejected (Unknown): cross-unit home lies
  outside checkout`.

TIP on the same inputs runs every route to completion.

### Why the corpus exited 1 on every side in attempt 1

Every attempt-1 corpus side failed at the Monkey gate:

```
could not verify Cycle 18 Libevent source: Pin(SourceRevisionMismatch {
  pin: "libevent", expected: "d82464a2…", actual: "660a11bc9…" })
```

- `git archive` leaves `third_party/libevent` empty.
- The pinned route finds the submodule through the binary's compile-time
  `CARGO_MANIFEST_DIR`.
- So git answered with the enclosing card worktree's HEAD (`660a11bc9`).

The earlier cards' snapshots carried the submodule. Canonicalisation played no
part: TIP-original had already passed unit planning, as its pre-pass over all
1,444 units shows. Attempt 2 clones the pinned submodule into each snapshot,
and all four corpus sides then exit 0.

The attempt-2 corpus drivers still report 101, with audit exit 0. The audit's
own `git` refreshed `third_party/libevent/.git/index`, the stat cache of the
fresh clone, during the route. That is the only snapshot file that changed on
any side (`drivers` in each comparison).

### Results

Verdicts are identical on every side and route, row by row: every unit's
status and reason code, every row's status, and every null-property row's
custody, combined and null verdict, scope and check outcomes
(`*-verdicts-*.json`).

| Route | Units | Rows (Safe / Pending / Unknown / Unsafe) | Null rows (Safe / Unknown / Unsafe) |
|---|---|---|---|
| corpus | 185 | 1,009 / 0 / 3,816 / 9 | 2,549 / 2,355 / 17 |
| slice | 16 | 57 / 4 / 342 / 1 | 190 / 216 / 0 |
| system | 15 | 99 / 0 / 345 / 4 | 211 / 243 / 0 |

**TIP-mirror against BASE-mirror** is byte-identical on every route, except
one key per run. In all ten compared artifacts, the only difference is one
key of the member pre-pass's lifetime-source summaries, appearing in both the
evidence and the pre-pass sections. On each side it is one key with an empty
summary. The key digests a unit's preprocessed text, and the Fluent Bit
database compiles `lib/monkey/mk_bin/monkey.c`, whose `MONKEY_BUILT` is
`__DATE__ " " __TIME__`. So that key changes with the clock: two runs share it
only within the same second. The two original-spelling slice runs did fall in
the same second, and their keys agree (`*-time-key-*.json`: 1,057, 1,038 and
1,036 shared keys with equal values; at most one unshared key per run, never
shared with another run). This is pre-existing, and BASE against BASE would
show it too. It is a follow-on.

**TIP-original against BASE-namespace** compares the same inputs in the same
spelling. TIP resolves the inputs root to the real path, where BASE-namespace
resolves it to the old spelling. So the check rewrites one prefix of TIP's
output, `/media/Data/GitHub/quirin-lang/<prepared root>` to
`/home/cosmo/Documents/GitHub/qcore-c/<prepared root>`: 21 occurrences
(system), 22 (slice), 2 (corpus). The old spelling already occurs
in TIP's output where the database spelled it, so the rewrite is many-to-one
by construction and is reported as such (`target_collisions_total`).

After the rewrite, the remaining differences fall into four classes, none of
them a verdict:

1. **Digests over a canonical path, which spells the resolved root.** The
   rewrite cannot reach inside a digest.
   - `inventory_sha256` of each configuration record (16 slice, 15 system):
     the record carries the canonical `expected_source`.
   - On the corpus, the ChunkIO frame-helper import's `configuration_digest`
     and build digests: `source_contract.rs` rewrites the helper unit's file to
     its canonical path. That gives two source-effects results, 34 leaves,
     including their bounds re-keyed under the changed contract id.
2. **The member pre-pass's evidence-file set.** The pre-pass joins generator
   sources under the route's canonical root, while it lists dependencies in
   the database's spelling.
   - On TIP-original the two spellings differ. The same file is then read
     under both (the admission record's third phase: 814 tasks against 676).
   - Sorting by spelling changes which file the reason of a dropped unit cites
     first (74 reasons, all `luajit` build-tool units). The dropping decisions
     are equal.
   - Code this card did not change does this (`member_prepass.rs`
     `exclusion_files`, file reads). It is the same string-spelling coupling,
     reached through the route's root rather than through unit selection.
     Follow-on `MEMBER-PREPASS-MIXED-SPELLING`.
3. **The clock-dependent key above** (system, corpus).

On the corpus, the report (`resource-diagnostics.json`), the null and outer
projections, `monkey-source.json`, the chunk-contracts sidecar and
`audit.log` are byte-identical after the rewrite.

The corpus namespace side used here is a rerun (`runs3/`,
`rerun_namespace_corpus.sh`). The first namespace corpus mounted a tmpfs over
`/home/cosmo/Documents/GitHub` and so hid `/home/cosmo/Documents/GitHub/monkey`.
That is the repository in which the libevent provider's `contracts.lock`
resolves a tip identity (C06.2), so its `mk_vhost_set` hold read
`repository-missing` instead of `sites differ`. The status was the same (not in
force), but the evidence differed. `pin_run.sh` now keeps every sibling
visible, and the slice and system namespace sides already ran with that fix.

**TIP-original against BASE-mirror**, the brief's comparison, cannot be
byte-identical. The mirror spells the database differently from the original
inputs, and Clang is given the database's spelling (constraint 11).

- Beyond the textual rewrite (`*-base-mirror-vs-tip-original.json` lists every
  substitution and count), every digest over the spelled compile record
  differs: `build_identity`, `source_identity`, `preprocessed_source_sha256`,
  `home_environment_sha256` and the pre-pass summary keys.
- So does everything derived from header `__FILE__` literals, whose length
  depends on the spelled include path. Examples: `char[160]` against
  `char[119]` in `monkey-source.json`, and reading-site byte offsets shifted by
  30.
- So does the mirror's ordering: the mirror moves the source tree under the
  card worktree, so "first unread unit" cites a different file.

The namespace pair shows that these differences come from the spelling of the
inputs, not from TIP. On identical spelling the only residue is classes 1–3,
and the verdicts are identical everywhere.

### Assessment

Canonical unit selection makes the original inputs work. TIP on them
reproduces BASE's verdicts on all three routes, row for row. On consistently
spelled inputs (the mirror), TIP publishes the bytes BASE publishes, apart
from the pre-existing clock-dependent key. The brief's stronger "byte-identical
apart from a one-to-one path substitution" does not hold for TIP-original
against BASE-mirror. The two read different spellings, and the spelling is
bound into identities and preprocessed text by design. The report states
exactly where they differ instead of claiming it.

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
