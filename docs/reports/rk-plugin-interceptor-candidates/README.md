# RK-PLUGIN-INTERCEPTOR-CANDIDATES

Base `1543259f0e32c8a003a9062e7de7f59a42dd4519`. [Prediction](PREDICTION.md)
was committed at `d623c3670`, with the runtime-only representation amendment at
`78c6af225`, before implementation. The runtime candidate surface is complete:
all required gates and fresh corpus/system/slice runs passed. As predicted,
static verdicts and runtime verdicts are unchanged. Static list closure remains
the named follow-on below.

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
`inventory_evidence.py CORPUS_SOURCE SYSTEM_SOURCE SLICE_SOURCE`,
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
the executable provider protocol is unchanged. Loading the inventory does not
register a static callback type or candidate closure. Both defining outer modules were deliberately re-pinned at
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
The corpus uses source 66910c10, the configured system uses 046accf6 and
the tail-pause slice uses `34155955e4962a56d94eb23c03a002880899a0d1`.
The slice's six relevant source files exactly match the reviewed byte set;
this does not assert equality of the entire source revision. Runtime envelopes
require review of the deployed plugin class. These are separate assumptions and do not certify a deployment.

The 32 baseline runtime custody Pending rows were read individually. Their
current reasons are existing cJSON, librdkafka callback/queue premises or
ordinary callback run-set/stated facts; none carries an interceptor-list
run-set fact. They motivate review of the outer class but are not a promised
promotion target for this runtime-only inventory.

## Fresh measurements and comparison

Fresh runs at repository checkpoint
`2a609d3ace0ceb0c057b66886afa845cb8dc1176` all exited zero. The gated
implementation is `d899486b47a582707d015a070c9e83e82a99a5a4`; the intervening
changes are records and measurement helpers, with no implementation, provider,
contract, proof or audit-input difference. The binary SHA-256 is
`d70ac523d158772d350470125215ba1afb31ee417f5d2bda4c769cf5321b9f13`.
[Measurement provenance](measurement-provenance.json) retains exact source
profiles, scope statements, model versions, source revisions and artifact
hashes. The configured profile's old pin remains a historical reference; each
configured run names its actual checkout revision separately.

Every static custody status and every null/combined status is identical on all
three selections. All 5,781 complete null JSONL rows are byte-identical. The
[per-row comparison](comparison-summary.json) records no new/removed runtime
move, changed status pair or changed reason fact in any of the three scenarios
(all loaded, Fluent-Bit-only, rdkafka-only), after normalizing only the outer
contract identity hashes in reasons. The existing moved-row records remain
228 on the corpus, four on the system and 16 on the slice per scenario.
No newly moved row requires a new body reading. The same 32 corpus custody
rows remain Pending; no Pending is promoted to Safe.

The following counts are identical before and after this card. Each property
keeps its own denominator, including repeated compilation configurations and
Unknown/Pending rows. Unsafe is provisional within this model and is not an
upstream defect claim.

| Selection | Property | Rows | Static S / P / U / provisional Unsafe | Runtime S / P / U / provisional Unsafe |
|---|---|---:|---|---|
| corpus | custody | 4,834 | 955 / 0 / 3,870 / 9 | 923 / 32 / 3,870 / 9 |
| corpus | null | 4,921 | 2,549 / 0 / 2,354 / 18 | 2,386 / 163 / 2,354 / 18 |
| corpus | combined | 4,921 | 839 / 0 / 4,055 / 27 | 806 / 33 / 4,055 / 27 |
| system | custody | 448 | 97 / 0 / 347 / 4 | 97 / 0 / 347 / 4 |
| system | null | 454 | 211 / 0 / 243 / 0 | 209 / 2 / 243 / 0 |
| system | combined | 454 | 65 / 0 / 385 / 4 | 63 / 2 / 385 / 4 |
| slice | custody | 404 | 52 / 4 / 347 / 1 | 52 / 4 / 347 / 1 |
| slice | null | 406 | 190 / 0 / 216 / 0 | 174 / 16 / 216 / 0 |
| slice | combined | 406 | 31 / 2 / 372 / 1 | 31 / 2 / 372 / 1 |

[Selection checks](candidate-selection.json) confirm the exact inventory digest
in force, 14 lists/14 aliases, static invocation support disabled and all three
loader pins in force (the two definitions plus the Fluent Bit proxy citation)
on every route. This is a checked review surface, not review of an installed
plugin's callback bodies.

