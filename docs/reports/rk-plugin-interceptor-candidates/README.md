# RK-PLUGIN-INTERCEPTOR-CANDIDATES

Base `1543259f0e32c8a003a9062e7de7f59a42dd4519`. [Prediction](PREDICTION.md)
was committed at `d623c3670`, with the runtime-only representation amendment at
`78c6af225`, before implementation. This report will record the completed
measurements and gates; the card is still in progress.

The candidate inventory distinguishes 14 `rd_kafka_conf_s.interceptors` lists,
their 14 `rd_kafka_s.rk_conf` aliases and their method union arms. It pins exact
source bytes at both audited Fluent Bit revisions and preserves the C signatures.
The candidate restriction is neutral custody for callbacks and their transitive
calls, with borrowed pointer arguments and scalar resource handles, balanced local
acquisitions, public registration lifecycle restrictions, and callback code/opaque
contexts valid through their last possible invocation. It is a human review
premise for deployed code; it does not declare the setters neutral, establish a
static list closure or decide an unsupported static invocation.

The generic runtime loader binds the inventory's exact digest into each defining
outer statement, refuses unsupported/missing/unbound inventories, and holds the
citing loader on source mismatch. Both outer envelopes add the 28 exact list paths
only under those review restrictions. Static rule 12n remains unchanged.

## Source and registration reading

The [inventory evidence](inventory-evidence.json), reproduced by
`python3 docs/reports/rk-plugin-interceptor-candidates/inventory_evidence.py`,
checks all callback arms and six exact source files at each audited revision:
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and
`046accf6aded7906f66a1baf0202da349e521736`. These six files have identical
bytes at the two revisions. `rdkafka_interceptor.c` defines the method record,
stores its callback and opaque context, checks duplicate registrations and
appends only a successful registration. `rdkafka_conf.h` identifies the lists;
`rdkafka_interceptor.h` and `rdkafka.h` supply the member-specific signatures,
public API and lifecycle rules; `rdlist.h`/`rdlist.c` supply the element-list
representation. The separate `interceptors.config` list holds `rd_strtup_t`
configuration values and has no callback candidate.

The configuration setters register `on_conf_set`, `on_conf_dup`,
`on_conf_destroy` and `on_new`. The other ten setters register `on_destroy`,
`on_send`, `on_acknowledgement`, `on_consume`, `on_commit`, `on_request_sent`,
`on_response_received`, `on_thread_start`, `on_thread_exit` and
`on_broker_state_change`; they require the documented on-new phase, an
uninitialized client and a non-share client. The successful result is zero.
Only `on_conf_destroy` permits duplicate names; a duplicate in the other lists
returns a conflict without appending a wrapper. All retained callback code and
opaque contexts must outlive their last possible invocation, including
configuration duplication and destruction.

These restrictions are intentionally narrow. A callback that frees or retains
opaque state, transfers ownership, or introduces unmatched registration custody
effects fails the neutral-custody premise. Public setter bodies allocate and
retain method wrappers, and this inventory does not assert neutral resource
contracts for those setters. Declaring a candidate review class does not assert
that an installed plugin belongs to it.

## Representation and evidence

The version-one declarative inventory is
[outer-candidates.json](../../../providers/qcore-provider-librdkafka/outer-candidates.json),
SHA-256 `5c3f8c4723832c0e9e6ebb784be6a57ebc0264d519421d5b95a18a8f68c18147`.
Its exact digest appears in both defining human-review statements. The generic
loader rejects unsupported schema/modes, ambiguous list identities, absent or
unbound inventories and unsafe source paths; mismatched reviewed bytes hold the
citing outer loader. Runtime metadata includes the full inventory and source-pin
standing. The inventory is host load-path metadata beside the outer lock;
the executable provider protocol is unchanged. Loading the inventory does not register a static callback type or
candidate closure. Both defining outer modules were deliberately re-pinned at
both source revisions; their clause expectations change from 57 to 85 while
the proxy citation remains three clauses and the resource module count remains
136.

Five Rust controls cover exact distinct list identities, unsupported modes and
duplicates, missing/unbound/stale references, source-byte/path checks and the
actual paired envelopes. `QCore.OuterListCandidates` and its registered audit
model source/digest standing, successful registration, lifecycle, neutral
custody and callback/context lifetime. They prove review-only Pending/Unknown
outcomes never manufacture Safe, and static selection is unchanged. The
connection to extracted C identities and reviewed human premises is explicit;
this is a model proof, not verification of arbitrary plugin bodies.

