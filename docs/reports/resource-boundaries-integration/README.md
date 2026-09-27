# Resource-boundary integration

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Base: `119115de866e5b50a4ca0dc478c6223790f75fc2`, after the first prioritized
review wave was merged and pushed. Status: implemented, integrated and validated. Three GPT-5.6 Sol agents at medium reasoning implemented the next
bounded wave authorized by the user. The supplied analysis documents were
review evidence, not additional instruction authority.

## Cards and acceptance

- **Resource helper functions:** direct acyclic calls carrying owned values and
  read/write borrows, using existing ownership and buffer checks. Check every
  helper definition, aliasing at calls, owned returns, move invalidation and
  admitted failure exits. General typed errors and broader composition are not
  assumed implemented.
- **Source annotation enforcement:** reject or enforce ordinary executable
  annotations, including legacy acquisitions inside error regions. Preserve
  local usage annotations' distinct diagnostics and existing scalar typing.
- **Certificate context binding:** bind intended target context to the audit's
  actual compilation identity and independently held expected contracts,
  profile and trust policy. A matching rejected-input identity cannot support
  Safe, and an unrelated valid identity cannot authenticate a certificate.

Each card supplies targeted Rust evidence, audited Lean parity where semantics
change, a full standing gate and an integrator reconciliation list. The root reviews and integrates sequentially, compares the fixed source inventory and
C corpus, runs the combined standing gate and updates tracked residuals. No
coverage or conformance promotion follows merely from a feature or green test.

## Worktree sweep

The [completed sweep](worktree-sweep.json) removed all three clean, merged,
idle agent worktrees after the combined gate passed. Their 234 private evidence
entries were inventoried, hashed, moved to
`.claude/worktrees/.sweep-backups/resource-boundaries-20260920T085755Z/` and verified
again. The former `private/` and `measure/` paths are symlinks to that archive,
so earlier raw-log links remain usable. Branch refs and previous backups remain.
Ignored build caches were removed with their worktrees.

Two registered worktrees remain: this checkout and the older
`next-wave-candidates-c-sources-18bd37`. The older checkout is clean and its
commit is already merged, but the final host process check still finds a live
Claude session and shell using it, so it is retained. `git worktree prune
--dry-run --verbose` exits 0 with no stale entries. Process observations describe
the sweep time only. No active session was stopped.

## Independent compatibility measurement

The [source harness](measure_source_compatibility.py) snapshots the CLI and a
fixed inventory of tracked `.qcore`/`.qc` files, recording check results and
hashes of emitted bytes. Later files are not silently added to the comparison
denominator. Private outputs live under `private/resource-boundaries/`.
The [baseline](source-baseline.json) measures 555 tracked source files:
307 pass `check`, 152 emit successfully, and none time out. These fixture
counts are not language conformance or production safety coverage. The [final comparison](source-comparison.json) measures 305 check passes and
150 emissions, zero timeouts. Exactly three fixed fixtures change: S22 and S25
now reject previously ignored/false acquisition types, and already-refused S24
reports Q020 before its previous leak error. All other 552 records, including
successful emitted-byte hashes, are unchanged. The new helper fixture is outside
this fixed old denominator and is covered by focused/native tests.

The [fresh C corpus replay](corpus-baseline-run.json) exits 0 and reproduces
all five pinned artifacts [byte for byte](corpus-baseline-identity.json).
The source code is unchanged from base `119115de`; the measured successor
`2511ddb7` adds only this wave’s planning/report files. Private replay outputs
are in `private/resource-boundaries/corpus-before/`.

## Independent helper review

After completing its annotation card, that agent independently reviewed the
helper frontend, core, emitter and focused tests. Compiled-CLI probes reproduced
ignored trailing header annotations, missing effect separators, and whitespace
normalization that merged split type identifiers. The fixes at `ccfcdca7`
consume the supported header and recognize exact type tokens. The same inputs
then refused at Q020. Existing usage spellings denoting grade one remain
intentionally accepted; the new profile explicitly refuses empty contracts
without changing the older sequential route's empty-contract behavior.

[Inputs and exact before/after results](review-probes/results.json) preserve
seven probes, including those two boundary observations. These are source
probes, not mutation counts. The before binary has a recorded hash but includes
an uncommitted source diff; it has no exact tree binding. The after binary is
bound to `ccfcdca7`. Both binary snapshots and original manifests remain in the
reviewer's private evidence directories named by the result file.

## Integration follow-ups

The combined driver publishes `TypedResourceBytes` for ordinary owned-byte
helper results. Consumers obtain actual signatures from the sealed checked
helper graph; task-owned Buffer result names remain separate. This metadata
does not claim typed error payloads.