The canonical five-artifact comparison was run with `--full`.
[Raw diff](artifact-diff.json.gz) and [artifact review](artifact-summary.json)
retain raw source keys alongside normalized comparisons: 64 modules on both
sides, 60 raw key prefix moves, no normalization collision and no changed module
text. All 185 corpus resource units differ only in the cross-unit home
**environment identities**; function statuses and diagnostic facts are unchanged.
Their raw/preprocessed home source hashes and link map are unchanged. Environment
hashes bind serialized contract source provenance (`legacy_cross_unit`), so
checkout prefixes change those identities. Three source-effect rows
(`cio_chunk_is_locked`, `cio_chunk_unlock`, `flb_input_chunk_release_lock`)
change cwd-bound configuration and derived task/evidence identities only;
`boolean_source` explicitly binds the process cwd. Their source/profile/compiler,
claims and path bounds are unchanged. The configured reports similarly change
contract-source dependency/inventory identities and review-hold module prefixes
only. Their complete contract sidecars are equal after checkout-prefix mapping;
all other diagnostic data are equal. [Artifact hashes](measurement-provenance.json) retain these provenance
differences explicitly. Chunk artifacts, configured source-effect
artifacts and all null artifacts are byte-identical. The preceding baseline
archives are not re-pinned.

Reproduction from this worktree uses:

```sh
export CARGO_TARGET_DIR=/home/cosmo/Documents/GitHub/qcore-c/private/rk-interceptor-candidates-target
bash docs/reports/rk-plugin-interceptor-candidates/measure.sh "$PWD" "$CARGO_TARGET_DIR/release/quirin-audit" tip-final-v1
python3 docs/reports/rk-plugin-interceptor-candidates/compare.py /home/cosmo/Documents/GitHub/qcore-c/private/w48-w44-rk-integration/measure-combined private/rk-interceptor-candidates/measure-tip-final-v1 private/rk-interceptor-candidates/compare-final.json
python3 docs/reports/rk-plugin-interceptor-candidates/summarize.py private/rk-interceptor-candidates/compare-final.json docs/reports/rk-plugin-interceptor-candidates/comparison-summary.json
python3 docs/reports/rk-plugin-interceptor-candidates/check_selection.py private/rk-interceptor-candidates/measure-tip-final-v1 docs/reports/rk-plugin-interceptor-candidates/candidate-selection.json
```

The retained run outputs are under
`/tmp/qcore-rk-interceptor-candidates/private/rk-interceptor-candidates/measure-tip-final-v1/`;
the baseline primary remains at the path in `baseline-provenance.json`.
Use a new output name for another measurement. The canonical comparator reads
the private gzip copies of the baseline's five raw artifacts in
`private/rk-interceptor-candidates/baseline-artifacts/`; [review_artifacts.py](review_artifacts.py)
checks the complete diff and both configured reports without changing a verdict.

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
with these gate observations. The final pin harness
[pin.sh](pin.sh) recorded nine zero exits (pin/check for each outer provider at
each reviewed revision, then `pin --verify-contracts`); the final contract sync
also exited zero. Logs are under `private/rk-interceptor-candidates/pins-lifetime/`
and `contracts-sync-lifetime.log`.

| Command | Exit | Retained log directory |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | `gate-standing-final/submodule.*` |
| `gate.sh standing-pre -final` | 0 | `gate-standing-final/` |
| `gate.sh standing-tests -final` | 0 | `gate-standing-final/test-binaries/` |
| `gate.sh providers-pre -final` | 0 | `gate-providers-final/` |
| `gate.sh providers-tests -final` | 0 | `gate-providers-final/` |
| `gate.sh docs -final` | 0 | `gate-docs-final/` |

The `gate.sh` path is `docs/reports/rk-plugin-interceptor-candidates/gate.sh`;
all relative log directories above are beneath this card's worktree-private
`private/rk-interceptor-candidates/`. Formatting, Clippy and provider checks are
not inferred from the test count; their separate exit files are retained.

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

`RK-INTERCEPTOR-STATIC-LIST-CLOSURE`: static interceptor list invocation remains
unsupported: a list/element/union-arm closure and candidate composition need their own sound rule. Callbacks that free
or retain opaque state or transfer ownership do not meet this card's neutral
candidate and remain unsupported. No deployed callback has been certified.