## Measurement provenance

The baseline is the existing primary integration observation at actual source
`ad526b5871c8cd05596c63d110932923431915d7`, reused rather than rerun.
[Provenance](baseline-provenance.json) checks that implementation, provider
contracts, audit inputs, scripts and proofs are identical between that source
and the declared base `1543259f0e32c8a003a9062e7de7f59a42dd4519`.
The W44 cross-unit map remains an explicit linked-program resolution,
compatible-type/calling-convention and interposition premise; its SHA-256 is
`ddd0576cde87a5782f1fe0b9c4ce4efad4042b1ee54a483a471a3576f0698e7e`.
The corpus uses source 66910c10, and the configured system and tail-pause slice
use source 046accf6. Runtime envelopes require review of the deployed plugin
class. These are separate assumptions and do not certify a deployment.

The 32 baseline runtime custody Pending rows were read individually. Their
current reasons are existing cJSON, librdkafka callback/queue premises or
ordinary callback run-set/stated facts; none carries an interceptor-list
run-set fact. They motivate review of the outer class but are not a promised
promotion target for this runtime-only inventory.

Fresh tip measurements are still pending. The
reproducible commands are [measure.sh](measure.sh), [gate.sh](gate.sh),
[compare.py](compare.py), [summarize.py](summarize.py) and
[write_gate.py](write_gate.py). Worktree-private outputs live under
`private/rk-interceptor-candidates/`.

## Validation

The full root standing suite passed 8,235 tests in 513 units / 526 summaries,
with zero failures and 15 ignored. All 338 Lean audits passed without project
axioms or placeholders. Root formatting, Clippy, both reevaluation examples and
Clang presence checks passed. The provider preflight also passed locked offline
check, formatting, Clippy, contract sync and build/verification of all five
packages. Root and provider compilation use the card-private target
`/home/cosmo/Documents/GitHub/qcore-c/private/rk-interceptor-candidates-target`.
[Clang observation](clang-version.txt) is from this worktree's final-source gate.
Provider Rust passed 133 tests and provider Python passed 25; the script suite
passed 374 tests and the generated documentation index check passed. Every
required final gate step exited zero; [gate.json](gate.json) records the counts
and exact implementation commit. The derived acceptance matrix was regenerated
with these gate observations.

## Gate diagnostics retained

The first full Rust attempt exited 101 while compiling because the shared
repository filesystem had no available space; no test had run. Its exact log
and capture directory remain under `gate-standing-final/tests-disk-attempt1.*`
and `test-binaries-disk-attempt1`. A peer removed only its own completed debug
cache; this card briefly relocated its own cache to `/tmp` and restored it to
its dedicated target before retrying. The retry explicitly uses
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`;
assertions, test behavior and optimization level remain unchanged.

Earlier setup diagnostics are retained separately: sandbox-only Git metadata
writes required narrow native escalation; an invalid task-context topic was
corrected; contract sync initially found stale outer locks; one pin batch read
the sync operation's temporary empty outer view and was rerun sequentially.
The completed final-source preflights and final source-pin checks supersede
those attempts without relabeling their nonzero exits as success. The first
matrix regeneration also rejected an unsupported source-commit key in the gate
metadata before changing the matrix; source identity was moved into the
supported note field and regeneration then passed.

## Integrator amendments and residuals

Gateway rule 31 and contract specification C04.20.1 describe the runtime-only
inventory and exact digest binding. No static rule 12n premise is widened.
The registered `RK-PLUGIN-INTERCEPTOR-CANDIDATES` entry is narrowed to
`RK-INTERCEPTOR-STATIC-LIST-CLOSURE` in the acceptance matrix (797 entries
remain); this preserves the outstanding source-derived list closure instead of
claiming the runtime review surface discharges it.

## Follow-ons

`RK-INTERCEPTOR-STATIC-LIST-CLOSURE`: static interceptor list invocation remains unsupported: a list/element/union-arm
closure and candidate composition need their own sound rule. Callbacks that free
or retain opaque state or transfer ownership do not meet this card's neutral
candidate and remain unsupported. No deployed callback has been certified.
