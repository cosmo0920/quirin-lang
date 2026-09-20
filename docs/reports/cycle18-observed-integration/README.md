# Cycle 18 observed composition integration

Base: `a524f4d95eaafd0dc5e879b070b9cf4c0e1a5e08`.
Status: bounded observed milestone implemented and validated; no acceptance promotion.

Three GPT-5.6 Sol agents at medium reasoning implement independent composition
context admission, sealed timed-wait source projection, and identity-bearing
runtime observation. The integrator owns configured audit wiring, cross-module
review, final gate, evidence reconciliation and publication.

## First executable milestone

A dedicated observed execution of the pinned Monkey timed-wait path is bound
to its callback and wrapper source, compile/build/profile identities, observer
and linked libraries, exact run records and lifecycle/argument identities.
The checker consumes the complete dedicated trace. It never filters unrelated
callbacks out of the older combined replay or promotes supplied booleans.

A successful artifact describes only its named run, selected path, assumptions
and bounds. Unobserved allocation/registration failures remain Unknown. The
existing three-callback accounting and Cycle 18 aggregate acceptance stay open
until their own source/path and platform obligations are discharged.

## Subsequent capability work

`cb_event` still requires all-path payload identity/bounds and write authority.
`cb_timeout` requires self-teardown and separate event/map/descriptor custody.
General callback/task handoff needs enforced spawn/join/cancel semantics;
currently declared task/deferral flags do not establish those properties.
Native Libevent sanitizer coverage and the deployed observation adapter need
host-specific evidence; a Linux run cannot stand for kqueue or Windows IOCP.

The parent `MONKEY-CALLBACK-CYCLE18-COMPOSITION` remains registered and open.
Five narrower follow-ups record the next discharge boundaries:
`CYCLE18-TIMED-WAIT-FAILURE-PATHS`, `CYCLE18-CB-EVENT-PAYLOAD-BOUNDARY`,
`CYCLE18-CB-TIMEOUT-SELF-TEARDOWN`, `CYCLE18-CALLBACK-TASK-HANDOFF` and
`CYCLE18-NATIVE-PLATFORM-MATRIX`. Their acceptance-matrix entries state explicit
discharge conditions. No existing follow-up is retired by this observed run.

## Integration requirements

Reuse the existing generic event kernel and sealed reactor imports. Keep new
semantic work in the layered crates, with matching audited Lean. Context
identity equality is a binding check, not attestation of an observation producer.
Preserve all existing verdicts and pinned evidence unless a separately scoped
property is actually discharged. No change to full language conformance is
implied. Reports from each card supply the reconciliation list and exact gate
commands, exits, counts and private log paths.

## Contract boundary

The source projection consumes the existing `monkey/wrapper_paths.qcore` and
`monkey/composition.qcore` declarations. Their exact selected clauses and byte
digests are bound into the sealed projection and the observed-run context.
Primitive matching of these predicates against imported C operations remains
the trusted Rust implementation; accepting contract text alone proves nothing.
The callback's borrowed, no-resource-effect declaration is checked against its
sole-return source body before constructing a callback model.

The configured entry point is `quirin-audit observed-wait CONFIG.json`. It reads
the pinned source/profile inputs, source compilation database, contracts,
Libevent build configuration, native run manifest and exact observation/binding
payloads. It constructs a fresh context from these configured inputs and checks
the sealed projection, identity-bearing observation and complete event model.
The result is `observed_composition_checked`, not a universal `Safe` verdict.
The normal `quirin-audit CONFIG.json` route retains its existing behavior.

## Conditional public join

`check_observed_wait_composition` is a conditional replay boundary. Its caller
must prepare the artifact identities; a matching context hash only identifies
that caller-prepared context and does not attest the compiler, observer,
libraries, linkage, host or run manifest. The join requires exact `pthreads`,
`producer_supplied_runtime_evidence`, and
`producer_supplied_selected_artifact_linkage` assumptions plus a syntactically
valid `native_run_manifest_sha256`. It then checks source/context equality,
decodes the exact trace and replays the event lifecycle. The serialized result
labels its claim `caller_prepared_artifact_identity_conditional_replay` and
names its context digest `caller_prepared_context_sha256`.

The configured audit remains the evidence-producing caller: it hashes and
snapshots every selected file, verifies the run manifest against those files,
and rereads them after replay. The generic verification API deliberately does
not reproduce that filesystem provenance and accepts syntactically valid
caller-prepared compile, build, observer and library identities as premises.

## Explicit trust and formal limits

