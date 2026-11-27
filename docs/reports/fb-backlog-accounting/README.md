# FB-BACKLOG-ACCOUNTING: ledger census, checked saturating transitions (rule 92)

Card FB-BACKLOG-ACCOUNTING (Fluent Bit plan P1.2), 2026-11-27. Base `76c1f9cfc`
(main, r77). Probe `9d2ae0076`, prediction `a971680e9`, semantic commit
`8ab7477c3` (the measured code; later commits change tests and records only).

## Headline

* **Census** ([census.json](census.json)): every site that changes or tests a
  backlog/input byte or count ledger at Fluent Bit `046accf6` (`sb.c`,
  `flb_input_chunk.c`, ChunkIO), classified as saturating transition, strict
  bound, batch overshoot, advisory limit or snapshot, with the inequality read
  from the C. Only ChunkIO's unit counters are stated by a contract
  (`counters.qv`, declared premises). No contract states either per-output
  byte ledger.
* **Gateway rule 92** (`crates/qcore-c-gateway/src/ledger_transition.rs`): a
  typed-source consumer with its own node admission. It certifies each write
  to a named unsigned ledger field as a guarded saturating subtraction or
  addition, or one of their two clamps. Optionally, each path must pair them
  one-to-one with the list link/unlink calls. A bare wrapping update, any
  other guard or clamp shape, a store or call between guard and update, a
  missing or doubled update, and any scan loop are refused (`Unknown`).
* **Lean**: `QCore.BacklogLedger`, 33 audited theorems.
* **No verdict moves.** The probe shows that the custody gateway reads every
  ledger statement, wrapping or not, as custody-neutral. Rule 92 is not
  consulted by any custody route. Accounting facts are therefore orthogonal
  to custody verdicts, and the opt-in measurement confirms it (§4).
* **The pinned bodies themselves stay `Unknown` for the ledger property.**
  Route T refuses the `flb_warn` argument lists before rule 92 reads them,
  and the discharge sits inside an `mk_list_foreach_safe` loop that T does
  not read. The certified fixtures reproduce the ledger statements verbatim
  but change the log's name argument and cut the loop to its matching
  iteration (§2). Both gaps are registered follow-ups.

## 1. Census and the admission inequalities

Four ledgers, each kept distinct (physical resources, per-output delivery
charges, budget credits):

| Ledger | Kind | Transitions | Limit test | Class |
|---|---|---|---|---|
| `fs_backlog_chunks_size` | per-output delivery charge (backlog queue) | `sb.c:277-287` charge, `:226-238` discharge, both saturating; the wrapper stores the charged size and removal subtracts it (exact identity) | read by `flb_input_chunk_output_would_exceed_limit` | strict bound |
| `fs_chunks_size` | per-output delivery charge (routed input chunks) | `flb_input_chunk.c:66-92` saturating helpers, called with recomputed real sizes (`:620`, `:838`, `:2661`, `:2747`, `:4011-4014`) | `flb_input_chunk.c:1176-1199` | strict bound |
| backlog `mem_limit` | budget credit | `cb_queue_chunks` `total += size` (`sb.c:716`) | `sb.c:605`, `:611` | batch overshoot |
| `mem_chunks_size`, `total_chunks_up` | physical resident bytes, mappings | snapshot (`cio_stream_size_chunks_up`), unit counters (`counters.qv`) | `:2961` (after the write), `cio_file.c:566` (enforced up only; `cio_chunk_up_force` skips it) | advisory |

The exact inequalities read from the C:

* **Strict bound.** `would_exceed(fs, bl, c)` is
  `!(fs <= T && bl <= T - fs) || c > T - fs - bl`. Every subtraction there is
  guarded, so it equals `fs + bl + c > T` over the naturals for every
  `size_t` input (`wouldExceed_iff`). Admission is `fs + bl + c <= T`.
  `total_limit_size == -1` (the `SIZE_MAX` sentinel) disables the test.
* **Batch overshoot.** `cb_queue_chunks` tests `total < mem_limit` once per
  round. It then queues one chunk per non-empty queue, so a round ends below
  `mem_limit + queues * largest chunk` (`batch_overshoot_bound`). The bound
  is reachable (`batch_overshoot_reachable`). Eviction (`sb.c:549-575`,
  `flb_input_chunk.c:556-738`) overshoots on the credit side by less than the
  last chunk it evicts.
