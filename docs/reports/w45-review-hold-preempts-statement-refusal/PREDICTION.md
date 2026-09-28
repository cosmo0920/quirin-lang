# W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: frozen prediction

Base: local `main` at `fefb9199` (this worktree's HEAD was already there).

**Order, disclosed.** This file was written after the gateway change and its
fixture tests (commit `3812e615`), but before any corpus, system or slice
measurement at either tree. The predictions below are about those three
measurements and about the two in-flight branches; the fixture results in
[README.md](README.md) are not predictions.

## Mechanism being predicted

A `Pending` function row is joined with every reason its function carries in
a second analysis of the same unit under the environment with every
declaration held for a human act (awaiting review, C06.1; pin mismatch, C06.2)
in force as written. A reason above `Pending` there is recorded on the row as
an `Unsupported` diagnostic starting `reason standing beside a call held for a
human act: `, so the row is `Unknown`. Rows that are not `Pending` are never
read or changed, and a unit with no `Pending` row is analysed once.

## Predictions

**Corpus (185 units, `66910c10`), system (`046accf6`), tail-pause slice
(`90978ac6`) on `main`.** No movement at all: every archived file
byte-identical to the BEFORE run (99%). Reason: the baseline archives carry
**0 Pending units and 0 Pending functions** (STATUS: no deferred contract,
every pin in force), and the second analysis runs only for a unit with a
`Pending` row; `main`'s provider tree declares no `resource_review_pending`
clause. BEFORE reproduces the baseline archives
`docs/reports/w47-condvar-join-publication/integration/artifacts/` as the
earlier cards measured (corpus `resource-diagnostics.json` byte-identical;
system and slice composing with 0 mismatches) (95%).

**W47-LOOP-OWNER-INSTANCE-FLOW (branch `worktree-agent-aa0821f6bf47bd9c0`),
per function, if merged with this fix.**

| Function | That branch | With this fix | Why |
|---|---|---|---|
| `src/flb_lib.c::flb_input` (static and shared variants), slice | Pending | **Unknown** (90%) | with `flb_input_new` in force as written, the statement's `ctx->config` actual keeps its `indirect memory access is outside the resource projection` refusal (the branch's report names it as the replaced leaf) |
| `src/flb_storage.c::flb_storage_create`, system | Unknown (reworded) | Unknown, unchanged bytes (99%) | not `Pending`, never read |
| every corpus row | unchanged | unchanged (99%) | no corpus unit calls the held callees |

**W47-CONDVAR-JOIN-STOP-PATHS (branch `worktree-agent-a1c83288552269c3a`).**
`flb_output_exit` (system), `flb_input_instance_exit` and `flb_create`
(slice) are `Unknown` there with other leaves open, so no row is `Pending`
and nothing is re-analysed: unchanged bytes (99%). Whether their held
statements also preempt a refusal changes attribution only; the verdict is
already `Unknown`.

**Fixture level** (not predicted; measured in the report): the shapes the
defect covers move `Pending → Unknown`; a row whose only reason is the hold
stays `Pending`.