The native producer supplies runtime observations and the selected artifacts'
linkage evidence. Hash comparisons bind those artifacts; they do not attest
the producer, host, executable execution or link process. The native harness
wraps Libevent calls and the original callback, records a fixed dispatcher-local
buffer, and flushes after teardown. Its activation observation is inferred from
the timeout flags at callback entry. Generation 1 describes a single lifetime,
not a general pointer-reuse detector. Fresh model names preserve the recorded
full-width native identities in separate singleton namespaces.

Interposed `event_new` registers the observer trampoline and dispatcher buffer
with Libevent; the trampoline forwards delivery to the original callback and
captured source argument. The trace records those logical source identities,
not the literal callback/argument pair stored inside the instrumented Libevent
event. Instrumentation transparency remains a premise; this is not an
uninstrumented production-execution certificate.

Lean modules model source obligations, context equality, exact identity-bearing
records, singleton identity renaming and the selected legal lifecycle. Native
truth, Clang import, cryptographic hashing and Rust/C/Lean implementation
refinement remain external boundaries. No theorem here establishes all-path
safety of the Monkey translation unit.

## Executed integration evidence

The fresh Linux x86_64 observer ran against the pinned Libevent source at
`d82464a277d0f42703702c4dfd9af6af38595a83`, compiling both the observer and
`libevent_core.a` with Clang address/undefined sanitizers and running with leak
detection enabled. [Native command and exit](native-run.json),
[artifact identities](native/run-manifest.json), [complete records](native/observations.jsonl)
and [bindings](native/bindings.json) are retained. Thread support is configured
in the build, but this harness neither links the pthread adapter archive nor
calls `evthread_use_pthreads`; the checked model is strictly single-dispatcher.
This run establishes no cross-thread safety or whole-Libevent sanitizer matrix.

The [configured audit](observed-config.json) produced a successful
[bounded composition report](observed-report.json), checking eleven records
and eleven event operations with the original full-width native identifiers
retained in its explicit mapping. The exact configured paths are host-local;
native objects, executable and build logs remain under
`private/cycle18-observed-integration/native/`.

The shipped [negative-control runner](check_negative_controls.py) changes
callback generation, removes a record, invents synchronization, substitutes
the backend, changes callback borrowing to ownership, and substitutes an
artifact digest. Payload hashes are recomputed so semantic mutants reach the
semantic gate. [All six controls](negative-controls.json) exited 1 and emitted
no composition report; each individual log is archived beside that record.
The Rust join test additionally covers source-summary/contract context swaps,
wrong bounds, changed raw bytes and an identity greater than `u32::MAX`.

Strict compile ingestion gained the exact native flags `-ffunction-sections`,
`-fdata-sections` and `-fsanitize=address,undefined`, transported to Clang.
A focused test checks both sanitizer preprocessor features. Paired `-MMD -MF`
dependency output switches remain in compile-task identity but are omitted
from the analysis invocation, like object output switches, so auditing cannot
overwrite retained dependency files; missing/duplicate pairs refuse. Other
sanitizer spellings still refuse. The first end-to-end attempt refused `-MMD`
before that explicit support was added; the second exited 0. An earlier WIP
compile failed before the agent modules were exported; neither failure is
counted as a passing gate.

## Review corrections

Review found that operation counts alone allowed same-count operand, field and
guard substitutions. The source projection now also requires a complete
canonical typed-summary digest vector matching one reviewed fixture or the
pinned production artifact, without mixing the two sets. This is an explicitly
bounded artifact recognizer; it is not general symbolic discharge of arbitrary
implementations satisfying the same contract. New descriptor, field and guard
mutations exercise refusal. The actual native configuration preprocesses to
the reviewed production vector.

The [initial combined gate](initial-gate-facts.json) measured `b1ad525a`:
4,953 Rust passed, 0 failed, 11 ignored; 262 summaries, 251 running targets
and nine doc-test sections; 156 Lean audits. Every recorded command exited 0.
It preceded the complete-summary correction and final public-boundary and
output-preservation review fixes, so the final corrected-code gate is recorded
separately. The isolated admission card also discloses an exact-tip workspace
run with one host-sensitive fork/Broken pipe failure and its isolated 12/12
rerun; that rerun does not change the full attempt's exit 101.
## Integrator reconciliation list

- Retain the configured audit's independent file hashing, run-manifest checks
  and post-replay snapshots; the public replay result cannot replace them.
