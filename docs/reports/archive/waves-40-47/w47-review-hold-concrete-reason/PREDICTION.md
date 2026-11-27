# W47-REVIEW-HOLD-CONCRETE-REASON: frozen prediction

Base: local `main` at `ec4c9d4a` (this worktree's `HEAD` was already there).
Frozen before any implementation and before the BEFORE measurement. The
baseline is `docs/reports/w47-loop-owner-instance-flow/integration/artifacts/`
(corpus, system, slice).

## Mechanism being predicted

User decision 2026-09-29 (follow-up `W45-REVIEW-HOLD-UNACCEPTABLE-AS-WRITTEN`):
a review-held clause whose own reason says it must not be accepted as written
stays `Pending`, and the published verdict carries the concrete reason.

1. Every published function row that records at least one held-call refusal
   (``call to `NAME` whose declared contract awaits human review
   (awaiting-human-review)``) gains one new member, `review_holds`: for every
   held declaration, in name order, each of its review markers in clause
   order, with the declaration, its module, the marker's scope and clause,
   the reason text verbatim, the owner, and a category,
   `acceptable-as-written-after-review` or
   `must-be-rewritten-before-acceptance`. A row with no held call is
   byte-identical.
2. The category is structural, not read from the prose: the review marker
   gains one optional trailing word, `rewrite_before_acceptance`
   (contract specification §C06.1, grammar G05). A marker without it keeps
   the spec's existing meaning (the resolution it offers is unwrapping, that
   is accepting the clause as written after review).
3. The contract sidecar's `pending_human_review` section gains the same
   `category` per entry and a count of rewrite markers; the console tally
   line gains that count; the `.qo` checkpoint's marker record gains the
   category (checkpoint schema 3 → 4, so an older checkpoint is refused, not
   misread).
4. Five markers whose own reason says the clause as written is false are
   marked `rewrite_before_acceptance`: `flb_input`'s `resource_borrowed(data)`
   and `flb_input_new`'s `resource_borrowed(config)` and
   `resource_borrowed(data)` ("retained, not borrowed ... must not be
   unwrapped as written"), `flb_start`'s contract-level marker (`ctx` handed
   to the engine thread until it is joined: a retention `resource_borrowed`
   does not state) and `flb_downstream_worker_runtime_stop`'s contract-level
   marker (`runtime` freed on one path and not on the `-1` return, which
   `resource_released` does not state). Reason texts are unchanged. The other
   seven markers stay plain. The four changed declarations are re-pinned at
   `66910c10`, `046accf6` and the `90978ac6` export (body identities
   unchanged; only the contract text digest moves).

## Predictions

**No verdict moves anywhere (99%).** Corpus (185 units, `66910c10`), system
(`046accf6`) and tail-pause slice (`90978ac6` export): every unit status,
function status, diagnostic leaf and `diagnostic_counts` value identical to
BEFORE; function tallies stay corpus 526 Safe / 10 Unsafe / 4,298 Unknown,
system 31 / 2 / 201, slice 24 Safe / 1 provisional Unsafe / 244 Unknown /
1 Pending (`src/flb_lib.c::flb_input`, both configuration rows). No row
becomes `Safe`; the category never enters the verdict.

**BEFORE reproduces the baseline (95%):** corpus `resource-diagnostics.json`
byte-identical to the archive; system and slice identical up to run paths.

**Report rows that change, only by the added `review_holds` member (95%):**

| Scope | Rows | Function (status) | Held declaration: markers (category) |
|---|---:|---|---|
| slice | 2 | `src/flb_lib.c::flb_input` (Pending) | `flb_input_new`: `borrowed(config)` (rewrite), `borrowed(data)` (rewrite), `return_interior(config, nullable)` (acceptable) |
| slice | 2 | `src/flb_lib.c::flb_create` (Unknown) | `flb_stop`: contract (acceptable) |
| slice | 2 | `src/flb_input.c::flb_input_instance_exit` (Unknown) | `flb_input_thread_instance_exit`: contract (acceptable) |
| system | 2 | `src/flb_storage.c::flb_storage_create` (Unknown) | `flb_input_new`: 3 markers as above |
| system | 2 | `src/flb_output.c::flb_output_exit` (Unknown) | `flb_output_thread_pool_destroy`: contract (acceptable) |
| corpus | 0 | none (no corpus function holds a call) | |

**Sidecars and console (95%).** On all three scopes the
`pending_human_review` section still has 9 declarations and 12 markers, with
5 marked `must-be-rewritten-before-acceptance`; held counts unchanged (slice
3 functions / 6 call sites, system 2 / 4, corpus 0 / 0). The retained texts of
`fluent_bit/input_instance.qv` and `fluent_bit/lib_engine_thread.qv` and
their digests change, and the `contract_revision` section's contract digests
of the four re-pinned declarations change; every pin stays in force
(131 pinned, 131 in force on each route). The console tally line gains the
rewrite count; no other console line changes. `parity.jsonl` byte-identical.
