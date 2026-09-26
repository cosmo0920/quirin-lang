# W45-SR5-SOUNDNESS — the admitting-direction holes STABILITY-REEVAL-5 found

**Card:** W45-SR5-SOUNDNESS, wave 45. **Base:** local `main` at `33558c67`
(STABILITY-REEVAL-5 had not landed; its report, probe and entries were read from
`integrate/stability-reeval-5` without checking it out). `main` was merged
twice while the card ran: `200fdcbe` (STABILITY-REEVAL-5) and `b1744a2a`
(DOCS-COMPACT-9); both merges were documentation only, and the rule edits were
applied onto DOCS-COMPACT-9's compacted gateway page.
**Prediction:** [PREDICTION.md](PREDICTION.md), frozen at `cf3c124e` before any
Rust or Lean change.

STABILITY-REEVAL-5 measured a `Safe` published over a double release, measured a
`Pending` forged by source text, and inferred eight further candidates. This card
probed every candidate with a one-off test, fixed each one that probed as admitting
or as a lowered verdict, and pinned both the fixed shapes and the two that probed
fail-closed. Every change refuses. **No function or unit moves on the 185-unit
corpus or on the Fluent Bit slice**, and nothing is promoted.

## 1. What the probe measured

The probe ([source](probes/zz_sr5s_probe.rs.txt), 72 rows) ran as a temporary
`crates/qcore-c-gateway/tests/` target over the synthetic `q_*` fixture contracts
and inline `k_*`/`u_*`/`mk_list_*` declarations. It ran once at the base
([probe-before.log](probes/probe-before.log)) and once at the final code
([probe-after.log](probes/probe-after.log)); each run exited 0, and the file was
then removed. `Safe` here promotes nothing.

| Finding | At the base | After | Class |
|---|---|---|---|
| `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` | `Safe`: `q_nullable_free(n->buf); node_destroy(n);`, its null-tested form, the same through `node_drop(n) { node_destroy(n); }`, with a declaration for the unit-defined destructor, and `n->next` before `node_destroy_next(n)`; also the unit verdict | `Unsafe` (`DoubleRelease`) with the refusal `member-released-again-by-callee`; the unit is `Unsafe` | admitting, fixed |
| (its neighbours) | `n->buf` before `node_destroy_next(n)`, the destructor alone, the declared-only destructor: `Safe`; mirrored order and both aliases: `Unknown` | unchanged | controls |
| `HELD-PREFIX-SOURCE-FORGES-PENDING` | `Pending` for `qcore_pin_mismatch__zz(0);`, `qcore_review_pending__zz(0);`, a spelling that collides with a withheld name, a unit-defined prefixed function, and as a unit verdict | `Unknown` (`reserved-prefix-in-source`); a genuine hold stays `Pending` | forgery, fixed |
| `PUBLICATION-ROOT-RELEASED` | `Safe` after `q_free(q)` and after a unit-defined `queue_destroy(q)` | `Unknown` (publication refused) | admitting under an unstated premise, fixed |
| `COUNTED-USER-CONDITIONAL-OPERAND` | `u_ref(t); c ? u_unref(t, 0) : (void)0; return 0;` holds `restored`, the body and its caller `Safe`; `&&` likewise; `pick(1)(t)` hides a call and the caller is `Safe` | clause `Undecided`, body and caller `Unknown` | admitting, fixed |
| `DECLARED-NEUTRAL-UNVERIFIED-AT-CALL` | a declared-neutral body writing the counter (or calling `u_ref`) in a loop: its caller `Safe` | caller `Unknown`; the verifying control stays `Safe` | admitting, fixed |
| `CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER` | `link_it(void *arg, …)` `Safe`; a caller that frees the element it linked `Safe` | both `Unknown` | admitting, fixed |
| `SCALAR-READ-PARENTHESISED-DOUBLE-DEREF` | the reader admits `(*q)->len`, `(*q)[0]`, `**q` (read); end to end a formal is untracked, so no `Safe` differs from its plain two-dereference neighbour | refused (`Unknown`) | premise breach, fixed |
| `PENDING-BYPASSES-UNIT-SAFE-GATES` | a genuine held call beside an `#if 0` or K&R definition: a `Pending` unit (the same unit without the hold is `Unknown`) | `Unknown` unit, the row keeping its `Pending` reason | forgery, fixed |
| `GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK` | re-publication under an alias head: `Unknown` | unchanged | fail-closed, pinned |
| `STATIC-ROOT-SHADOW-READER` | three shadowing declarations containing `(`: `Unknown` | unchanged | fail-closed, pinned |

## 2. Mechanisms

Each repair lives in the module it corrects, behind a hook that names the card.

