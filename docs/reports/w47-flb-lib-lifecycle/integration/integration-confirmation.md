# Wave-47 round-7 integration confirmation

Branch `integrate/w47-r7`, HEAD `a1e8d831` = main `c6ea2482` [W47-REVIEW-HOLD-CONCRETE-REASON]
+ merge `a8a323d1` [W47-FLB-LIB-LIFECYCLE] (flb_start re-pinned at 3 trees)
+ merge `a1e8d831` [W47-SERVICE-PROPERTY-KEYS].

Gate measured in the integrator's own worktree at `private/w47-int-k/gate-a1e8d831`
(mechanical gate helper pass; no source or test edits).

## The three landings/merges

```
Merge W47-SERVICE-PROPERTY-KEYS onto the round-7 integration

input_instance.qv and contracts.lock merged without conflict (disjoint
declarations; lock is valid, 66 unique entries in function order).
Composed count pins: v1 declarations 251 -> 253 (flb_destroy +
flb_config_set_property), held 9 -> 11. STATUS and router by three-way
row merge; follow-ups 566; wave-47.md newest first; INDEX and matrix
regenerated. Pin tests, provider Python suite and the card's 12 key
controls pass; pin --check of the 8 changed or added declarations exits
0 at 66910c10, 046accf6 and the 90978ac6 export.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

W47-SERVICE-PROPERTY-KEYS: gate exits and figures

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

Merge W47-FLB-LIB-LIFECYCLE onto main after W47-REVIEW-HOLD-CONCRETE-REASON

lib_engine_thread.qv: the card's resource_lifecycle clauses kept beside
main's review markers (flb_start keeps rewrite_before_acceptance).
contracts.lock: structural union (65 pinned); flb_start, changed on both
sides, re-pinned with quirin-audit pin at 66910c10, 046accf6 and the
90978ac6 export (earlier identity voided); pin --check of flb_start,
flb_stop and flb_destroy exits 0 at all three trees. Records: wave-47.md
newest first, matrix (+ the card's update_ledger.py: 562 -> 565), INDEX.
Pin tests and both Python suites pass.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

## Gate table (private/w47-int-k/gate-a1e8d831)

All steps exit 0: submodule, tests, clean-dev, fmt, clippy, reevaluation-probe,
reevaluation-followup, lean, python, docs-index, provider-tests,
provider-clean-dev, provider-fmt, provider-clippy, provider-contracts-check,
package-build, package-verify-qcore.{chunkio,cmetrics,ctraces,libevent,librdkafka},
provider-python, clang-version. Plus release-build.exit 0, corpus.exit 0,
system.exit 0, slice.exit 0.

Test totals (`^test result: ` lines cross-checked against the runner's totals
line): workspace `test binaries: 354 units, 367 summaries; 6333 passed; 0 failed;
11 ignored` (no `^test result:` lines in this runner's own log; nextest-style
per-binary "ok N passed; 0 failed" lines, all 0 failed, including
`qcore-compiler::test::cycle20_verifier_plugins [exclusive]: 20 passed; 0 failed`
-- no rerun needed). Provider Rust: 43 `^test result: ` lines, sum 129 passed /
0 failed (1 doctest ignored). Lean: 224 audits passed, no axioms beyond the
allowed `propext`/`Classical.choice`/`Quot.sound`. Python root suite: 365 tests,
OK. clang: Ubuntu clang version 21.1.8 (6ubuntu1).

## Pin checks

`target/release/quirin-audit pin --check` (whole crate):

| crate | tree | exit | recorded | in force (other rev) | not in force | stale |
|---|---|---|---|---|---|---|
| quirin-provider-fluent-bit | 66910c10 | 0 | 66 | 0 | 0 | 0 |
| quirin-provider-fluent-bit | 046accf6 | 0 | 66 | 0 | 0 | 0 |
| quirin-provider-fluent-bit | 90978ac6 export | 1 | 15 | 51 | 0 | 0 |
| quirin-provider-libc | 66910c10 | 0 | - | - | - | - |
| quirin-provider-libc | 046accf6 | 0 | - | - | - | - |
| quirin-provider-libc | 90978ac6 export | 0 | - | - | - | - |

The 90978ac6 export exit 1 is the known W47-FB-PIN-EXPORT-IDENTITIES condition
(older declarations lacking an export identity): 51 in force through another
revision, 0 not in force, 0 stale, as expected. `pin --verify-contracts`
exits 0 for both crates.

## Verdict identity (compose3.py, base = docs/reports/w47-loop-owner-instance-flow/integration/artifacts/<scope> x3, since neither round-7 card moves a status)

- corpus: merged 526 Safe / 10 Unsafe / 4,298 Unknown; 0 mismatches; COMPOSES.
- system: merged 31 Safe / 2 Unsafe / 201 Unknown; 0 mismatches; COMPOSES.
- slice: merged 24 Safe / 1 Unsafe / 244 Unknown / 1 Pending; 2 leaf mismatches,
  both expected (see `leaf-diff.txt`): `flb_create` (line 18447, `flb_destroy`
  call moved from uncontracted to awaiting-human-review) and `flb_service_set`
  (line 18792, the held `flb_config_set_property` call at flb_config.c:691
  reported in place of the statement's argument refusal). No status mismatches;
  no mismatch outside these two classes. DOES NOT COMPOSE only because of these
  2 expected leaf diffs (exit 1, as the brief anticipates).

The Pending `src/flb_lib.c::flb_input` rows carry `review_holds` with category
fields: two `must-be-rewritten-before-acceptance` (resource_borrowed(config),
resource_borrowed(data)) and one `acceptable-as-written-after-review`
(resource_return_interior(config, nullable)), each with `reason_code:
awaiting-human-review`.

See `identity-corpus.log`, `identity-system.log`, `identity-slice.log`,
`leaf-diff.txt`.

## Census re-derivation (90978ac6 and 90978ac6~1)

**Typed loop-owner census** (`docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py`
+ `census.py`, rebuilt `typed_dispatch_facts` and `loop_owner_census` examples):
all facts.py and census.py exits 0. Re-derived `fixed.json`, `fixed-configured.json`,
`previous.json`, `previous-configured.json` are byte-for-byte identical
(sha256 match) to the archived census in
`docs/reports/w47-service-property-keys/artifacts/census/`. Site tallies:
fixed discharged 8 / not-discharged 126 / unresolved 10 (148/130/10 configured);
previous discharged 8 / not-discharged 132 / unresolved 7 (151/136/7 configured);
0 sites moved. The fixed test's 3 extra unresolved sites (10 vs 7) are the two
`_WIN32` definitions, as expected. No differences.

**Condvar/join census under the lifecycle module**
(`docs/reports/w47-flb-lib-lifecycle/scripts/census.sh`, rebuilt
`condvar_join_census` example): build/fixed/previous/system/corpus exits all 0.
Re-derived tallies, `lifecycle.status` (clause table), and `lifecycle.conformance`
are identical to the archived census in
`docs/reports/w47-flb-lib-lifecycle/artifacts/census/` for all four readings
(fixed, previous, system, corpus) -- raw JSON bytes differ only in embedded
absolute paths/ordering-irrelevant metadata, but tally/clause_status/conformance
compare equal by value. Clause table: L1 start-requires-unstarted in force,
L2 stop-ends-started in force, L6 destroy-requires-unstarted in force;
L3/L4/L5 refuted by body (all three witnessed by the second/first/no-start join
at `src/flb_lib.c:1058`). C5 (the fixed test's releases) not discharged; the
old test's releases likewise not discharged. No differences found.

## Records

See `gate.json`, `clang-version.txt` + `.exit`, `identity-*.log`, `leaf-diff.txt`,
and the gzip archives of corpus/system/slice under `integration/artifacts/`
with `manifest.json` (sha256, bytes).
