# Cycle 18 executable contract extraction

Base: `c4d3f1873fbc937e36d47af4dd23d199d732cd91`.
Status: complete; corrected combined gate and parity checks passed.

The user requested that source and identity checks remaining in Rust be
extracted into Quirin contracts. Three GPT-5.6 Sol agents at medium reasoning
implement a generic typed-fact predicate interpreter, executable timed-wait
source policy, and executable context/identity policy. The integrator owns
configured audit wiring, contract-corpus pins, shared exports, parity evidence,
the combined gate, matrix/status records and publication.

Expected values and acceptance relationships move into consumed `.qcore`
predicates. Rust retains source traversal, fact extraction, hashing, parsing and
the operational event-model interpreter. Facts do not carry proof authority:
the configured caller must independently prepare the artifacts and compare
with the admitted context. A contract supplied by an observation producer
cannot replace the verifier's selected policy.

The source policy preserves the complete approved summary vectors and refuses
mixed fixture/production identities and same-count source mutations. The identity
policy binds exact source, observation and context relationships. Every extracted
policy is tested for changes that affect acceptance; a declaration that is only
stored or compared as text does not complete this task.

Parity must precede retiring the corresponding hardcoded checks. The previous
CLI executable and focused test logs are retained under
`private/cycle18-contract-extraction/baseline/`. Ordinary corpus, callback and
CT classifications remain unchanged. A successful bounded observed run is
still conditional replay, never an all-path `Safe` claim or Cycle 18 closure.

## Integrated changes

The source policy has 13 declarations and 102 clauses; the identity policy has
3 declarations and 29 clauses. The typed interpreter accepts bounded integer
and decoded text facts. Original wrapper/composition contract bytes are
unchanged. The global module pin deliberately grows from 88 to 90; the resource
manifest remains unchanged at 33 selections and 59 resource-v1 declarations.

The configured audit now carries the decoded semantic profile and derives
bounds from actual trace identities. It continues to describe actual producer
trust classifications rather than filling facts with the policy's desired
values. Source summaries carry the new source-policy digest; the observation
rules identity binds the identity policy's exact bytes.

Review caught and corrected two defects before the combined gate: an override
could delete its own digest-equality clause, and separate declaration checks
could populate the other half of their facts with desired values. Generic
policy admission now binds selected bytes before execution, and declaration
checks evaluate a complete actual-fact vector. Regression tests cover both.
The byte-admission check is part of the interpreter's trust boundary; it cannot
be disabled by the policy being admitted.

## Evidence boundary

The C extraction and Rust/Lean correspondence remain reviewed premises. Lean
modules prove abstract predicate and admission properties, not extraction of
the Rust verifier or correctness of arbitrary C. The instrumented runtime
payload is the prior pinned Linux epoll run; this extraction does not claim a
new native execution. Its before/after CLI replay is measured separately.

No new verdicts, callback composition coverage or CT conformance are claimed.
The three callbacks remain accounted but uncomposed and Unknown; aggregate
Cycle 18 acceptance stays open. Existing named follow-ups remain in the matrix.

## Before/after CLI parity

`python3 docs/reports/cycle18-contract-extraction/check_parity.py
private/cycle18-contract-extraction/baseline/quirin-audit target/debug/quirin-audit
private/cycle18-contract-extraction/baseline/config.json
private/cycle18-contract-extraction/cli-parity` exited 0. The baseline executable
is retained from `c4d3f187`; the extracted executable was built after semantic
tip `3c7a5410`. [Binary identities and comparison](cli-parity.json) record one
accepted run and six refusing controls per executable. Status, conditional
claim, eleven records/operations, selected paths and identity mapping agree.
The only context differences are the source policy digest/aggregate, identity
policy digest, and corresponding assumption entry. Private positive/negative
logs live under `private/cycle18-contract-extraction/cli-parity/`.

## Frozen corpus and cleanup

The release audit at semantic tip `3c7a5410` exited 0. The full source,
diagnostics, source-effects, chunk-contract and contracts artifacts are
[byte-identical](corpus-bytes.json) to the frozen W42 baseline; the
[structured diff](corpus-diff.json) and [run identity](corpus-run.json) record
commands, identities and unchanged results. Release build logs and raw output
are under `private/cycle18-contract-extraction/final-corpus/` and its parent.

[Worktree sweep](worktree-sweep.json) removes the three merged extraction
checkouts and retains the active older external Claude/shell worktree. It
preserves 45 evidence entries with old-path links under
`.claude/worktrees/.sweep-backups/cycle18-extraction-20260920T131024Z/`.
The source agent's final report announced package completion while a rerun was
still writing the same log. Removal interrupted that rerun: the retained log
contains two missing-fixture failures and a missing-source doctest failure.
Its claimed 301/0 package result is not used as successful evidence here.
The first sweep's post-removal hash assertion exposed the continued writes;
the resumed manifest records settled bytes and discloses the interruption.
The root standing gate independently remeasures every workspace target.

The source agent's isolated whole-Lean audit also exited 1 because its branch
lacked the root-owned aggregate registration for `TypedContractFacts`; the root
registers both new modules. Targeted Lean success is recorded separately from
that failed whole-Lean attempt. Earlier logs and frozen claims are retained,
not rewritten into passes.

## Initial combined attempt

`private/cycle18-contract-extraction/final-gate-20260920/` records the initial
full workspace attempt: 4,967 passed, one failed, eleven ignored; 263 result
summaries, 252 running targets and nine doc-test sections, exit 101. Its sole
failure was `contract_loader`'s stale 88-module pin. The deliberate correction
sets 90 modules and 161 non-resource declarations (145 + 16), retaining the
33 selected modules and 59 resource declarations. Fmt, all-target Clippy,
both probes, 158 Lean audits and five isolated host-sensitive rows all exited 0.
A complete corrected gate is measured separately; no failure is hidden or
converted into a pass. The first run used two build jobs and four test threads;
the corrected run uses two build jobs and eight test threads on this 16-CPU
host, with the five host-sensitive rows again run separately with one thread.

## Final standing gate

The corrected gate at `5c3ccc0913433621a8701bee501bc58f1deddb28` passes:
**4,968 Rust passed, 0 failed, 11 ignored; 263 result summaries across 252
running targets and nine doc-test sections; 158 Lean audits; 317 Python tests**.
[Gate](gate.json) and [commands, exits and run identity](gate-facts.json) retain
the measured source and every non-Python command. Private logs are under
`private/cycle18-contract-extraction/corrected-gate-20260920/`.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned submodules present |
| `cargo test --workspace --no-fail-fast` | 0 | 4,968 passed, 0 failed, 11 ignored |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | expected result |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | expected result |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0 | 158 audits |
| five isolated host-sensitive tests listed in `gate-facts.json` | 0 each | each row passed |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/cycle18-contract-extraction/gate.json` | 0 | regenerated ledger |
| `python3 -m unittest discover -s scripts/tests` | 0 | 317 tests after regeneration |

Clang 21.1.8 was recorded in the measuring checkout. The 122 provider tests
remain carried Unix evidence; provider code/schema did not change. Python
output and exits are `python-tests.log`/`.exit`; matrix outputs and exits are
`matrix.log`/`.exit` and `matrix-final.log`/`.exit` in the same private run.
The final generator passes update the measured Python note and retain a
repository-relative gate provenance path; the Python suite is repeated against
that final ledger (`python-final.log`). The 16 swept reconciliation items have explicit dispositions and
checked applied claims. The residual scan includes historical/prose candidates;
this extraction introduces no new residual or retirement. The ledger remains
at 284 named follow-ups, with callback, CT and full-conformance counts unchanged.