1. **Rule 5k reads what every `R(B)` does to `B->F`**
   (`legacy_member_release.rs`, section 4). P1 accepted `R(B)` on its
   contract's release mode alone, and derived summaries are in force.
   `MemberEffect` now characterises a callee this unit defines: its only formal
   `p` may be used only as `p->G`, in null tests, and as the single actual of
   a callee read the same way, to a fixed point (a cycle is unknown). A callee
   whose body releases `B->F` and follows the release unconditionally (P4's
   shape) makes that release statement refuse under
   `member-released-again-by-callee`, and `analyze_defined_function` reports a
   `DoubleRelease` at its line (the member has no acquisition in this body, so
   the release line stands in for one). A callee that only touches `B->F`, or
   is unknown, withholds the place. A callee with no body here is governed by
   its declaration: a declared release does not cascade to members
   (P-B08-SHALLOW, stated). The reading covers declared callees the unit
   defines too; it only withholds.
2. **A held call is only ever tool-minted** (`legacy_reserved_spelling.rs`,
   gateway 10i). Before any of the six prefix rewrites runs, on the three
   analysis routes and the mode census, every identifier the source spells
   with a reserved prefix (the escape prefix `qcore_source_spelled__`
   included) is renamed with the escape prefix, consistently across the unit.
   After it, every marked name a reader sees was minted from the environment's
   withheld, pending, refuted or withdrawn set. An escaped callee refuses as an
   uncontracted call. The stale comments in `legacy_pin_hold.rs` and
   `legacy_review_hold.rs` are corrected.
3. **A released container root discharges no publication**
   (`legacy_container_publication.rs` rule 7, gateway 10d). The premise was
   decided by reading 10d/10f: 10d discharges because the container outlives
   the call, and no rule states that a container's release hands on its
   elements; stating one would be a new contract obligation for every list
   API. So the shape is made `Unknown`. `CallerFormals::with_released_roots`
   collects the stable roots handed whole to a position whose contract in force
   is not `Borrowed` (a direct release, a summary's `Released`, a consumption,
   a variadic tail); a formal and a local initialised from it alone share it.
   Only a publication reads the set: the first AFTER run showed that applying
   it to 10f's foreign unlink refused four teardown bodies that unlink elements
   and then release the root (§4).
4. **An element bound over a formal takes the formal's rule** (rule 8):
   `ForeignOrigins::note_over_formal` records every name 5l(2) bound from a
   never-bound formal, and `settle_foreign_unlinks` marks a publication of it
   as a formal entry: an unresolved identity, never the borrowed reading.
5. **The counted-user walker** (`legacy_counted_users.rs`, gateway 10e)
   undecides a call or counter write behind `?:`, `&&` or `||` (a superset of
   the positions C may skip), and treats a call of a call's result as opaque;
   the unit pin that read `f ( a ) ( b )` as a named call is inverted, as 10c's
   rule 2 requires. A declared `resource_user_neutral()` of a body the unit
   defines goes through `verify_declared`, as every other declared clause does.
6. **Rule 5j's reader** (`legacy_return_family.rs`) counts a `*` through
   parentheses and ends the one dereference only after closing parentheses.
   A starred root counts only the `*`s whose parentheses close right after the
   name, so a `*` applied to a cast group never counts for the type name inside
   it (the first AFTER run showed `rd_list_get_int32`'s
   `*(int32_t *)rl->rl_elems[idx]` losing its refusal through exactly that, §4).
7. **A `Pending` unit passes the `Safe` gates** (`legacy_c.rs`, `projection.rs`,
   gateway 10i): the unread-definition gate and the certificate target scope run
   on the `Pending` arm, and the audit's inventory withholding applies to a
   `Pending` unit; any failure makes the unit `Unknown` (the join with
   `Unknown`), each function row keeping its own held reason.

## 3. Lean

Three modules, each registered in its own audit, with no `sorry` and no project
axiom (`check_lean_audits.py`: 210 audits, 207 before):

* `QCore.MemberReleaseCallee` (`AuditMemberReleaseCallee.lean`, 13 theorems):
  under the repaired admission **an admitted member is not released again
  through any callee's summary** (`an_admitted_member_is_not_released_again`),
  a reported place is never admitted, an uncharacterised callee withholds, a
  wrapper carries its callee's releases, and witnesses for the probe's rows.
* `QCore.ReservedSpelling` (`AuditReservedSpelling.lean`, 12): no escaped name
  carries a marker when the escape prefix diverges from every marker; a marked
  name after minting was minted from the withheld set; hence
  **`pending_only_from_a_minted_hold`**; a genuine hold stays `Pending`; the
  measured forgery was `Pending` and is `Unknown`.
* `QCore.Sr5Soundness` (`AuditSr5Soundness.lean`, 15): a released root
  discharges nothing and an alias shares the release; an element over a formal
  is never borrowed; a skippable or opaque event holds no clause; an unverified
  neutral body is opaque; an admitted read dereferences exactly once; **a gated
  unit is never `Pending`**.

