# Cycle status and recorded evidence

[Development table of contents](../../DEVELOPMENT.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
## Project status

Development proceeds in bounded, individually accepted **cycles**; every
record is archived under [`docs/cycles/`](../cycles).

**Complete (Cycles 1–17).** The sequential language end to end — dimension
and quantitative-axis declarations, interval-graded usage demand, ownership,
resource-flow bounds, path-sensitive typed errors, sealed error-region
cleanup, CFG lowering to the tagged-error C11 ABI, an executable small-step
model — plus the structural front end, the whole-program checker, the
multi-file driver, translation certificates, a restricted concurrency
checker (access authority, happens-before, mutex capabilities, lock order,
destruction custody), advanced control (cancellation, fallible LIFO cleanup,
double errors, linear capture), and a combined closure certificate.
[Cycle 17](../cycles/CYCLE_17.md) added principal product inference, the
fail-closed unrestricted-C11 gateway, explicit weak-memory candidate
executions, and a native threaded custody backend, with recorded MSVC and
AddressSanitizer runs.

**Bounded and recorded (Cycle 18).** A fail-closed
[Libevent event-reactor checker](../cycles/CYCLE_18.md): generation-safe
lifecycle and custody tracking, explicit callback authority, priority
schedule exploration, cross-context conflict checks, and source pinning,
with Lean theorems and
[multi-platform build evidence](../cycles/CYCLE_18_LIBEVENT_VALIDATION.md).
Sanitizer and deployed observation-hook evidence remain pending.

**In progress (Cycle 19).** The gateway applied to a
[pinned Fluent Bit corpus](../cycles/CYCLE_19.md) (CMetrics, CTraces,
librdkafka, ChunkIO, the input-chunk wrapper, Monkey): resource-function
contracts, library custody mechanisms (refcount/list/queue/arena),
a bounded chunk-lifecycle model, and Monkey/Libevent callback composition
with recorded native observations. **Verdicts stay honest**: raw translation
units are not promoted to `Safe` until per-path identity imports and real
observation evidence satisfy the cycle's fail-closed gates — see the
[library audit record](../cycles/CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md)
and the archived [evidence reports](../reports).

The [latest teardown observations](../reports/cycle19-native-teardown/README.md)
run the pinned native constructors and complete context → stream → chunk →
backend memory teardown under ASan, UBSan and leak checking. Five executions
bind 82 allocation generations and 13 chunks to reciprocal rings and exact
release order. The source chain also preserves unresolved OS custody through
parent release in 26 of 43 conditional mixed-backend cases. All 407 workspace
tests and the pinned audit pass; aggregate verdicts are unchanged. A further
[nine allocation failures and control](../reports/cycle19-constructor-failures/README.md)
account for 55 generations across rollback and teardown, preserving null-field
successes and zero error outputs on backend failure. General caller/failure
provenance, filesystem guarantees, aggregate custody, remaining wrapper lifecycle
and full event composition remain open. The
[unsupported backend close classification](../reports/cycle19-unsupported-close/README.md)
checks all other integer backend kinds symbolically: outer chunk release
preserves unresolved backend custody, so the result remains `Unknown`.
[Conditional parent teardown](../reports/cycle19-nested-unsupported/README.md)
retains those exact opaque identities through stream and context release.
[Four native unsupported-type constructor cases](../reports/cycle19-unsupported-constructors/README.md)
return null chunks and release all 24 allocation generations through rollback
and surviving-stream teardown. General caller reachability remains open.
[Twelve real Linux cleanup observations](../reports/cycle19-file-os/README.md)
now bind close, unmap and deletion to OS endpoint probes and inode witnesses in
exclusive temporary fixtures. A failed zero-length unmap retains its mapping.
These helper executions do not establish arbitrary filesystem teardown or
durability. [Six enclosing native filesystem runs](../reports/cycle19-file-tree/README.md)
add constructor, ring, release-order and OS endpoint evidence for one-file trees.
All 66 allocation generations are released. Injected unmap/close failures retain
live OS resources after parent release, and sync failure still allows cleanup.
[Eleven filesystem allocation failures and a control](../reports/cycle19-file-constructor-failures/README.md)
now account for 81 acquired generations and 73 recorded releases. Failed root
and chunk-name copies trigger two observed UBSan null-argument terminations;
eight generations remain outstanding in those interrupted prefixes. Ten returning
cases complete leak checking. These observations do not establish general caller
or failure guarantees, and no raw unit is promoted.
[Eight constructor OS-failure cases](../reports/cycle19-file-os-constructors/README.md)
add 88 fully released allocation generations. A suppressed rollback close retains
the original descriptor/inode after null chunk return and parent release; harness
reclamation is recorded separately. File-size fallback and POSIX allocation
return/errno conventions remain distinct, with general OS guarantees still open.
[Seven existing-file profiles](../reports/cycle19-file-existing/README.md)
release all 70 allocation generations. Valid and read-only content, malformed
magic, short headers and read-only empty files retain distinct outcomes and their
original inode/bytes. Suppressed chmod plus rollback-close failures preserve a
descriptor residual after parent release. General caller and lifecycle gates
remain open.
[Five native transition sequences](../reports/cycle19-file-transitions/README.md)
release all 50 generations and seven descriptor/mapping epochs. Logical rollback
leaves the mapped header length unchanged, so down/up reloads appended bytes.
Locked begin retries while direct write succeeds. These bounded observations
preserve the native behavior without claiming production reachability or general
transaction/lock guarantees.

[Three sync-failure sequences](../reports/cycle19-file-sync-failures/README.md)
release 30 more generations and four OS epochs. Failed commit preserves the
transaction and retry clears it. Down continues cleanup after failed sync;
commit while unmapped succeeds without syncing. These fixed injected observations
do not establish general OS failure behavior or durability.

[Four failed-down sequences](../reports/cycle19-file-unmap-failures/README.md)
release 40 more generations and seven OS epochs. Failed unmap retains the
original mapping, descriptor, up ring and counter; preceding successful sync
remains visible. Retry or parent teardown then releases that original epoch.
[Four descriptor-failure sequences](../reports/cycle19-file-descriptor-failures/README.md)
release 40 further generations and acquire five OS epochs. Failed close moves
the chunk down but retains its descriptor; down/up retries reject. A second
suppressed close leaves that descriptor live after parent release, with harness
reclamation recorded separately. Failed reopen creates no epoch; a successful
retry creates a fresh one. General reopen failures, production caller provenance,
aggregate custody and full event composition remain open.

**Active (Cycle 20).** Extraction of library-specific verification into a
[versioned verifier-plugin ecosystem](../cycles/CYCLE_20.md): a
length-delimited JSON protocol, manifests, a fail-closed registry, bounded
process providers, and a certificate firewall. Protocol v1 carries only
Boolean assertions, so the core currently refuses all v1 `Safe` proposals
and treats unchecked `Unsafe` proposals as `Unknown`; source-level
`verify scope` declarations now derive obligations and dispatch providers.
The [bounded evidence-v2 increment](../reports/w3-evidence-v2/README.md)
replays observation predicates against pinned contracts and records agreement
for supplied runs under an explicit observation-trust assumption. A standalone
provider builds against `qcore-schemas` alone. Source lifecycle effects remain
unimported, empty inventories remain unverified, and first-party providers
remain unpromoted. The [generic verifier host](../reports/w3-verifier-host/README.md)
is extracted into `qcore-verifier-host`, buildable without provider crates;
production profile import and remaining kernel consolidation are still open.

**Trust and audits.** An independent
[reevaluation](../REEVALUATION_SPEC.md) (2026-09-05) reproduced six
certificate/weak-memory discrepancies, followed by a
[seven-finding follow-up](../REEVALUATION_FOLLOWUP.md); all thirteen are
repaired, with the remaining boundaries (production semantic-witness imports, OS-level
provider sandboxing, and the custody modes W4 still does not infer) documented
rather than papered over.

**Direction.** The [generalization plan](../GENERALIZATION_PLAN.md)
reviews the architecture and orders the work to (a) make the checker generic
over source sets — contracts and profiles as data, core-derived obligations,
interprocedural summary inference — and (b) decompose the monolithic crate
into a pluggable contract/verification system with a versioned schema
surface and out-of-process providers. W0 is executed
([record](../reports/w0-schema-crate/README.md)): the schema surface and
the CLI are standalone crates, with a test guarding the downward-only
dependency rule; providers stay compile-time crates for now (decision
recorded in the W0 report). The W1 relocation is executed
([record](../reports/w1-contract-crate/README.md)): the contract loader
and observation interpreter live in `crates/qcore-contract`, above the new
`qcore-core` kernel seed and `qcore-frontend` front-end crates. The W1
literal-site migration is executed
([record](../reports/w1-literal-sites/README.md)): the remaining Rust
contract-table literals in tests and examples now load `.qcore` fixture
modules through the loader. W1 is executed
([record](../reports/w1-clause-vocabulary/README.md)): the clause-family
boundary between the loader and the observation/lifecycle interpreters is
documented and pinned corpus-wide, and the C ABI scalar table is complete;
unifying the interpreters is a W5-adjacent item and error sentinels need
target-interface work. The
W6 CT conformance harness is stood up
([record](../reports/w6-ct-harness/README.md)): `compiler/tests/ct_corpus.rs`
executes the manifest-mapped spec §55 cases through the real source pipeline;
CT02/CT12/CT16 initially mapped sequential-core fixtures. Batch 2
([record](../reports/w6-ct-batch-2/README.md)) adds CT19 and CT37 and
records the full triage of all 47 then-unmapped cases. The later
[native task increment](../reports/w6-native-source-tasks/README.md) maps CT15
and upgrades CT19 to actual task source; [explicit recovery](../reports/w6-task-recovery/README.md)
adds CT13. [Owned-task mappings](../reports/w6-owned-source-tasks/README.md)
and [handler rethrow](../reports/w6-handler-rethrow/README.md) now leave
39 of 50 unmapped after the [reused-address mapping](../reports/w6-reused-address/README.md). [Worker allocation](../reports/w6-worker-allocation/README.md)
upgrades CT16 to real child allocation/error cleanup and retains its sequential
implicit-cleanup control. W4 is
executed for parameter/return custody with validation
([record](../reports/w4-custody-inference/README.md)): the C gateway now
infers each internal function's parameter and return custody summary bottom-up
over the call graph and publishes it only after re-running the intraprocedural
analyzer against it, so a wrapper around a contracted API is analyzed instead
of being `Unknown`. Declared contracts always take precedence, a disagreement
is reported rather than resolved silently, and the safety certificate's recheck
re-derives every summary independently. The [bounded W4 extension](../reports/w4-custody-extensions/README.md)
adds direct out-parameters, stores of formal resources into formal owners,
reallocation wrappers and possibly-null owned returns. Recursive functions,
aggregate removal and unsupported out-slot aliases keep their callers `Unknown`.
The [gateway relocation](../reports/w4-gateway-crate/README.md) extracts this
analysis, Clang discovery and typed-AST transport into `qcore-c-gateway`, retaining
the compiler API and exact 185-unit diagnostic/verdict/projection parity.

Further reading: [module modes and index syntax](../QCORE_MODULE_MODES.md),
[QTT alignment](../QTT_ALIGNMENT.md),
[post-Cycle-9 roadmap](../ROADMAP_AFTER_CYCLE_9.md).

<!-- migrated-section:end -->
