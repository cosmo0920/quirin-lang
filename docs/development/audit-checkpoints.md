# Audit checkpoints (`.qo`)

[Development documentation](README.md) · [`quirin-audit` guide](../../crates/qcore-audit/README.md) · [Card report](../reports/w45-audit-checkpoints/README.md)

With `--checkpoint-dir`, `quirin-audit current` seals **one complete `.qo` per
source translation unit and compile configuration** as that unit finishes. Each
file contains all ten sections below, including the full run plan and global
publication inputs. Several compile records for the same C source produce
separate checkpoints. Workers seal files independently on up to `--jobs`
threads, even while ordered console publication waits for an earlier unit.
After all units finish, composition streams the files in plan order to produce
the whole-run report byte for byte (apart from measured timing output). A
later `--resume` reuses only unchanged units. The format is also decomposable
to YAML or TOML.

This page is the format's specification. The normative Rust definitions are in
`crates/qcore-audit/src/checkpoint.rs`; the run is in `checkpoint_run.rs` and
the composition in `compose.rs`.

## Units and steps

The route's library groups still determine source selection, headers and
ordered report rows. Checkpoint planning expands each library group into one
slot per compile configuration. Its filename is `<group-label>-<key-sha256>.qo`,
where the full SHA-256 is over that unit's source path and compile-record key.
Thus two configurations of one source have distinct names, and an identical
unit selected twice is refused. The first unit of a library retains its console
header; later units have an empty header, so composition prints it once.
The original library labels remain in the report rows.

Each unit checkpoint records the pipeline's **steps** in order, one section
each, each separately digested:

