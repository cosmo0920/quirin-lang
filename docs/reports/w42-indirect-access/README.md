# W42-INDIRECT-ACCESS — bounded scalar destination stores

This card adds one typed route for an exact `BASE->FIELD = RHS` statement when
`BASE` may carry a tracked resource identity. It does not admit general
indirection, pointer-member stores, member chains, subscripts, dereferences,
calls or mutating right-hand sides. The frozen pre-measurement forecast is
[PREDICTION.md](PREDICTION.md).

## Implemented rule

The classifier requires all of the following:

1. the token shape is exactly `BASE -> FIELD = RHS`;
2. `FunctionTypes::may_bind(BASE)` is true, making this route disjoint from the
   existing scalar-field no-op for bases that cannot bind tracked identities;
3. `BASE` is a declared record pointer and `FIELD` is a declared scalar field;
4. `RHS` is nonempty and scalar-only, with no assignment, increment, decrement,
   call, member access, chain, subscript or dereference.

The explicit `ScalarMemberStore` transfer retains the existing same-type alias
read obligation. It admits the dereference only when the base denotes exactly
one known, non-null, live identity with neither a nullable marker nor an
unresolved-effect marker. The scalar destination and scalar-only RHS cannot
rebind tracked identities. Both an admitted store and a flow refusal therefore
preserve pointer, custody, field-link and scalar-fact maps; the refusal records
`UnknownAlias`, so it cannot make the function `Safe`.

Pointer fields remain refused by this route; shapes outside this conjunction
retain their prior handling, including the separate unbound scalar-field
no-op.

## Validation of boundaries

The 11 source tests cover a live tracked base, released and unguarded nullable
bases, guarded nullable admission, a released same-typed peer, a distinct fresh
acquisition, pointer fields, indirect and mutating RHS shapes, explicit
lowering, stale scalar-branch facts, local W4 summary laundering, and refusal
frame preservation. The frame regression acquires independent `p` and `q`,
releases `p`, refuses `p->state = 1`, and requires both `UnknownAlias(p)` and
the still-visible leak of `q`. The mutation test uses `flag++` and a reachable
double release to distinguish the true branch from a stale `flag == 0` fact.

`QCore.ScalarMemberStore` states the exact syntax/type and flow premises. Its
transfer result preserves the projection in both admitted and classified
flow-refused cases, and the latter theorem also requires the unknown-alias
reason. `AuditScalarMemberStore.lean` reports no project axioms or placeholders;
the Boolean decomposition theorems use only permitted `propext`. C extraction,
declaration reading and correspondence to the compiled Rust remain review
premises under evidence rule 28.

Focused validation before the corpus run:

| Command | Exit | Result |
|---|---:|---|
| `CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p qcore-c-gateway --test scalar_member_store -- --nocapture` | 0 | 11 passed, 0 failed |
| `lake build QCore.ScalarMemberStore` | 0 | module built |
| `lake env lean AuditScalarMemberStore.lean` | 0 | audited declarations printed; no forbidden axiom |

## Isolated corpus measurement

The root BEFORE at
`/home/cosmo/Documents/GitHub/qcore-c/private/w42-priority/baseline/` was a
fresh exit-0 reproduction of the B-series final artifacts. The corrected AFTER
at `/tmp/agent-w42-indirect/private/w42-priority/after-frame-final/` used source
commit `c3f500d676024de1a496c2b6b4f309720e3f099e`, release-binary SHA-256
`f846400f8ac616856d2a005c3b08e6918d4bc3b1aa9474824646229ccf60abfe`,
the same absolute pinned source, compile database and ChunkIO package, and
exited 0. `run.json` records the exact command and UTC interval
`2026-09-20T02:41:19Z`–`02:47:43Z`.

