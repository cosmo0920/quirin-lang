# Audit checkpoints (`.qo`)

[Development documentation](README.md) · [`quirin-audit` guide](../../crates/qcore-audit/README.md) · [Card report](../reports/w45-audit-checkpoints/README.md)

A checkpointed `quirin-audit current` run divides the audit by **library
group**. It analyses one group at a time, seals that group's intermediate
results into a binary checkpoint `<label>.qo` ("Quirin-audit Object"), drops
the group's state, and composes the report from the checkpoints by streaming
them. Peak memory is bounded by one group rather than by the corpus. A later run
can **resume**, reusing each group whose identity is unchanged, and any set of
one run's checkpoints can be **composed** into the report the whole run writes,
byte for byte. A checkpoint also **decomposes** into YAML or TOML for reading.

This page is the format's specification. The normative Rust definitions are in
`crates/qcore-audit/src/checkpoint.rs`; the run is in `checkpoint_run.rs` and
the composition in `compose.rs`.

## Groups and steps

The **groups** are the route's own groups, in order: the source profile's
`audit.json` libraries (each under its `label`, or its library name), or the
groups of an explicit `--selection` file (card W45-FB-SYSTEM-SELECTION), then
`monkey-libevent` when a Monkey database is given. Configure the grouping by
editing `audit.json` or by passing a selection. A label names its checkpoint
file, so it must be a plain file name (`[A-Za-z0-9._-]`, not starting with `.`),
and labels must be distinct. No unit may belong to two groups: the checkpointed
run and the composer both refuse an overlapping plan.

Each group's checkpoint records the pipeline's **steps** in order, one section
each, each separately digested:

| # | Section | Contents |
|---|---|---|
| 0 | `identity` | every input of the group's result (see [Identity](#identity-and-resume)) |
| 1 | `plan` | this group's position and every group of the run with its units' keys |
| 2 | `global` | run-wide publication inputs: console preamble, revision and report key, the report's evidence record, the `.contracts.json` assumptions, the pending-review markers, the `contract_revision` section template, the historical reference revision |
| 3 | `preparation` | per unit: `library`, `file`, `compile_configuration` (with the compile-task identity, raw and preprocessed digests and reason codes), `enumeration`, `origin_coordinates` |
| 4 | `lowering` | per unit: `reason`, `reason_code`, `source_functions`, `not_projected` (the projection or refusal outcome) |
| 5 | `verdicts` | per unit: `status`, `functions` (each function's summary: verdict, diagnostics, counted-user clauses), `diagnostic_counts`, and any row member no other step names |
| 6 | `certificates` | per unit: whether a published `Safe` rests on a certificate bound to the unit's compile-task identity, or the binding was refused (derived from the rows) |
| 7 | `contracts` | the contract set in force (manifest and source digests, per-module digests, provider selection, mode, lock digests, pending-review counts) and what the run-final sections read of this group: held calls per callee (`awaiting-human-review` and `contract-pin-mismatch`) and the analysed function names |
| 8 | `result` | the group's header line, each unit's console lines, its tallies (without timings), its per-category diagnostic counts, the checkout sources its units stand for |
| 9 | `provenance` | wall time, sampled peak and process high-water RSS, the job count, the three preparation timings: measurements, outside identity and outside every composed verdict |

Sections 3–5 hold one entry per unit, in unit order; each entry is the row's
members that step owns, each as its value's **pretty JSON text at depth zero**
(serde_json's two-space printer). A row is rendered back by merging the three
entries' members in key order and nesting each value at its depth, which is
exactly what serde_json prints for the row, so no value is ever re-parsed.

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
1.x (varint integers, length-prefixed strings and sequences, a tag byte per
option, a varint discriminant per enum variant) and must decode with no bytes
left over. Schema version 1 has exactly the ten sections above, in that order.

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
A resumed run treats a refused checkpoint as absent and recomputes the group.

## Identity and resume

The `identity` section is the reuse key. Its **run** part is shared by every
group of one run:

- the analyzer: package, version, build profile, the source tree's `HEAD` when
  the binary was built (`build.rs`; absent in a packaged build), and the
  SHA-256 of the running executable;
- the evidence mode, the canonical checkout path and its `HEAD`, the source
  profile digest;
- the contract set: the manifest and elaborated-source digests, the provider
  selection and evidence records' digests, every `contracts.lock` the pin
  evaluation read with its SHA-256, and the digest of the pin evaluation's
  section template (which contracts are pinned, in force or withheld over
  which reviewed sites; card W45-CONTRACT-REVISION-MODE);
- the Clang identity (program, version, target);
- every `QCORE_*` environment variable except `QCORE_AUDIT_JOBS`.

Its **group** part is the label, the group-wide refusal, and per unit a
**fingerprint**: the row's `file`, the SHA-256 of the unit's compile
configuration (which carries the compile-task identity: command, flags,
compiler, raw and preprocessed-source digests, dependencies), and the
preparation facts the configuration does not carry (the syntax-validation
error, the line-marked preprocessing digest or error).

`--resume` recomputes the identity now, which needs each unit's Clang
preparation but none of its analysis, and reuses a group only when the whole
section is equal. Otherwise it recomputes, and it reports why on standard error
(`checkpoint <label>: reused (identity equal)`, or `recomputed (<reason>)`:
`no checkpoint`, `checkpoint refused: …`, `analyzer build differs`,
`contract set differs`, `source revision differs`, `unit N (<file>): compile
configuration differs`, …) and in `<dir>/checkpoint-run.json`. A reused
checkpoint is left untouched unless the run's plan or run-wide publication
inputs changed; then its step sections are re-sealed unchanged under the new
`plan` and `global`. A census run (any census stream active) refuses
checkpoints: census streams are process-global and ordered by analysis.

## Composition

`quirin-audit checkpoint compose FILE.qo... [--report PATH]
[--monkey-source-report PATH] [--binding-directory DIR]` refuses unless the
files are exactly one run's plan: every checkpoint agrees on the run identity,
the `global` section and the plan; each group appears once; none is missing; no
unit is in two groups. It then merges the small results (tallies and
per-category counts are summed, sources and held-call rows united, analysed
function names united), writes the report's head, and streams the rows one
group at a time. It writes the report, its `.contracts.json` sidecar (with the
`pending_human_review` and `contract_revision` sections rendered from the
merged facts, `source_file_count` and `configuration_count`), and the explicit
Unknown sidecars, and prints the whole run's console transcript. The
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

- The schemas use only strings, unsigned integers, options, sequences, structs
  and one enum. `Option` never occurs inside a sequence, because TOML has no
  null: a `None` field is omitted in TOML (`null` in YAML) and read back as
  `None`.
- The enum (`preparation` of a unit fingerprint) is externally tagged:
  `not_prepared` is a string, `prepared` a table of its fields.
- TOML integers are signed 64-bit; a value above `i64::MAX` cannot be written
  and `decompose` refuses rather than truncating. No field reaches that range in
  practice (lengths, counts, microseconds).
- Row members and other JSON values are strings holding their exact JSON text;
  no number is re-parsed, so no float can drift.
- TOML emits a table's plain keys before its sub-tables and arrays of tables;
  field order is not significant on reading.

## Memory

The whole run holds every unit's rows until it writes the report. A
checkpointed run holds one group: its units' analyses (up to `--jobs` at once),
then the group's rows as JSON text until the group is sealed. The composer holds
one group's step sections. Within a group, `--jobs` multiplies the working set,
because each job holds one unit's analysis and Clang output; lower `--jobs` to
lower the peak. Measurements are in the [report](../reports/w45-audit-checkpoints/README.md).