- Treat `caller_prepared_context_sha256` as identity equality only.
- Keep unobserved allocation and registration failures `Unknown` and make no
  callback-denominator or Cycle 18 aggregate promotion from this single run.


## Final review boundary

The public replay checker now requires the exact thread-configuration,
observation-trust and linkage-trust assumptions plus a valid manifest digest.
Its serialized claim is `caller_prepared_artifact_identity_conditional_replay`;
`caller_prepared_context_sha256` identifies caller-prepared premises. It cannot
independently authenticate the named build or observer artifacts. The configured
CLI performs the file loading, hashing, manifest comparisons and post-replay
snapshot checks. Tests use synthetic artifact digests only as explicit premises
of the library-level conditional replay.

Report destinations are checked against the configuration, selected artifacts,
contracts, observations, source inventories and protected profile/source/build
directories. Exact and symbolic aliases, including dangling symbolic targets,
refuse; Unix file identities additionally refuse hard links. Three focused
unit tests cover distinct output, direct/directory aliases and link aliases.
This prevents configured output from overwriting retained inputs; it is not a
claim against adversarial concurrent filesystem mutation. Hard-link identity
checks are Unix-specific.

The [final configured audit](final-observed-audit.json) at corrected semantic
tip `7b146cc7` exited 0. The six shipped negative controls were rerun against
that binary and all exited 1 without a report. These are current-code results;
the earlier attempts remain in private logs.

## Worktree preservation and reconciliation

The [worktree sweep](worktree-sweep.json) removed the three completed, clean,
merged agent checkouts, preserving 504 private measurement entries in a
separate archive with before/after SHA-256 checks and old-path links. Build
caches were disposable. The older `next-wave-candidates-c-sources-18bd37`
checkout remains because its external Claude and shell sessions were live.

The inclusive-base [reconciliation record](reconciliation.json) accounts for
26 items, including the preceding close's fifteen items by reference. No
frozen record is rewritten. The [residual scan](residual-sweep.json) returns
57 candidates from historical increments and standing rules, with no untracked
candidate introduced by these four Cycle 18 reports; this is not a claim that
all historical candidates are discharged. The five new capability follow-ups
are registered in the acceptance matrix, and none is retired here.


## Corpus preservation

The release audit was rebuilt and rerun at corrected semantic tip `7b146cc7`.
The [run record](corpus-run.json) records exit 0, exact command, environment,
binary digest and private log directory. [All five artifact hashes](corpus-identity.json)
match the frozen W42 baseline byte-for-byte. The repository's
[artifact itemizer](corpus-diff.json) also reports no additions, removals or
changes, including the contract module inventory and selected declarations;
raw equality confirms that normalized module names conceal no difference.
The earlier `b1ad525a` comparison is retained separately as
[initial run](initial-corpus-run.json) and [initial hashes](initial-corpus-identity.json).
No hardcoded compatibility route is deleted, and no unit/function/CT or
callback-coverage result is promoted.


## Final standing gate

The [final gate](gate.json) measures corrected semantic tip
`7b146cc7ac77678423dc7e98ae83ac7aefd31f54`. Later changes are evidence,
reconciliation, generated matrix and status documentation only.
[Exact commands, exits, counts and private logs](gate-facts.json) record:

| Check | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 |
| `cargo test --workspace --no-fail-fast` | 4,957 passed, 0 failed, 11 ignored; 262 summaries; exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py --jobs 2` | 156 audits; exit 0 |
| Five named host-sensitive isolated tests | each exit 0 |
| Matrix regeneration with this gate | exit 0 |
| `python3 -m unittest discover -s scripts/tests` | 317 tests; exit 0 |

There are 251 Running targets plus nine doc-test sections; summary counts and
target counts are distinct. Python ran after derived matrix regeneration;
[its record](python-tests.json) identifies that run. Final regeneration records
the measured Python count and completed gate. The [Clang recording](clang-version.txt)
was made in this measuring checkout, exit 0. The provider count of 122 is carried
Unix evidence, not remeasured. All final logs remain under
`private/cycle18-observed-integration/final-gate-20260920/`.

The isolated observer's first sandbox workspace attempt hit LeakSanitizer's
ptrace limitation (4,834 passed / 106 failed, exit 101); its complete rerun
outside ptrace passed 4,940 / 0 / 10. The admission card's separately disclosed
fork/Broken pipe failure and projection card's cancelled obsolete runs retain
their original status. Neither an isolated retry nor this final green gate
rewrites those attempts. No published merge figure substitutes an isolated
card's gate for the combined result.
