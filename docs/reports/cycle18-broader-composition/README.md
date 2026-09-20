# Cycle 18 broader callback composition

Base: `229578d8`. Status: bounded aggregate and one caller-size capability implemented; universal Cycle 18 callback acceptance remains `Unknown`.

## Result

This card implements the composition boundary for the three Monkey callbacks
without promoting any callback to `Safe` or fully composed. The verifier derives
the exact source denominator from constructor-private typed-C summaries:

| Callback | Imported source paths | Rechecked bounded evidence |
|---|---:|---|
| `cb_event` | 4 | four-path payload/source/context replay |
| `cb_timeout` | 3 | one selected running-callback teardown execution |
| `cb_wait_2_timeout` | 1 | one eleven-record wait lifecycle plus the eight-case failure/source-model inventory |
| **Total** | **8** | **three independently rechecked evidence families** |

`check_cycle18_composition` accepts raw evidence and constructor-private
projections/validated contexts, not serialized reports or caller-supplied
completion booleans. It reruns `check_observed_wait_composition`,
`check_wait_failure_paths`, `check_event_payload_boundary`, and
`check_timeout_teardown_evidence`. The aggregate compares independently derived
callback digests using the canonical encoding of each existing projection:
full `SourceSummary` for `cb_event`, and `(function, revision, paths)` for the
wait and timeout projections. The distinct encodings are explicit; relabeling
one digest as another refuses.

The wait-failure family retains its own constructor-private source projection.
Its observations are replayed against that projection, and Quirin compares
canonical digests of the complete source-summary and contract maps with the
observed-wait projection. Equal revision/profile labels cannot bridge divergent
maps. Its selected Monkey source, build and compile-task identities must also
equal the wait context's identities. The timeout family independently joins its
exact callback digest and requires the same selected Monkey source/build bytes;
its native observer and Libevent build remain separately identified.

The executable policy is
`contract/qcore/monkey/cycle18_composition.qcore`. It receives primitive source
revision, profile, library, role, path-count, source/build/compile-task, context,
and callback-digest facts. Quirin evaluates the equalities. Rust assembles sets,
rehashes bytes, invokes the existing typed checkers, and reports diagnostic
grounds. Validated wait and payload contexts may name different observer
binaries, harnesses, and native builds; each identity remains bound to its own
evidence. Their selected Fluent Bit revision, Monkey profile, Libevent revision,
and Libevent profile must agree. Timeout and failure-harness identities are
independently prepared inputs whose canonical digests and exact revision/profile
joins are checked and published as declared premises, not execution attestation.

The returned type has only `Cycle18AggregateVerdict::Unknown`. A failed evidence
join remains an explicit refused family inside an `Unknown` report. Invalid or
incomplete callback source inventory issues no eight-path aggregate at all.
The denominator therefore cannot shrink when evidence is missing.

## Concrete source capability

`check_cycle18_caller_size_bound` derives every retained
`mk_event_loop_create` call argument from one sealed caller `SourceSummary`.
It accepts only integer literals, requires a positive value within signed
`int`, performs checked multiplication by the freshly rechecked payload element
size, and asks Quirin to verify the exact multiplication and source
revision/profile equalities. The result records the caller summary digest,
all capacities, the element size, and maximum allocation bytes.

This is a caller-local proof. The configured caller inventory must still be
shown complete before the aggregate `caller_size_bounds` residual can be
retired. Nonliteral sizes, zero, identity substitution, empty call sets and
overflow refuse. The focused positive control checks size 256 with a 72-byte
element, deriving 18,432 bytes. This capability makes exact literal production
callers dischargeable without pretending that the production call graph has
already been enumerated.

## Source findings and remaining discharge conditions

The pinned Libevent adapter does not supply the missing universal facts:
`_mk_event_loop_create` does not guard a null `event_base_new` result, its fired
allocation multiplies a signed caller size, and `cb_event` increments
`fired_count` without a local capacity guard. Those are source obligations, not
facts a native success run can add. The aggregate retains seven named
conditions:

1. `initialization_reachability`: prove the opaque `event_init` branch
   unreachable or check it under the selected production profile.
2. `caller_null_handling`: compose every production caller with null loop and
   context returns.
3. `caller_size_bounds`: enumerate the production callers and apply the exact
   caller-size proof to each one.
4. `allocation_authority_provenance`: derive storage extent and exclusive
   callback write authority without a caller declaration.
5. `registration_liveness`: establish behavior after timer-registration
   failure against the selected backend.
6. `natural_activation_reachability`: observe or prove production readiness
   and timeout activation rather than controlled activation.
7. `all_path_execution_coverage`: compose both `cb_timeout` teardown paths and
   its non-teardown success path. One selected teardown execution cannot cover
   all three source paths.