| Measurement | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| projected functions `Safe` | 221 | 223 | +2 |
| projected functions provisional `Unsafe` | 84 | 84 | 0 |
| projected functions `Unknown` | 4,529 | 4,527 | -2 |
| `unsupported` | 49,735 | 49,371 | -364 |
| `unknown_alias` | 4,309 | 4,665 | +356 |
| `leak` | 272 | 272 | 0 |
| `summary_conflict` | 1 | 1 | 0 |

Units remain 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only; raw
units remain unpromoted. `ctraces/ctr_span.c::ctr_span_start_ts` moves directly
from Unknown to Safe. Its newly available summary removes the uncontracted-call
reason in `ctr_span_start`, which also moves Unknown to Safe. No other function
or unit moves.

The diagnostic multiset removes 365 leaves and adds 357, a net removal of 8.
The removed set is 362 lexical indirect-access refusals, two uncontracted-call
leaves in the newly summarized `ctr_span_start`/`ctr_span_start_ts` chain, and
the prior `ctr_span_create` unresolved-`ctx` alias reason. Of the 362 lexical
refusals, 350 exchange at the same coordinate for a specific `UnknownAlias`;
12 lexical refusals and the two downstream call leaves have no same-coordinate
replacement. Seven additional downstream `UnknownAlias(p)` leaves appear in
`monkey-libevent`'s `mk_http_parser`, so the category total rises by 356. The
full multiplicity-preserving comparison is archived as
`artifacts/frame-delta.json.gz`; the private uncompressed result is
`private/w42-priority/comparison/frame-delta.json`.

The forecast predicted 349 removed indirect leaves, zero additions or
exchanges, and one Unknown-to-Safe mover. Measurement found 362 retired lexical
indirect refusals plus two removed call leaves, 350 direct exchanges plus seven
downstream additions, and two movers. The second mover is the transitive caller.
This is a recorded prediction miss, not a rewrite of the frozen forecast.

This measurement does not discharge `INDIRECT-EXCHANGE-PROXY-UNVALIDATED`.
The historical 349-leaf census was a static proxy over its own shape, while
this card measures the narrower typed route plus flow refusals; no coordinate
crosswalk proves that the two populations are identical. The three-way outcome
above is the measured card subset, not a validation of every leaf counted by
the proxy. `INDIRECT-CENSUS-FEATURE-UNBUILT` is also unchanged and unrelated to
this admission.

The ChunkIO contract artifact and Monkey source artifact are byte-identical to
BEFORE. The selected-contract artifact differs only because its 33 source-map
keys embed the root checkout path in BEFORE and the `/tmp/agent-w42-indirect`
path in AFTER; the contract payloads match after that key normalisation. The
source-effects artifact differs only in the boolean-field configuration,
build, obligation and task identity digests at result indices 16, 24 and 63,
which bind the worktree-sensitive contract source identity. No source-effect
verdict payload changes. The root's same-checkout combined AFTER is the proper
test for byte identity. Leak count 272 is unchanged.

An earlier AFTER at `private/w42-priority/after-final/` used the draft blanket
poison on flow refusal. It withdrew four leak leaves and is superseded evidence,
not the card result. The independent-`q` regression exposed why that behavior
was wrong: a typed scalar-only store cannot rebind unrelated pointer identities.
The earlier, still older `private/w42-priority/after/` run used the pre-review
eligibility draft and is also excluded. Both directories and identities remain
preserved rather than folded into the corrected measurement.

## Standing gate