* **Advisory.** `mem_buf_limit` pauses only after `mem_chunks_size >=
  mem_buf_limit`, once the write has happened. `max_chunks_up` is strict in
  `_cio_file_up(enforced)` but is bypassed by `cio_chunk_up_force`, which the
  backlog uses (`sb.c:412`, `:633`). Nothing in these tests bounds the
  overshoot. Choosing which inequality to certify for resident bytes is a
  human decision (follow-up FB-LEDGER-ADVISORY-LIMITS). This card certifies
  only what the C states and adopts no bound of its own.

Two signedness sites are recorded as premises, not defects:

* `sb.c:317-324` stores `cio_chunk_get_real_size`'s `ssize_t` in a `size_t`.
  Its `< 0` test is therefore always false, and the `-1` arm (an unknown
  store type, `cio_chunk.c:360`) would charge `SIZE_MAX` (P-LEDGER-STORE-TYPE).
* `sb_release_output_queue_space` compares `size_t` with `ssize_t`. It is
  correct only for a positive requirement at entry, which its caller
  establishes (`flb_input_chunk.c:1127`).

The signed-delta magnitude `(size_t)(-(chunk_size + 1)) + 1`
(`flb_input_chunk.c:3986-3991`) is exact and avoids negating `SSIZE_MIN`
(`negative_magnitude`, `negative_magnitude_in_range`).

## 2. Rule 92

