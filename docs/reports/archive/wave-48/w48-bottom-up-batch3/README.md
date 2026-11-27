# W48 bottom-up batch 3 — scalar accessor conditions

Base `198f6fc4349d3a47591a2378835ae9286859d918`; branch
`codex/w48-bottom-up-batch3`. Prediction commit
`331129c5985a01ec64e9dc4fcc55feb3a0b0a7bc` precedes semantic implementation
`bb6c2b0c3`. Measured analyzer and repaired test source is
`1601ba4383253adab7e58b0c469c82dd8c94f4d2`; subsequent edits own records only.
The [prediction](PREDICTION.md) is frozen.

## Selection and checked boundary

The detach-lookup census ranked `rd_kafka_log0` first at 3 callers per own
blocker. Its only own call, `rd_kafka_log_buf`, has eight leaves: three indirect
stores, two unresolved `rko` uses, queue enqueue and callback condition/dispatch.
`rd_kafka_q_enq` enters `rd_kafka_q_enq1`, which recursively forwards queues
while transferring or destroying the op. Rule4's neutral prefix-closed fixed
point does not justify this effectful dependency. A new neutral callback,
consumed-op declaration or assumed acyclic forwarding graph would change the
premise boundary. This is a protected class1 ownership/callback choice, not a proof
that future source-derived queue and callback closure is impossible.

`msg_write` and `sched_thread_cleanup` tie next at score2. The latter waits on
`mk_http_thread_destroy`: TLS get/set, unresolved `libco_params`, unpublished
list unlink and `mk_thread_destroy` remain. The bounded class2 source-reading candidate is `msg_write`:
its single leaf is `errno == EAGAIN`, preprocessed on this Linux source to
`(*__errno_location()) == 11`. The existing libc declaration already supplies
no custody result; its typed scalar-pointer reader previously served nested
actuals. This card extends only the reading position.

The adapter accepts a direct external zero-argument call, a complete unambiguous
scalar-pointer declaration, and an in-force contract with no parameters, None
result and no container effect. It reads exactly `*F()` or `(*F())` compared by
`==`/`!=` to one unsuffixed decimal literal. Ordinary call instantiation must
still yield an identity call. The original call node precedes a two-way unknown
scalar decision in the existing CFG, preserving short-circuit and loop frequency.
No value refinement, return-expression extension, actual, write, address,
volatile/atomic pointee, local body, owned result or richer indirect read is
admitted. Existing alias/lifetime/call checks remain operative.

`QCore.ScalarAccessorCondition` proves the five-way admission conjunction over
all32 Boolean vectors and that observation after the ordinary call preserves
its errors and complete custody state while covering both Boolean outcomes.
This is model parity, not proof of source extraction (boundary28). Rust binds
the same32 vectors and retained-source replay. The26 constructor/accessor tests
include leaks and double release remaining Unsafe, missing/owned/local/actual/
write/richer-read refusals, short circuit and while/do/for/switch frequency,
stale-certificate rejection and replay of the verbatim Linux production body.
The fixture is Linux-only; no Windows/macOS errno claim is made.

Only the explanatory `nested_call_admission` premise in `libc/errno.qv` changes;
its four clauses, modes and declared authority remain. The official pin command
updates exactly the flexible contract-text hash in `contracts.lock`, with the
[pin receipt](pin-receipt.json). No new neutrality contract is introduced.
The report-owned [link map](link-map.corpus.json) adds only `mk_fifo_send` to the
six existing homes. Actual link resolution, type/calling-convention agreement
and non-interposition remain explicit linked-program assumptions.

## Reproduction and evidence

Private evidence root: `private/w48-bottom-up-batch3/`. The base executable was
built before implementation at the documentation-only prediction checkpoint;
its analyzer sources equal exact base (the build/head detail is retained in
`private/w48-bottom-up-batch3/base-binary.json`). It was copied immutably. Runtime snapshots use
git archives with307 files and equal file sets; only errno premise text and
its flexible lock binding differ. Original pinned runner bytes are identical.
[Runtime input hashes](runtime-inputs.json) and each route's invocation receipt
bind binary, analyzer source, upstream source, profiles, selection/database,
link map, environment, job count and before/after source/binary identities.
Snapshots are data copies, not other cards' worktrees. The corpus source pin is
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; configured system is
`046accf6aded7906f66a1baf0202da349e521736` and slice is
`34155955e4962a56d94eb23c03a002880899a0d1`, equal before/after per route.

`measure.py` runs the unchanged pinned corpus runner and the existing configured
system/tail-pause routes, with two analysis jobs per route and three routes in
parallel only after Rust/provider tests finish. Every route has fresh output.
All five corpus artifact families, and each route's null JSONL and outer projection,
are archived under `artifacts/{corpus,system,slice}/{before,after}`. Configured
routes emit diagnostics/contracts and empty Unknown chunk/source-effect records
that explicitly refuse historical evidence at the current revision. They do not
emit the source family. Presence, empty results and paired absence are explicit;
none supplies safety evidence. `archive.py` uses the existing
duplicate-preserving row comparator and corpus-repin artifact comparator.
Raw full rows, diagnostics and contract keys are retained. `review_fields.py`
and `review_precise.py` enumerate every changed field; the latter allows only
published finite identity correspondences and requires full payload equality
where it classifies source-effect changes as identity-only. Checkout prefixes
are normalized for comparison only, with collision checks. Null and runtime
comparisons retain their own denominators and every moved record.