| # | Section | Contents |
|---|---|---|
| 0 | `identity` | every input of the unit result (see [Identity](#identity-and-resume)) |
| 1 | `plan` | this unit's position and every unit checkpoint of the run with its key |
| 2 | `global` | run-wide publication inputs: console preamble, revision and report key, the report's evidence record, the `.contracts.json` assumptions, the pending-review markers, the `contract_revision` section template, the historical reference revision, and (schema 5) the member fold's inputs: its environment and, when the member-facts pre-pass ran, the run's closure of each invoked member with the pre-pass's evidence section |
| 3 | `preparation` | per unit: `library`, `file`, `compile_configuration` (with the compile-task identity, raw and preprocessed digests and reason codes), `enumeration`, `origin_coordinates` |
| 4 | `lowering` | per unit: `reason`, `reason_code`, `source_functions`, `not_projected` (the projection or refusal outcome) |
| 5 | `verdicts` | per unit: `status`, `functions` (each function's summary: verdict, diagnostics, counted-user clauses), `diagnostic_counts`, and any row member no other step names |
| 6 | `certificates` | per unit: whether a published `Safe` rests on a certificate bound to the unit's compile-task identity, or the binding was refused (derived from the rows) |
| 7 | `contracts` | the contract set in force (manifest and source digests, per-module digests, provider selection, mode, lock digests, pending-review counts) and what the run-final sections read of this unit: held calls per callee (`awaiting-human-review` and `contract-pin-mismatch`), the analysed function names and (schema 5) the member and state facts the unit recorded for the member fold |
| 8 | `result` | the library header line (empty after its first unit), each unit's console lines, its tallies (without timings, with the `Pending` count), its per-category diagnostic counts, the checkout sources its units stand for, and the unit's **verdict** |
| 9 | `provenance` | wall time, sampled peak and process high-water RSS, the job count, the three preparation timings: measurements, outside identity and outside every composed verdict |

Sections 3–6 hold exactly one entry. Row values are recursively encoded as
typed binary values. Integers, Boolean values, finite floating-point bit
patterns and common verdict/status codes are numeric payloads, not JSON text.
Strings carry names, paths, digests and messages. On composition, the values
are reconstructed as JSON and rendered with serde_json's canonical two-space
printer; the three row steps are merged in key order. This reproduces the
whole-run report bytes without storing pretty JSON in the `.qo`.

## Binary container (format version 1)

| Offset | Length | Contents |
|---|---|---|
| 0 | 2 | magic `QO` |
| 2 | 2 | format version, big-endian `u16` (`00 01`) |
| 4 | 4 | guard `0D 0A 1A 0A` (CR LF SUB LF), which a text-mode transfer corrupts |
| 8 | 4 | header length *H*, little-endian `u32` (at most 1 MiB) |
| 12 | *H* | the header, postcard-encoded: `{ schema_version: u32, sections: [ { name: string, length: u64, sha256: [u8; 32] } ] }` |
| 12+*H* | 32 | SHA-256 of bytes `0 .. 12+H` |
| 44+*H* | Σ lengths | the sections, concatenated in manifest order; the file ends exactly after the last |

Every section is its schema type encoded with [postcard](https://postcard.jamesmunns.com/wire-format)
1.x and must decode with no bytes left over. Schema **5** has exactly the ten
sections above. Postcard encodes unsigned integers and enum discriminants as
base-128 varints, signed integers as zigzag varints, Booleans as one byte,
strings as a varint byte length followed by UTF-8, and arrays/maps as a
varint element count followed by elements. Struct fields follow their Rust
declaration order. Options use a 0/1 tag. The `F64Bits` variant encodes the
IEEE-754 binary64 bit pattern as a `u64` varint, retaining negative zero and
adjacent finite values exactly. Non-finite bit patterns and duplicate object
keys are refused on read.

The recursive `BinaryValue` tags are stable since schema 3 (schema 4, card
W47-REVIEW-HOLD-CONCRETE-REASON, adds only each pending-review marker's
`category` to the `global` section; schema 5, card MEMBER-PREPASS-OTHER-ROUTES,
adds only the run's member facts: `member_facts_sha256` to the run identity,
`member_fold` to `global` and `callback_members` to `contracts`):

| Code | Payload |
|---:|---|
| 0 | `Null`, none |
| 1 | `Bool`, one byte |
| 2 | `I64`, zigzag varint |
| 3 | `U64`, varint |
| 4 | `F64Bits`, `u64` varint of raw IEEE-754 bits |
| 5 | `Status`, nested code: 0 Safe, 1 Pending, 2 Unknown, 3 Unsafe, 4 data_only, 5 gateway_failure |
| 6 | `String`, UTF-8 byte length and bytes |
| 7 | `Array`, count then recursive values |
| 8 | `Object`, count then ordered `(key, value)` pairs |

`ResultSection.verdict` uses codes 0 Safe, 1 Pending, 2 Unknown, 3 Unsafe.
`CertificateRecord.outcome` uses 0 PublishedSafe, 1 BindingRefused,
2 NotPublished. Unknown codes are refused. Schema 2 files, which held pretty
JSON strings and group-level checkpoints, and schema 3 files, whose markers
carry no category, and schema 4 files, which carry no member facts, are
deliberately refused by this reader; `--resume` recomputes their units as
schema 5 files and leaves the old files untouched. Schema 1 was already
refused.

The **content digest** of a checkpoint is the SHA-256 of the postcard encoding
of `(schema_version, manifest entries except provenance)`. Every section but
`provenance` is a deterministic function of the run's inputs, so two runs over
the same inputs — at any `--jobs` — write checkpoints with equal content
digests; `quirin-audit checkpoint verify` prints it.

**Refusal.** A reader refuses the whole file, and uses no section of it, when
the file is shorter than its prelude or its declared length (`truncated`), has
bytes after the last section (`trailing`), has the wrong magic, another format
or schema version, a header or any section that does not match its SHA-256,
an unexpected section list, or a section that does not decode exactly. The
composer verifies every file completely before it writes anything, writes the
report to a temporary sibling and renames it into place only when complete.
A resumed run treats a refused checkpoint as absent and recomputes that unit.

## Identity and resume

The `identity` section is the reuse key. Its **run** part is shared by every
unit of one run:

- the analyzer: package, version, build profile, the source tree's `HEAD` when
  the binary was built (`build.rs`; absent in a packaged build), and the
  SHA-256 of the running executable;
- the evidence mode, the canonical checkout path and its `HEAD`, the source
  profile digest;
- the contract set: the manifest and elaborated-source digests, the provider
  selection and evidence records' digests, every `contracts.lock` the pin
  evaluation read with its SHA-256, and the digest of the pin evaluation's
  section template (which contracts are pinned, in force or withheld over
  which reviewed sites; card W45-CONTRACT-REVISION-MODE), and the digest of
  the checker-plugin section when a checker ran (card
  W45-USER-CONTRACT-PLUGINS: a plugin's verdict enters every function that
  applies its contract), and the digest of the refutation table the unit was
  analysed under (card W45-REFUTED-CONTRACT-WITHDRAWAL, contract spec C06.4:
  the empty table's digest in a first phase);
- the Clang identity (program, version, target);
- every `QCORE_*` environment variable except `QCORE_AUDIT_JOBS`;
- (schema 5, card MEMBER-PREPASS-OTHER-ROUTES) the SHA-256 of the run's member
  facts when the member-facts pre-pass ran (gateway rule 12n): every unit
  decided its member invocations over them, so a change anywhere in the
  compile database that changes them recomputes every unit, and checkpoints
  of two run sets never compose (`member facts differs`).

Its **group** part is the unique unit label, inherited library refusal, and a
**fingerprint**: the row's `file`, the SHA-256 of the unit's compile
configuration (which carries the compile-task identity: command, flags,
compiler, raw and preprocessed-source digests, dependencies), and the
preparation facts the configuration does not carry (the syntax-validation
error, the line-marked preprocessing digest or error).

`--resume` recomputes the identity now, which needs each unit's Clang
preparation but none of its analysis, and reuses a unit only when the whole
section is equal. Otherwise it recomputes that unit, and it reports why on standard error
(`checkpoint <label>: reused (identity equal)`, or `recomputed (<reason>)`:
`no checkpoint`, `checkpoint refused: …`, `analyzer build differs`,
`contract set differs`, `source revision differs`, `unit N (<file>): compile
configuration differs`, …) and in `<dir>/checkpoint-run.json`. A reused checkpoint is left untouched unless the run's plan, run-wide
publication inputs or its library-header position changed; then its analysis
step sections are re-sealed unchanged under the new `plan`, `global` and
`result.header`. A unit that becomes the first selected unit gains the library
header, and one moved later loses it. A census run (any census stream active) refuses
checkpoints: census streams are process-global and ordered by analysis.

## Composition

**Two phases (card W45-REFUTED-CONTRACT-WITHDRAWAL).** A declared contract
its own analysed body refutes is withdrawn from callers in every unit, so unit
results are independent only under one refutation table. A checkpointed run
first analyses every unit under the empty table and composes; when the
composed rows record a refutation, it analyses every unit again under that
table, whose digest is in every unit's identity (so no first-phase checkpoint
serves the second), and composes again. Both phases seal to the same
`<dir>/<group-label>-<key-sha256>.qo` paths, so the second overwrites the first's checkpoints: a
resumed run whose table is not empty finds, in each phase, the other phase's
digest and recomputes every unit, reusing none
(`CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES`). Only a run whose table is empty,
which has no second phase, reuses on `--resume`. Checkpoints sealed under different
tables disagree on the contract set and never compose. The run records the
applied table in its assumptions before any unit, and the composer renders the
`refuted_contracts` section from it and from the rows.

`quirin-audit checkpoint compose FILE.qo... [--report PATH]
[--monkey-source-report PATH] [--binding-directory DIR]` refuses unless the
files are exactly one run's plan: every checkpoint agrees on the run identity,
the `global` section and the plan; each planned unit appears once; none is missing; no
unit is selected twice. It then merges the small results (tallies and
per-category counts are summed, sources and held-call rows united, analysed
function names united; the run's verdict is the join of the units' verdicts,
and a checkpoint whose recorded verdict differs from the one its tallies imply is
refused), writes the report's head, and streams the rows one
unit at a time. It writes the report, its `.contracts.json` sidecar (with the
`pending_human_review`, `contract_revision` and `refuted_contracts` sections rendered from the
merged facts, `source_file_count` and `configuration_count`), and the explicit
Unknown sidecars, and prints the whole run's console transcript (with the
checker-plugin tally when a checker ran). It reports the run's verdict on
standard error and in `checkpoint-run.json`.

**The member fold (gateway rule 12n; card MEMBER-PREPASS-OTHER-ROUTES).**
When the contracts invoke a callback member or state a constructor's result
state, the whole run folds every unit's member and state facts before
publishing (`callback_member_fold`). The composer folds the same inputs from
the checkpoints: the environment and the run's closures in `global`, each
unit's facts in its `contracts` section and the rows' `library`, `file`,
`status` and `functions`. It writes the fold's `callback_members` section
into the sidecar (with the pre-pass's evidence as `prepass`) and prints the
`callback members:` line where the whole run prints it. It does not rewrite
rows, so a fold that would demote a row is refused (with the run's facts
installed every unit decided over the carried closures and the member part
demotes nothing, `QCore.MemberPrepassRoutes`). A run whose contracts invoke a
member and whose checkpoints carry no run closures is refused, as before.

**Unit verdicts.** A row's status maps to the four-value verdict of
§L21.2: `Safe`, `Pending`, `Unknown` and `Unsafe` are themselves; a
`gateway_failure`, or any status this build does not know, is `Unknown`; a
`data_only` unit, which defines no function and so carries no obligation, is
`Safe`, the join's identity. The
`preparation:` console line sums the recorded timings, which are measurements.

## Text decomposition

`quirin-audit checkpoint decompose FILE.qo --format yaml|toml [--output F]`
writes one document: `format`, `format_version`, `schema_version`,
`content_sha256`, the `manifest` (each digest as hex) and every section's typed
contents. `quirin-audit checkpoint recompose TEXT --format yaml|toml --output
FILE.qo` parses it back, re-encodes every section, and refuses unless the result
matches the recorded manifest and content digest, so an edited text never
becomes a checkpoint; an unedited one recomposes to the original bytes.

Mapping rules, chosen so both forms are lossless:

- The text form exposes typed scalar values and externally tagged enums.
  Optional struct fields absent in TOML appear as `null` in YAML. Recursive
  binary `Null` values use the explicit enum tag, preserving array positions.
- TOML integers are signed 64-bit; a value above `i64::MAX` cannot be written
  and `decompose` refuses rather than truncating. No field reaches that range in
  practice (lengths, counts, microseconds).
- Row values and global publication inputs are decoded from numeric binary
  tags. Finite floating-point bit patterns reconstruct the same serde_json
  numbers; decompose/recompose preserves the binary tags and bit patterns.
- TOML emits a table's plain keys before its sub-tables and arrays of tables;
  field order is not significant on reading.

## Memory and interrupted runs

A checkpoint worker retains one unit's analysis and the ten sections for that
unit until it atomically renames the file. Up to `--jobs` units may run at once;
the ordered consumer retains only small completion records. A later unit's
completed `.qo` is sealed before the consumer waits for an earlier, slower
unit, with an immediate `checkpoint ready: PATH` line on standard error.
After all units finish, `checkpoint linking N file(s)` marks composition.
Process interruption leaves every already sealed unit available to `--resume`. An interrupted unit's `.qo.partial` is ignored and its missing or
refused `.qo` is recomputed. Files are synced before rename; Unix also syncs
the containing directory. Process-interruption recovery is portable, while
host-crash durability depends on filesystem and platform guarantees. The composer reads one unit's step sections at a
time. Each self-contained `.qo` repeats the full plan and global inputs; this
uses more disk than one file per library, but permits independent durability
and standalone verification of each compile configuration.

## The pinned corpus route

Card CORPUS-GROUP-CHECKPOINTS ([report](../reports/corpus-group-checkpoints/README.md)).
`quirin-audit pinned --checkpoint-dir DIR [--resume]` checkpoints the corpus
arbiter. The normative Rust is `crates/qcore-audit/src/pinned_route/checkpoints.rs`;
the Lean model is `QCore.StagedCheckpoints`.

**Groups and stages.** A group is one of the route's libraries (`audit.json`'s
libraries, then `monkey-libevent`): ChunkIO's prelude and postlude share typed
certificates across a library's units, so a library is the smallest result that
stands alone. The route runs up to three **stages**, each a phase of the whole
run: `first` (the loaded contracts), `refuted` (when a body refutes its own
declaration: the contracts with the refutation table, C06.4) and `imported`
(when the link map selects cross-unit homes: the imported environment and
homes, accepted only when its rows leave the table unchanged). Each stage seals
its own file, `DIR/<library>.<stage>.qo`, so the stages never overwrite each
other. A group is sealed by the worker that completes it — its postlude runs
then — whatever the other groups are doing, so an interrupted run keeps every
completed group. Groups of one stage are analysed concurrently, as in the whole
run.

**Container.** The binary container above, with its own layout: schema version
**1** and the five sections `pinned-identity`, `pinned-prelude`, `pinned-units`,
`pinned-finish`, `provenance`. The current-route reader refuses these files
(schema and section list) and the pinned reader refuses current-route files.
`pinned-units` holds each unit's console lines, report rows, diagnostic
counts, bounds and null rows and evidence, member facts, tally, native and
teardown results, as typed binary values; `pinned-finish` holds the postlude's
console lines and the teardown results it appended; `provenance` (wall time,
jobs, peak RSS) is outside the content digest. `quirin-audit checkpoint`
reads current-route files only.

**Identity** (`pinned-identity`). A group is reused only when the whole section
equals the one computed now; otherwise it is recomputed and the reason is
printed (`checkpoint <library>.<stage>: recomputed (<field> differs)`) and
recorded in `DIR/pinned-checkpoint-run.json`.

| Part | Fields | Read by |
|---|---|---|
| run | analyzer (package, version, profile, build commit, binary SHA-256); canonical Fluent Bit root and its verified pinned revision; the process working directory (the route binds it into each row's `hypothesis_context.build_identity` and the source-effect configuration digests); SHA-256 over every file of the source-set profile directory; the source-contract configuration's SHA-256 and, in package mode, every file of the package; the contract manifest's path and SHA-256, every lock the pin evaluation read and the pin template's digest; Clang (program, version, target); every `QCORE_*` variable but the job count, with `QCORE_NULL_PROPERTY`, `QCORE_OUTER_PROJECTION` and `QCORE_MEMBER_PREPASS_SECTION` recorded as `selected` (their value is where the run writes) and `QCORE_CROSS_UNIT_LINK_MAP` as its content's SHA-256; the run's member facts' SHA-256 | every group |
| stage | the stage name; SHA-256 of the exact contract environment every unit of the stage is analysed under (pin holds, result ranges, the refutation table, imports and link resolution included); SHA-256 of the homes supplied to every unit; SHA-256 of the installed bounds and null environments (with the bounds refutation table) | every group of that stage |
| group | label, source root, and per unit: SHA-256 of `[file, directory, command]`, the raw source's SHA-256, the validated preprocessing's SHA-256, and the preparation facts (validation error, line-marked preprocessing digest or error) | that group |

Whole-run facts therefore invalidate every group of the stage that reads them
and no other stage: a changed refutation table recomputes every group's
`refuted` stage and leaves its `first` stage reused. Refused before any work,
because a reused group would not reproduce what they write or the identity does
not digest what they read: a census stream, `QCORE_TYPED_PATH_PARITY`,
`QCORE_AUDIT_IR_DIR`, `QCORE_UNIT_TIMING`, the bounds selection
(`QCORE_BOUNDS_PROPERTY`, follow-up `CORPUS-CHECKPOINT-BOUNDS-SELECTION`) and a
process source-contract configuration (`QCORE_SOURCE_CONTRACTS`; the
`QCORE_CHUNKIO_PACKAGE` package is digested). A unit that cannot be
fingerprinted recomputes its group, which is not sealed; a group with a failed
step or postlude is never sealed.

**Composition.** The whole-run fold is shared: the final stage's groups are
expanded into the steps the fold consumes (prelude, units, postlude, at their
plan positions, with the Monkey gate between the Fluent Bit libraries and
Monkey), and each stored unit is restored to its report fragment one at a time.
The phases' rules are the whole run's: a stopped stage (a failed prelude or
unit) skips the later stages and the fold stops at the failure; an `imported`
stage that fails or changes the table is refused and the previous stage's
groups are published. Whole-run steps that read no group result — the library
summary profiles, the pin and bounds evaluations, the member-facts pre-pass,
the definition census and home exports, the Monkey gate and every sidecar —
run every time.