What the Lean does not cover: that the Rust readers compute these inputs (the
member effect, the lexed identifiers, the `Borrowed` positions, the skippable
tokens, the star count) is trust boundary 28, and P-B08-SHALLOW is a hypothesis.

## 4. Arbiter

`docs/reports/w43-return-expression/scripts/run_audit.py OUT COMMIT --jobs 8`,
each run into a fresh private directory.

| Run | Tree | Binary sha256 | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `33558c67` | `42b64124…` | 0 | [before-run.json](arbiter/before-run.json) |
| AFTER (first) | `d796ad67` | `2e24f41b…` | 0 | [after-first-run.json](arbiter/after-first-run.json); superseded |
| AFTER | `26e3c60d` | `b9226320…` | 0 | [after-run.json](arbiter/after-run.json) |

**BEFORE and AFTER against the baseline**
(`docs/reports/w45-container-root-reach/integration/artifacts/`):
`baseline_identity.py` exits 0 for both; the diagnostics, source and
chunk-contract artifacts are byte-identical
([before](arbiter/before-identity.json), [after](arbiter/after-identity.json)).
The contracts sidecar differs only in the top-level `refuted_contracts` key the
brief names, and source-effects only in its hex digests
([sidecar_check.py](scripts/sidecar_check.py), exit 0,
[before](arbiter/before-sidecars.json), [after](arbiter/after-sidecars.json)).
The harness refused a command line that spelled the integrator worktree's path,
so `--root-alias` was not passed; the sidecar script reads each checkout root
from the files instead.

**Attribution.** AFTER against BEFORE: 0 removed and 0 added leaves, 0 moving
functions, 0 status moves, 0 unit moves
([after-vs-before.json](arbiter/after-vs-before.json)). Functions stay
**398 Safe / 9 provisional Unsafe / 4,427 Unknown**, units 5 / 8 / 168 / 4, leak
34, unknown_alias 3,571, unsupported 41,314, Pending 0. **No correction, no new
provisional Unsafe, no leak or violation to classify.** The baseline does not
move, so nothing is archived as a new one.

The first AFTER run ([after-first-vs-before.json](arbiter/after-first-vs-before.json))
moved leaves in six functions, all `Unknown` before and after; both causes were
this card's over-reach and are corrected in `26e3c60d`:

* `mk_rconf_free`, `mk_string_split_free`, `mk_server_listen_exit`,
  `mk_vhost_destroy`, `mk_vhost_handler_free` (and the slice's
  `flb_task_queue_destroy`): the released-root set had also withdrawn 10f's
  foreign unlink from teardown bodies that unlink elements, then release the
  root (+8 `unlink … not published` / `unresolved resource identity` leaves,
  −2 `unknown_alias`). The set now reads publications only.
* `rd_list_get_int32` (`rdlist.c:588`): the first star counter counted the `*`
  of `*(int32_t *)rl->…` for the cast's type name, so the return lost its
  refusal (−1 leaf: an **admitting** leaf move inside an `Unknown` function).
  The counter now requires the parentheses to close right after the name, and
  `* ( q * ) p` is a unit pin.

**Prediction against outcome.** Predicted: no move towards `Safe`, 0 movers for
the prefix, the Pending gates and 5k, and 0 to 5 corrections from the other
repairs. Measured: 0 movers of any kind. The prediction's upper bound was not
reached; the unmeasured corpus reach SR5 inferred for 5k ("rare") is measured
at 0.

## 5. Fluent Bit system slice

A fresh Clang build prepared at `046accf6` under
`private/w45-fb/w45-sr5-soundness/attempt-1` ([prepare-attempt.sh](scripts/prepare-attempt.sh),
exit 0), then `quirin-audit current --selection
contract/profiles/fluent-bit/system-selection.json --jobs 8`
([run-current.sh](scripts/run-current.sh), exit 0) with the AFTER binary
(`8289ccec…`). Compared leaf by leaf with the archived slice
`docs/reports/w45-refuted-contract-withdrawal/artifacts/slice/after-046accf6.json.gz`,
which W45-FB-ENGINE-PROGRESS found equal to the current one: **0 leaves, 0
functions and 0 units move** ([after-vs-archived.json](slice/after-vs-archived.json),
[console](slice/after-26e3c60d.console.log)): units **0 / 2 / 13**, functions
**28 / 2 / 204 / 3 not projected**. The declared neutralities the slice rests on
(`flb_task_retry_destroy`, `flb_output_thread_pool_flush`) are now verified
wherever their defining unit also calls them, and the measurement shows that
moves no clause. The first run showed the foreign-unlink over-reach in
`flb_task_queue_destroy` (+8 leaves, [after-first-vs-archived.json](slice/after-first-vs-archived.json)),
corrected as above. The prediction (no movement) holds.