## Measured outcomes

All six fresh audits exit0. Static custody is conditional on the loaded
contracts, named null premises and selected linked-program premise; it is not
unconditional C conformance.

| Route | Selected rows | Before Safe / Unknown / Pending / provisional Unsafe | After |
|---|---:|---|---|
| Corpus | 4,834 | 963 / 3,862 / 0 / 9 | **967 / 3,858 / 0 / 9** |
| System | 448 | 99 / 345 / 0 / 4 | unchanged |
| Tail-pause slice | 404 | 53 / 346 / 4 / 1 | unchanged |

The three prediction targets become custody Safe: `msg_write`, same-unit
`mk_fifo_send`, and `mk_mq_send` under the explicit `mk_fifo_send` home premise.
The fourth gain, Linux `net_plain_send_file`, is an incidental exact
`errno != EAGAIN` comparison anticipated by the prediction's incidental-shape
allowance. Its existing `sendfile` contract is unchanged. All963 prior Safe
rows remain Safe, and all9 provisional Unsafe rows retain their status.
`mk_net_conn_create` loses an errno refusal but remains provisional Unsafe;
this is no new validated upstream finding.

Corpus has23 changed diagnostic/status rows:22 condition refusals and three
call leaves disappear (two `msg_write`, one `mk_fifo_send`); no diagnostic is
added. [Attribution](artifacts/corpus/attribution.json) retains each row,
changed source location and file hash. Source, chunk-contract and source-effect
payloads are equal after the explicit checkout-prefix correspondence. Contract
module key sets agree; errno is the only changed module text.

The added home is independently replayed in caller sessions. Its Monkey header
bodies therefore appear repeatedly in unit evidence: `MK_EVENT_NEW` validated
occurrences4→180, `mk_mem_alloc`19→180, `mk_mem_alloc_z`16→180 and
`mk_mem_free`26→180; unvalidated `mk_fifo_queue_destroy`1→180,
`mk_list_add`18→180 and `mk_list_del`17→180. These are replay occurrences,
not distinct native definitions or new proof methods. The
[complete body-evidence review](artifacts/corpus/body-replay-occurrence-review.json)
checks every added validated clause payload against one already present before,
retains every old payload and accounts for all other additions as unvalidated.
All declaration standing classes, methods, failed/withdrawn counts and modes
remain unchanged. Errno remains flexible-standing / declared-assumption;
its usage160→162 rows and7→11 Safe rows does not validate its implementation.
The initial authority guard's exit1 on these occurrence counters is retained;
the final review distinguishes occurrence counts from authority.

System has no status or diagnostic changes; four raw rows differ only by
checkout review-hold paths. Complete diagnostic and contract payloads agree
under the finite substitutions in its context review.
Slice has13 changed diagnostic rows, all staying Unknown: nine condition
refusals and six helper-call refusals disappear, while three unresolved `end`
identities become visible. Source inspection follows Linux
`FLB_PIPE_WOULDBLOCK()` to errno comparisons and ordinary same-unit W4 summaries
for `tail_signal_pending` / `tail_consume_pending`. Existing read/write/error
declarations propagate to six callers. Two `flb_input_set_property` occurrences
newly carry a custody null premise for `end`; no pointer fact is inferred.
Eight other raw rows differ only by checkout paths. The
[slice attribution](artifacts/slice/attribution.json) retains every change.
The identity-only guard correctly refuses to classify these as identity changes;
the final context checker retains semantic differences and separately records
four declaration-usage increases. No standing changes.

All4,921 /454 /406 null verdict dictionaries agree (5,781 total). Corpus null
rows change only four custody fields and two combined fields: `msg_write` and
`net_plain_send_file` become combined Safe; `mk_fifo_send` and `mk_mq_send`
remain combined Unknown. Corpus combined Safe841→843. Slice changes two
`custody_null_premises` carriers; its null dictionaries and verdicts agree.
Every runtime scenario's moved records are identical. Corpus runtime counts
follow the same four custody/two combined static gains; no new envelope or
runtime coverage is inferred. System/slice scenario counts are identical.
See each route's `projection-comparison.json` for separate denominators.

## Fresh ranking