The three source-preserving failure cases remain provisional `Unsafe` evidence,
never upstream defect claims: base creation failure, fired allocation failure,
and timer creation failure. The aggregate does not erase them when its three
bounded evidence families join.

## Lean correspondence

`proof/QCore/Cycle18Composition.lean` models exact 4/3/1 inventory, revision,
profile, source/build and evidence joins, the invariant eight-path denominator,
source-digest substitution refusal, always-Unknown publication, and exact
caller-size allocation extent. `proof/AuditCycle18Composition.lean` audits eight
theorems. The Lean model abstracts checked strings and digests as positive
naturals. Rust performs concrete parsing, canonical SHA-256 checks, source
projection and evidence replay. Their correspondence is maintained by review;
there is no extraction or refinement theorem.

## Focused validation

Commands use `CARGO_BUILD_JOBS=2`. Private logs are under
`private/cycle18-composition/` in the card worktree.

- `cargo test -p qcore-verification --lib cycle18_composition -- --nocapture`:
  5 passed, 0 failed. The cases cover exact 3-callback/8-path source inventory,
  literal caller extent, nonliteral/zero/identity refusal, and policy refusal of
  `Safe` or a shortened obligation list. A fixture-derived control also creates
  two constructor-private wait projections with the same revision/profile and
  source-summary map but different contract maps; the cross-family join refuses.
- `lake build QCore.Cycle18Composition`: 2 jobs passed.
- `lake env lean AuditCycle18Composition.lean`: eight theorem audits emitted;
  no project axiom or placeholder was added.

The root integrator owns the complete workspace standing gate and the real
configured aggregate replay. This lane does not claim those runs.

## Publication review correction (2026-09-21)

The final configured consumer now fails closed before writing its aggregate
report unless `joined_evidence_families` is exactly `{cb_event, cb_timeout,
cb_wait_2_timeout}` and `refused_evidence_families` is empty. The successful
bounded result remains `Unknown`; this publication guard does not promote a
callback or the aggregate.

The aggregate output is also compared with all four prepared leaf-report
destinations using the same symbolic-path normalization and existing-file
identity checks as the evidence-output guard. This rejects a shared planned
path before either report exists and rejects existing hard-link aliases. Each
prepared leaf now retains the already verified Fluent Bit revision, and the
three Libevent-backed leaves also retain their already verified Libevent
revision. Publication re-runs `git rev-parse --verify HEAD^{commit}` against
those retained expectations after all four sequential preparations, so a
checkout commit change refuses even when every selected file byte is unchanged.

Focused correction validation used the private target at
`/home/cosmo/Documents/GitHub/qcore-c/private/cycle18-broader/publication-target`
and logs under
`/home/cosmo/Documents/GitHub/qcore-c/private/cycle18-broader/publication-review/`:

- `cargo test -p qcore-audit --no-fail-fast`: exit 0; 91 passed, 0 failed,
  0 ignored across unit and integration targets. The new controls cover a
  partial join, a nonempty refusal map, planned and hard-linked leaf-report
  aliases, and a checkout revision change with unchanged selected bytes.
- `cargo clippy -p qcore-audit --all-targets -- -D warnings`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- `git diff --check`: exit 0.

The complete workspace standing gate and configured aggregate replay remain
pending on the root integration tree after all publication lanes and matrix
regeneration are merged.

## Integrator reconciliation list

- Wire the configured consumer by preparing the three source projections and
  validated contexts from immutable configured inputs, then pass the raw
  observation/binding/allocation/failure/timeout evidence to the aggregate.
  Never deserialize a prior leaf report as aggregate authority.
- Publish the configured aggregate report only after all three bounded evidence
  families join. A family refusal stays visible and does not reduce the
  eight-path denominator.
- Supply wait-failure and timeout source/build/compile-task identities from the
  configured file snapshots. They are declared provenance premises, not
  attestation.
- Retain the wait-failure projection separately from the observed-wait
  projection and pass both into the aggregate. Do not replay one family's
  observations against the other family's source token.
- Regenerate the acceptance matrix so its derived Lean audit list/count includes
  `AuditCycle18Composition.lean`; do not hand-edit the derived section.
- Update the Monkey contract corpus count/pins for
  `cycle18_composition.qcore` in the root integration commit.
- Keep callback accounting at 3/3 accounted, 0/3 fully composed, and three
  explicit `Unknown` rows. `MONKEY-CALLBACK-CYCLE18-COMPOSITION` is not retired
  until the seven discharge conditions above are actually proved or observed
  at their stated scope.
- Preserve the event-payload result as a bounded property report. It is not an
  event-lifecycle certificate, and its controlled activation is not natural
  activation evidence.
- Preserve the selected timeout execution as one checked path. Do not credit it
  to either unobserved timeout path.