The corrected code checkpoint `e90e2958` was measured outside the sandbox
because LeakSanitizer refuses execution under ptrace; leak detection remained
enabled. The run used
`CARGO_BUILD_JOBS=1`, `CARGO_PROFILE_TEST_DEBUG=0` and Lean `--jobs 1` after
recursive submodule initialization. Raw logs and direct exit files are in
`/tmp/agent-w42-indirect/private/w42-priority/gate-final-2/`.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned submodules initialized |
| `clang --version` | 0 | Ubuntu Clang 21.1.8 recorded beside `gate.json` |
| `cargo test --workspace --no-fail-fast` | 0 | 4,857 passed, 0 failed, 10 ignored; 249 strict summaries across 247 targets (238 running, 9 doc) |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | passed |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | passed |
| `python3 scripts/check_lean_audits.py --jobs 1` | 0 | 149 audits passed |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w42-indirect-access/gate.json` | 0 | isolated matrix regenerated; 149 Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 305 passed |

The direct commands, timings, environment, exits and Rust counts are in
`gate-final-2/gate-facts.json`; Python output and its direct exit are
`python-tests.log` and `python-tests.exit`. This isolated branch commits its
regenerated acceptance matrix solely to satisfy the matrix-consistency test.
The integrator must retain its combined follow-up ledger and regenerate the
final matrix from the combined gate rather than merge this snapshot
arithmetically.

The provider count is carried evidence; this card does not change or remeasure
the provider. Full-language conformance remains false.

The preceding run at `private/w42-priority/gate-final/` is retained failed
evidence. Workspace tests and Clippy exited 101 before producing any test
summary because `alias_aggregate_probe` had an exhaustive instruction printer
that did not yet render `ScalarMemberStore`; every other command through the
149 Lean audits exited 0. Commit `e90e2958` adds the base and alias-obligation
state to that diagnostic rendering. This example-only repair does not alter the
production audit binary, so the corrected corpus result remains bound to its
recorded binary digest without a redundant rerun.

## Integrator reconciliation

The root integrator owns the shared status, plan, increment, router and final
combined acceptance matrix. This card proposes:

1. add the following gateway amendment, with a router entry:

   > W42-INDIRECT-ACCESS adds one route disjoint from the existing unbound
   > scalar-field no-op: an exact `BASE->FIELD = RHS` statement may lower to
   > `ScalarMemberStore` only when `FunctionTypes::may_bind(BASE)` is true,
   > `BASE` is a declared record pointer, `FIELD` is a declared scalar field,
   > and `RHS` is nonempty, scalar-only, contains no
   > member/chain/subscript/dereference/call/assignment, and contains no
   > increment or decrement. Transfer retains the existing same-type
   > alias-read obligation and admits the store only when `BASE` resolves to
   > exactly one known, non-null, live identity with neither nullable nor
   > unresolved-effect markers. The scalar destination and scalar-only RHS
   > cannot rebind tracked identity, so both admitted stores and flow-refused
   > stores preserve pointer, custody, field-link, and scalar-fact maps; a flow
   > refusal still records `UnknownAlias` and keeps the function non-Safe.
   > Pointer fields remain refused by this route; shapes outside this
   > conjunction retain their prior handling, including the separate unbound
   > scalar-field no-op.

2. update current measured function counts to 223 Safe / 84 provisional Unsafe
   / 4,527 Unknown and diagnostic counts to unsupported 49,371 / unknown_alias
   4,665 / leak 272 / summary_conflict 1 after the combined AFTER confirms the
   isolated attribution;
3. record the two mover identities, 350 exact-coordinate exchanges, seven
   downstream additions and eight-leaf net reduction rather than the frozen
   proxy forecast;
4. add the required newest-first increment paragraph and regenerate the final
   combined acceptance matrix from the integrator's own gate;
5. retain every registered follow-up. This card retires none and does not alter
   the frozen leak-272 history.

## Integration confirmation — 2026-09-20

Merged through `f3f99df5d2dcdf27e9b73ecd81dd7fe126419ca4`; combined source `f3f99df5d2dcdf27e9b73ecd81dd7fe126419ca4`
passed **4,863 Rust / 149 Lean / 313 Python**, zero failures,
with fmt, Clippy, both probes and the isolated host checks green. The
[integration gate](integration-gate.json) retains the combined measurement;
its colocated Clang record is explicitly borrowed from this card, while the
[combined report](../w42-priority-integration/README.md) carries the actual
combined recording, pinned AFTER, exact attribution and remaining limits.
The isolated checkpoint above remains unchanged.