The unchanged original planner is run on both fresh corpus reports. The
[complete plans](artifacts/before-plan.json.gz) and
[after plan](artifacts/after-plan.json.gz), plus the
[score ranking/readiness record](artifacts/ranking-readiness.json), retain every
row. Candidate bodies2,222→2,220; hypothetical all-body ceiling362→360;
single-body waits256→254; no-extra-body same/cross closure remains6/6.
None is a promised Safe gain. Logger remains score3 with its unclosed ownership/
callback dependency; scheduler cleanup remains score2 with its five-leaf callee.
The next cJSON object detach rows still require richer return/Removed transport.
No readiness is inferred for the remaining unreviewed ranking rows.

All six detach/delete wrappers remain Unknown. The existing broader bottom-up
tracker stays open, with the current ranking and separately scoped slice `end`
residuals. The hypothesis-graph candidate remains independent.

## Validation and attempts

The [provider receipt](provider-receipt.json) records all13 commands exiting0:
workspace check, `cargo test --workspace --no-fail-fast` (133passed/0failed/
1ignored doctest), fmt, Clippy, contracts sync/check,25 Python tests, and
`providers/package.py build` plus verify for all five packages. Private command
logs are under `gate-providers-1/`. Provider results are freshly measured.


The full final Rust runner passes **8,359 / 0 failed / 15 ignored**,520 units,
533 summaries, zero filtered, exit0. Its [receipt](rust-receipt.json) selects
counts and timing without publishing the runner's complete captured environment.
Private logs: `gate-rust-2/rust.log` and `gate-rust-2/rust-units/summary.json`.
The full pinned Lean build and **345 audits** exit0 (`lean-gate-1.log`), with
[proof hashes](lean-receipt.json); no Lean source changed afterwards.
`cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`
exit0 after the test-fixture repair (`fmt-after-fixture.log`,
`clippy-after-fixture.log`). Both `cargo run -p qcore-compiler --example
reevaluation_probe` and `reevaluation_followup` exit0 (`source-tools-1/`).
Recursive submodule initialization exits0 in this fresh checkout. RootPython
passes374 after the initial matrix regeneration and again after the final
gate-backed regeneration (374/0failed in38.646s, exit0), recorded in `command-receipts.json` and
`private/w48-bottom-up-batch3/python-final.log`. Documentation index generation
and its check are recorded there too. All command exits/counts are retained in
[the command receipts](command-receipts.json) and [gate](gate.json).

The [focused attempts](focused-attempts.json), [preparation attempts](preparation-attempts.json)
and [interrupted attempts](interrupted-attempts.json) retain failures rather
than replacing them with the last green result. The initial three baseline
audits lost their execution session without final report/exit and are excluded.
The first full Rust gate ran all units and exited101:8,358passed/1failed/
15ignored. Its sole failure was an existing fixture assuming `.git/modules`
beneath a linked worktree's `.git` file. The test now asks the initialized
submodule for its absolute git directory and retains both scoped ownership
exceptions and every assertion. The focused repaired control passes1/0failed;
the full rerun above passes. This repair changes test setup, not analyzer semantics.
Rustfmt also normalized three existing unformatted files. Early red fixture,
compile/import, Lean simplification, missing usleep declaration and stale errno
lock attempts have their own exits and logs. No failure is counted as validation.

## Follow-ons

- `W48-BOTTOM-UP-RESIDUAL-PLAN` remains open with this card's fresh ranked plan.
  The logger's queue transfer/destruction and callbacks require explicit checked
  source/effect closure or a deliberately chosen premise; no neutral substitute
  is authorized here. `sched_thread_cleanup` retains its dependent TLS/list/
  thread-destroy leaves. Each body needs its own readiness reading.
- The configured slice still refuses unresolved `end` identities in
  `flb_input_set_property` and `configured_limit`. Scalar errno tests supply no
  pointer fact; two newly exposed custody null-premise carriers remain on
  Unknown rows. These are separate from the corpus ranking.
- Existing cJSON successor/object lookup and whole-return/nested Removed
  transport remain open. Scalar observation grants no child membership or
  richer return transport, and all six detach/delete wrappers remain Unknown.
- Scalar accessor observation remains exact-shape and condition-only. Other
  accessors, platforms, memory validity and source-extraction correspondence
  need independent obligations; the named libc premise remains declared.

## For the integrator

Keep caller-session home replay occurrences distinct from independent C-body
conformance counts. The new home's header evidence is copied into many caller
collectors; this card records that multiplicity without claiming new standing.


Integrate serially from this isolated branch; it does not advance main. Preserve
this report's exact analyzer/input identities, prediction chronology, failing
attempts and distinction between model proof, source replay and declared C
contracts. Rerun integration measurements if merged semantics change; publish
a separate integration gate when counts differ. Keep `mk_fifo_send` conditional
on the explicit selected-program link premise rather than promoting the map to
a source fact. Reconcile the shared `STATUS.md`, acceptance-matrix follow-up,
report index and current part10 increment against sibling cards without
rewriting their frozen reports. Preserve the approved four-clause errno count
and its deliberate flexible lock update. Likely conflicts also include gateway
module registration/condition desugaring/rules, `proof/QCore.lean`, libc errno
text/lock and the native-platform worktree fixture.