[Rule text](../../agents/gateway.md#constraint-92). The checker reads route
T's paths. It certifies a transition only where the nearest guard that
reads the ledger has the exact shape and the right outcome. For an update,
it also requires that the guard's operand equal the update's operand once
reads are replaced by their places. Between the earliest of those reads and
the write there may be no store, call or local store. The four transitions
and their decision table are `decide` in Rust and `decide'` in Lean, matched
row for row (`decision_vectors`, `decide_vectors`).

**Fixtures** (`crates/qcore-c-gateway/tests/unit/ledger_transition.rs`, compiled into the crate's own unit tests: a new test binary would push `test_gate_scope_census`'s read-census coverage below its 90% floor, 703 of 782):

| Shape | Source | Result |
|---|---|---|
| `flb_input_chunk_output_size_subtract`, `input_chunk_output_size_add` | `flb_input_chunk.c:66-92`, `flb_warn` expanded | underflow clamp + discharge; overflow clamp + charge |
| `sb_append_chunk_to_segregated_backlog` | `sb.c:262-290` | allocation failure: no link, no charge; success: one link, one saturating charge |
| matching iteration of `sb_remove_chunk_from_segregated_backlog` | `sb.c:224-244`, loop cut to the matching element | one unlink, one saturating discharge; no match: neither |
| rollback after the charge that unlinks and discharges | mutation control | admitted |
| bare `-=` / `+=`, guard on another operand, swapped arms, clamp to 1, store or call between guard and update | mutation controls | refused: not an admitted transition |
| doubled discharge, missing discharge, rollback without discharge, charge before the allocation test | mutation controls | refused: not paired |
| `>=` guard | mutation control | refused by the node admission |
| the pinned `flb_warn` argument lists | `flb_input_chunk.c:70-72`, `sb.c:229-233`, `:278-282` | T refuses (`unsequenced call/read operands`) before rule 92 |

The two deviations from the pinned text are stated in the fixtures. First,
`flb_output_name(...)` in the log's argument list is replaced by a literal.
T has no ordering model for an opaque call beside unsequenced reads, so the
faithful text is refused, and that refusal is itself pinned. Second, the
discharge is read as its loop body's matching iteration, because T reads no
`mk_list_foreach_safe`. `mk_list_add`/`mk_list_del` are declared functions
rather than the header's inline bodies.

**What Lean adds** beyond the decision table:

* The guarded forms do not wrap in `size_t` arithmetic, and the bare forms
  do.
* The invariant `ledger = sum of linked charges` is preserved by
  charge-on-link (under the physical bound P-LEDGER-PHYSICAL),
  discharge-on-unlink, residency up/down, partial fan-out over shared chunks
  (the first `k` queues charged, `return -3`) and repeated removal (a no-op
  under one wrapper per chunk per queue).
* A rollback that unlinks and discharges restores the state exactly.
* Under the invariant, both clamp arms are unreachable.
* A missing discharge breaks the invariant. A doubled discharge breaks it
  unless the second update's underflow clamp hides it
  (`doubled_discharge_breaks_or_clamps`). Saturation can mask a double
  decrement, which is why rule 92 refuses it statically.

Partial fan-out keeps every ledger equal to its linked wrappers. The
known A2 observation (FB-AUDIT-03-PARTIAL-FANOUT: `sb_segregate_chunks`
closes the chunk under a still-charged wrapper) is a custody fault, not an
accounting one.

**Premises** (stated, not proved): P-LEDGER-EXCLUSIVE (no concurrent writer
while the function runs), P-LEDGER-CALLEES (callees do not write the
ledger), P-LEDGER-ABI (the supplied maximum is the target's), and
P-LEDGER-PHYSICAL. P-LEDGER-RECOMPUTED-CHARGE also stands: `fs_chunks_size`
discharges recompute the chunk's real size rather than subtracting what was
charged, so its conservation is not established.

## 3. Hook points

`crates/qcore-c-gateway/src/lib.rs`: one `pub mod ledger_transition;` line,
beside `ffi_layout`. No other shared file changes, and no provider contract,
lock, schema or route changes. The generated files are
`docs/agents/gateway.md`, `docs/agents/README.md` and `proof/QCore.lean`.

## 4. Measurement

PREDICTION: no row moves on any route. **Measured: as predicted.**

| Run | Exit | Units Safe/Unsafe/Unknown/data-only | Functions Safe/Pending/Unknown/Unsafe |
|---|---|---|---|
| base `76c1f9cfc` | 0 | 9 / 6 / 162 / 4 (181) | 1,205 / 2 / 3,590 / 7 (4,804) |
| tip `8ab7477c3` | 0 | 9 / 6 / 162 / 4 (181) | 1,205 / 2 / 3,590 / 7 (4,804) |

There are no moved rows. `changed_rows.py` finds no row whose status or
leaf multiset differs, and `moved.py` reports `{}`. The reports are not
byte-identical. `evidence_version` carries the measuring checkout's git
HEAD: both side trees sit inside this worktree, so they read `a971680e9`
and `d65d11761`. The reading-site environment digests derived from it differ
too, and each function's reading-site evidence is listed in digest order.
[identity.py](scripts/identity.py) blanks those fields and compares the
evidence as a multiset. With that, all 4,804 function rows and all 181 unit
statuses are identical ([identity.json](artifacts/identity.json);
[totals](artifacts/)). Binary SHA-256: base `fe5c6770…`, tip `da958902…`.

The runs used opt-in `current --monkey-linked` over
`private/fb-046accf6`, each from its own `git archive` tree and release
binary ([scripts](scripts/), copied from
return-family-untracked-member-cast). Logs:
`private/rmc/run-{base,tip}/` in this worktree. The system-slice route was
not run, because rule 92 is not on any route. The pinned routes
(`quirin-audit measure --declared`) cannot run on this host (no
`build/cycle19` inputs), so no pinned figure is claimed.

## 5. Gate

GATE_RESULT

## 6. Follow-ups

Registered in [integration-amendments.json](integration-amendments.json):

* FB-LEDGER-LOG-ARGUMENT-ORDER: rule 92 over the pinned bodies.
* FB-LEDGER-LOOP-SITES: the discharge inside its list iteration.
* FB-LEDGER-RECOMPUTED-CHARGE: `fs_chunks_size` conservation.
* FB-LEDGER-ROUTE-WIRING: no route reports ledger readings.
* FB-LEDGER-ADVISORY-LIMITS: which resident-byte bound to certify (a human
  choice).
* FB-LEDGER-SIGNEDNESS-SITES: the two premises of §1.
* GATE-SCOPE-READ-CENSUS-FLOOR: one more root test binary fails the
  Python suite's read-census floor (§5).

Amended: FB-PLAN-CARDS-UNTRACKED (this card's slice and the ids above now
track the plan card), W45-FB-BACKLOG-PUBLICATION-BLOCKED (OB-8's arithmetic
is a rule 92 reading; the custody leg is unchanged), and
W46-FLUENT-ACCOUNTING-LOG-CALLBACK-EFFECT (log calls lie only on clamp
arms, outside the update window). The plan row of FB-BACKLOG-ACCOUNTING is
left to the integrator, as the brief requires.
