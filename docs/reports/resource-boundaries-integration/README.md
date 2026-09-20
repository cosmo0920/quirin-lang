# Resource-boundary integration

Base: `119115de866e5b50a4ca0dc478c6223790f75fc2`, after the first prioritized
review wave was merged and pushed. Status: implementation and validation in
progress. Three GPT-5.6 Sol agents at medium reasoning implement the next
bounded wave authorized by the user.

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
change, a full standing gate and an integrator reconciliation list. The root
will review and integrate sequentially, compare the fixed source inventory and
C corpus, run the combined standing gate and update tracked residuals. No
coverage or conformance promotion follows merely from a feature or green test.

## Worktree sweep

The requested sweep found five registered worktrees: this integration checkout,
three isolated worktrees reused by the current agents, and the older
`next-wave-candidates-c-sources-18bd37`. The older worktree is clean and its
commit is an ancestor of main, but a live Claude session and shell use it.
It is retained to avoid disrupting that session. The three active agent
worktrees are protected. No stale registrations were found by the dry-run prune,
and no eligible worktree was removed. Prior `.sweep-backups` and private logs
are preserved. Process observations describe the sweep time only.

## Independent compatibility measurement

The [source harness](measure_source_compatibility.py) snapshots the CLI and a
fixed inventory of tracked `.qcore`/`.qc` files, recording check results and
hashes of emitted bytes. Later files are not silently added to the comparison
denominator. Private outputs live under `private/resource-boundaries/`.
The [baseline](source-baseline.json) measures 555 tracked source files:
307 pass `check`, 152 emit successfully, and none time out. These fixture
counts are not language conformance or production safety coverage. Final
comparison and per-change dispositions remain pending.

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