## 6. Pinned controls

`crates/qcore-c-gateway/tests/sr5_soundness.rs` (15 tests) pins every fixed
shape not `Safe`, with the violation or refusal it now carries, beside its `Safe`
neighbour; the Clang-dependent ones (the 5k unit verdict, the `container_of`
launder, the Pending gates, the two fail-closed candidates) return early without
Clang, as the family does. `crates/qcore-audit/tests/sr5_pending_gates.rs` pins
the Pending gate on the audit route (`quirin-audit contracts` over the held-out
revision-mode fixture: `Pending` alone, `Unknown` beside a K&R definition whose
reason names it). Unit pins: `tests/unit/legacy_return_family.rs` (nine refused
shapes), `legacy_counted_users.rs` (`f ( a ) ( b )` opaque), and the escape's own
tests in `legacy_reserved_spelling.rs`.

## 7. Ledger

Retired, each with its ground in [retired-followups.json](retired-followups.json)
([update_ledger.py](scripts/update_ledger.py), applied once):
`MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE`, `HELD-PREFIX-SOURCE-FORGES-PENDING`,
`PUBLICATION-ROOT-RELEASED`, `COUNTED-USER-CONDITIONAL-OPERAND`,
`CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER`, `DECLARED-NEUTRAL-UNVERIFIED-AT-CALL`,
`SCALAR-READ-PARENTHESISED-DOUBLE-DEREF`,
`GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK` (DOCS-COMPACT-9 revised the
text; this card pins the fixture), `STATIC-ROOT-SHADOW-READER` (fixture arm),
`PENDING-BYPASSES-UNIT-SAFE-GATES`, and `W45-REVIEW-PREFIX-COLLISION-UNPINNED`
(its comment arm, true again once a collision cannot read as held). Amended:
`W45-UNCHECKED-DECLARATIONS-TRUSTED` (a declared neutrality is verified inside
its defining unit; the cross-unit premise stays open).

Registered ([followups.json](followups.json)):
`W45-SR5-DECLARED-RELEASE-DOES-NOT-CASCADE` (5k's bodiless declared release is
read under P-B08-SHALLOW; at most the 7 corpus functions rule 1 moved to
`Safe`), `W45-SR5-PENDING-INVENTORY-ARM-UNDRIVEN` (1 arm), and
`W45-SR5-SCALAR-READ-STAR-THROUGH-CAST` (a pre-existing `*(T *)q->f` gap found
while repairing 5j; 4 textual shapes in the pinned tree, no corpus move).

## 8. Gate

[scripts/gate.sh](scripts/gate.sh), detached, at `e4f10653`, `CARGO_INCREMENTAL=0`,
every exit in its own file, logs worktree-private under `measure/gate-e4f10653/`;
exits and the Clang recording copied into [gate/](gate/). Figures in
[gate.json](gate.json).

GATE_TABLE

The provider gate was not run: nothing under `providers/` changed (constraint 18
not engaged), and no contract was added or changed, so no pin is due.

## 9. Boundaries

* **Premises stated, not proved:** P-B08-SHALLOW for a bodiless declared release
  (registered); the Rust readers compute what the Lean models take as input
  (trust boundary 28); the lexer yields the identifiers the escape reads.
* The new `DoubleRelease` is a **provisional `Unsafe`** candidate over synthetic
  fixtures (the second release needs the member non-null at the call, which
  nothing excludes); it occurs nowhere in the corpus or the slice and is never an
  upstream defect claim.
* Unknowns stay in the denominator; no function entered `Safe`.

## Reproduce

```sh
cargo build --release -p qcore-audit --bin quirin-audit -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
python3 docs/reports/w43-return-expression/scripts/run_audit.py FRESH_DIR COMMIT --jobs 8
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py FRESH_DIR docs/reports/w45-container-root-reach/integration/artifacts
python3 docs/reports/w45-sr5-soundness/scripts/sidecar_check.py FRESH_DIR docs/reports/w45-container-root-reach/integration/artifacts
python3 docs/reports/w45-sr5-soundness/scripts/diff_reports.py BEFORE/resource-diagnostics.json AFTER/resource-diagnostics.json OUT.json
docs/reports/w45-sr5-soundness/scripts/prepare-attempt.sh attempt-N BINARY
docs/reports/w45-sr5-soundness/scripts/run-current.sh attempt-N RUN BINARY \
  --selection contract/profiles/fluent-bit/system-selection.json --jobs 8
python3 docs/reports/w45-sr5-soundness/scripts/update_ledger.py   # applied once
docs/reports/w45-sr5-soundness/scripts/gate.sh LOG_DIR
```