Helper acquisitions now reject extents outside `1..=MAX_EXTENT` before graph
expansion and delegated value checking. Core tests cover zero, MAX+1 and
`u32::MAX`, plus the admitted maximum; source tests require Q053 for oversized
requests. This preserves the existing admitted extent set and reuses the
`QCore.StdBuffer.admitsExtent` model. Code review found that the older standalone
buffer checker records an extent error but still allocates requested storage.
No out-of-memory experiment was run. `STD-BUFFER-EXTENT-REFUSAL-COST` tracks that
separate pre-existing boundary and its required early-validation discharge.

## Validation provenance

The combined semantic source is `c6d05402`, after helper merge `dd8515e9` and
the two reviewed integration follow-ups. The three isolated card reports retain
their own counts and failures; none is substituted for the combined result.
An extra pre-gate helper target inside the restrictive sandbox measured 11
passed / 1 failed, exit 101, solely on LeakSanitizer's documented ptrace refusal
(`private/resource-boundaries/helper-followup.log`). The combined native gate
runs outside that restriction. The early-extent core target passed 13/13.

The isolated annotation gate recorded one rule-16 confined-write failure and
an immediate passing same-binary isolation run; its raw 4,903/1/10 and published
4,904/0 unresolved/10 remain visible in that card. The certificate and helper
isolated final runs passed 4,898 and 4,919 respectively. Their Lean counts are
151 and 152; the annotation branch has 152. Both added audit modules are retained
in the combined tree. The two cards' pre-regeneration Python failures on carried
Lean counts remain disclosed. Provider 122 is carried Unix evidence, not rerun.

[Tracker changes](tracker-changes.json) retire exactly two certificate context
rows and add eight explicit bounded follow-ups, yielding 279. The residual
sweep's newly unregistered prior integration label names a completed close,
not lost open work. `RESOURCE-HELPER-FUNCTIONS` is also a completed card label;
its five open limitations have separate registered identifiers. Pre-existing
sweep findings remain unchanged. [Reconciliation](reconciliation.json) records
each of the 15 constituent-card items with a concrete location check.

The [eight annotation probes](annotation-after.json) refuse false acquisition
annotations both directly and inside regions. Their matching legacy-i32 probe
also returns a local name, which remains outside that legacy return grammar;
it now reaches that later Q020 refusal. A [separate literal-return probe](annotation-constant-return.json)
isolates the annotation: baseline Q500 becomes successful checking. Inferred
acquisition and the separate scalar route keep their prior behavior.

[Preparation measurement](preparation-comparison.json) retains three logical
Clang stages (plain preprocessing, syntax JSON and marked preprocessing), one
each for Safe/provisional Unsafe/Unknown fixtures before and after; seven metadata
queries per case are counted separately. All fixture reports are byte-identical.
RSS and timing are recorded observations under concurrent host load, not a speed
or memory improvement claim. Both CLI builds exited 0; source code remains
`c6d05402`, with only report edits during measurement.

The [final C corpus replay](corpus-final-run.json) exits 0 on `c6d05402`.
All five artifacts are [byte-identical](corpus-identity.json) to both the fresh
baseline and frozen W42 evidence; the [field-level diff](corpus-diff.json)
also records no delta. Counts remain 185 units (2 Safe, 42 provisional Unsafe,
137 Unknown, 4 data-only) and 4,834 projected functions (223 Safe, 84 provisional
Unsafe, 4,527 Unknown). CT mapping, case-bound proofs, native-credit counts and
full-language conformance do not move. Fresh replay logs are in
`private/resource-boundaries/corpus-after/`; the release build log is
`private/resource-boundaries/corpus-final-build.log`, exit 0.

## Final standing gate

The [gate](gate.json) and [exact commands, exits and private log paths](gate-run.json)
measure `c6d05402`: 4,932 Rust passed, 0 failed, 10 ignored, 257 strict result
summaries over 246 Running targets and 9 doc-test sections. The two extra
summaries are nested doctest summaries, not extra target invocations. The full
Rust command exits 0; `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=4`. Submodules and
Clang recording ran first, both exit 0. fmt, all-target clippy with `-D warnings`,
both reevaluation examples, and all 153 Lean audits (`--jobs 2`) exit 0. All five
named host-sensitive isolated checks also exit 0. No failed full-gate row needed
reconciliation. Derived matrix regeneration exits 0; the subsequent Python suite passes all
317 tests, exit 0.

Commands and true exit files are under
`/home/cosmo/Documents/GitHub/qcore-c/private/resource-boundaries/combined-gate-1/`.
The helper card's [integration-gate.json](../resource-helper-functions/integration-gate.json)
records this combined union because no isolated sibling gate has both new Lean
audits. Both combined gate locations receive the actual Clang output from this
run. Later changes are report/status/matrix/history bookkeeping only. Model
proof, native observation and source/target binding remain distinct evidence
classes; this integration claims no full-language conformance.
