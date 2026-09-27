# W46 contract gap expansion

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Astra owns the gap census and integration; Sol 6 at medium
reasoning implemented the libc/Libevent/Monkey, metric/trace and Fluent Bit event
tracks in isolated worktrees. Base `9bce3986`. The
[prediction](prediction.md) was committed before implementation integration.

## Gap selection

The [census script](scripts/census.py) records exact uncontracted-call diagnostics
in the requested symbol families, including the unit configuration, function,
source location and any existing tracked declaration. A declaration can be an
observation policy, unselected, unreadable or withdrawn. Its mere existence does
not establish that the resource analysis has an applicable contract.

| Requested family | Corpus callees / call leaves | System-slice callees / call leaves |
|---|---:|---:|
| libc names | 38 / 126 | 5 / 12 |
| Libevent | 0 / 0 | 0 / 0 |
| Monkey | 193 / 370 | 4 / 8 |
| CMetrics | 73 / 305 | 3 / 8 |
| CTraces | 48 / 96 | 0 / 0 |
| input chunk | 19 / 31 | 25 / 85 |
| flushing/event management | 2 / 2 | 61 / 228 |

These are symbol-family counts, not a claim about each function's library
ownership. They count each compilation configuration, and the scopes overlap:
do not add the columns. No Libevent leaf in these selections says nothing about
the completeness of its API contracts. Libevent additions need focused evidence.
The complete records are [corpus](census/corpus-before.json) and
[system slice](census/system-before.json). This is not an exhaustive API inventory
and not safety coverage.

## Baseline evidence

The pinned arbiter ran at the base with `--jobs 8`, exit 0, private log
`private/w46-contract-gaps/baseline/corpus/audit.log`. It reproduces the previous
baseline diagnostics, chunk-contracts and source artifacts byte for byte:
405 Safe / 9 provisional Unsafe / 4,420 Unknown functions, and 5 Safe / 8
provisional Unsafe / 168 Unknown / 4 data-only units. The contracts artifact is
equal after the recorded checkout-root substitution; source-effect identity
digests differ with the contract-source identities. The
[identity record](census/baseline-identity.json) publishes each comparison
separately, including the nonidentical ones.

The configured system slice at `046accf6` ran with `--jobs 4`, exit 0, private log
`private/w46-contract-gaps/baseline/system/console.log`. Its 15 unit records and
function records are unchanged apart from compile-configuration contract-source
dependencies and their inventory digests ([comparison](census/system-baseline-identity.json)).
The run preserves 2 refuted contracts and 10 held call sites. Counts per
compilation configuration are 54 Safe / 4 provisional Unsafe / 390 Unknown
function rows; those are not the status ledger's deduplicated function count.

## Source-read boundaries

Both Fluent Bit checkouts are clean. The relevant source revisions are
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and
`046accf6aded7906f66a1baf0202da349e521736`; Libevent also has the repository's
pinned submodule identity. Source pins record reviewed text, not a proof of C.

* `cmt_log_print` calls the configurable `cmt->log_cb`; a high call count
  does not license a neutral contract. The callback's effects need a premise
  the current generic contract can express and enforce.
* `ctr_id_set` destroys/replaces `cid->buf`. `ctr_id_get_buf` returns that
  separate allocation, not an interior address of `cid`; declaring an alias or
  an interior result of `cid` would misstate the relation.
* `flb_input_chunk_flush` may bring the underlying chunk into memory, lock it
  (the source explicitly allows relocation), then return its content pointer
  and mark the wrapper busy. A borrow/none declaration would lose that relation.
* `flb_output_flush_prepare_destroy` moves an object between active and destroy
  lists. `flb_output_flush_finished` iterates the destroy list and destroys
  its coroutines. Their effects reach objects owned by a container.
* `flb_engine_flush` dispatches each selected input. Queue publication, task
  user counts, retry context and callback effects need their own checked
  specifications; a scalar return is not evidence of custody neutrality.
* The existing output singleplex/task-user specifications, including refuted
  OR-1 and OR-7, are retained. This work must not make a held caller Safe by
  replacing those specifications with weaker declarations.

The three implementation reports distinguish the selected declarations from
these open boundaries. These are reviewed source premises, not proofs of the
corresponding C bodies or complete lifecycle coverage.


## Implemented inventory and integration decisions

The [added-symbol inventory](census/added-contracts.json) compares tracked
resource declarations with `9bce3986`: **36 additions**, no analyzer admission
change. The selected resource environment grows **183 → 219 declarations** in
**50 → 54 modules**; the entire contract corpus grows **114 → 118 modules**
(73 provider-owned, 45 shared). There are 114 pinned declarations and 105
flexible libc declarations; rejected legacy-policy declarations remain 216.

| Implementation card | Additions | Final branch tip | Main merge |
|---|---:|---|---|
| [libc / Monkey / Libevent](../w46-libc-event-contract-gaps/README.md) | 4 / 4 / 3 | `f61882a0` | `eb913c38` |
| [CMetrics / CTraces](../w46-metric-trace-contract-gaps/README.md) | 13 / 6 | `0c3b627e` | `542f7d6d` |
| [Fluent Bit event helpers](../w46-fluent-event-contract-gaps/README.md) | 6 | `d461a488` | `ec8fded0` |

The four libc declarations (`memcmp`, `strncmp`, `strnlen`, `strcasecmp`) also
have existing builtin handling for some expression routes. They provide reusable
catalog/ABI contracts and now support additional condition/summary routes. The
initial corpus uncontracted-call census alone understated this benefit: three
functions move under `strcasecmp`, and the system slice removes four `strnlen`
uncontracted-call leaves plus four `strcasecmp` condition leaves. Three Libevent queries have focused source
and ABI evidence despite no Libevent baseline leaves. Source pins cover both
Fluent Bit revisions; Libevent queries additionally cover `d82464a2`.

Shared-file merge resolutions take the union of all declarations, source pins,
Lean imports and exact-count assertions. `providers/contracts.py sync` and
`check` passed after the final merge. The concurrent `W46-ACCESS-PATH-CENSUS`
merge and its two follow-ups are preserved; its own integration established
unchanged default-build artifacts. It changes feature-gated measurement only.

Eight proposed residuals were registered with their proposed wording: the four
Fluent Bit proposals in that card's `followups.json`, and the four proposals in
[this report](followups.json). The retained-result follow-up
`W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED` now explicitly includes the raw TLS
getter; its scalar flush-ID wrapper does not discharge it. The previously
registered `W36-CTRACES-FAMILY-FIRST` is discharged by integrator confirmation
of the provider, namespace, directory and existing facade naming convention;
[the original entry and decision](retired-followups.json) are preserved.
That is a repository-organization decision, not C-body evidence. Follow-ups:
511 → **518** (+8, −1, one amended). `docs/agents/providers.md` now states the
resource-view equality invariant and refers to current status for its count.


## Merged measurements and attribution

The release binaries were rebuilt successfully after merge `eb913c38`. Both
runs exit **0**: the 185-unit corpus uses `run_audit.py ... eb913c38 --jobs 8`
with `/usr/bin/clang`, and the configured system slice uses this report's
`run_system.py ... --jobs 4` with `/usr/lib/llvm-21/bin/clang`. Exact command,
binary digest, environment and timestamps are in each archive's run record.
Private runs: `private/w46-contract-gaps/after/corpus/` (`audit.log`) and
`private/w46-contract-gaps/after/system/` (`console.log`, `stderr.log`, `exit`).
The builds' logs and exit files are `private/w46-contract-gaps/after-build-{corpus,system}.{log,exit}`.

| Scope | Before | After |
|---|---|---|
| Corpus functions: Safe / provisional Unsafe / Unknown | 405 / 9 / 4,420 | **411 / 9 / 4,414** |
| Corpus units: Safe / provisional Unsafe / Unknown / data-only | 5 / 8 / 168 / 4 | unchanged |
| Corpus unsupported / unknown_alias / leak | 41,239 / 3,572 / 34 | **41,086 / 3,573 / 34** |
| Corpus uncontracted leaves on Unknown functions | 9,084 | **9,022** |
| System configuration rows: Safe / provisional Unsafe / Unknown | 54 / 4 / 390 | unchanged |
| System unique projected functions: Safe / provisional Unsafe / Unknown | 28 / 2 / 204 | unchanged; 3 excluded |
| System unsupported / unknown_alias / counted_user | 3,321 / 713 / 4 | **3,303 / 713 / 4** |
| System uncontracted leaves on Unknown functions | 731 | **717** |

The six function moves are `mk_rconf_section_get`, `mk_config_key_have` and
`bool_val` under `strcasecmp`; `mk_string_search` and `mk_string_search_n`
under `_mk_string_search`; and `cat_scalar_value` under seven CMetrics scalar
premises. No function moves from Safe to a non-Safe result. No new leak or
violation leaf appears. Unknowns remain in their denominators; no bounds,
callback composition or whole-library conformance is inferred.

The corpus removes **155** leaves and adds **3** across **84** changed functions.
The added leaves are one still-refused borrowed expression in `mk_rconf_read`
and two unresolved `conn` identities at newly modeled event calls in
`mk_net_conn_create`. One `fw` alias refusal disappears immediately after
`MK_EVENT_NEW` becomes a borrow; the next pointer-valued store and its later
alias refusals remain. These functions stay Unknown.

The system slice removes **20** leaves and adds **2** across **16** changed
configuration/function rows. The two additions preserve a counted-user
refusal in `flb_output_task_flush`: the newly declared flush-ID wrapper is
still undecided against its body because its TLS getter lacks an applicable
body/contract there. The [refutation table](artifacts/system-after/refutation-preservation.json)
is exactly unchanged: **2 refuted declarations, 3 held functions, 10 held
call sites**. OR-1 and OR-7 stay provisional Unsafe in their own bodies and
held Unknown at callers.

The automated attribution deliberately exits **1** for both scopes: it leaves
30 corpus and 2 system differences for source review. These are multiline
conditions, macro expansions, header-local helper summaries, two conditions
calling a newly Safe same-unit body, and the downstream `fw` alias effect.
The [exact reviews](census/reviewed-attribution.json) bind every remaining
observation to source digests and excerpts. `scripts/check_attribution.py`
exits **0** with no missing or stale review, covering all 32 differences; it
checks record identity, not the correctness of the source-reading judgment.
The automated reports remain unaltered rather than erasing their initial
unattributed entries ([corpus](census/corpus-attribution.json),
[system](census/system-attribution.json)).

[Unit metadata](census/unit-metadata-differences.json) changes only where
expected: diagnostic reasons in 42 corpus and 4 system units, plus all 15
system units' contract manifest/source dependency identities and derived
inventory hashes. Unit verdicts, reason codes, function inventory and scope
are unchanged. [Preservation checks](census/preservation-checks.json) also
confirm byte-identical corpus source and chunk-contract artifacts. The
concurrent access-path census introduces no unexplained default-build change.

Reproduce with the scripts in this report. Raw gzip archives are
[corpus before](artifacts/before/manifest.json),
[corpus after](artifacts/after/manifest.json),
[system before](artifacts/system-before/manifest.json) and
[system after](artifacts/system-after/manifest.json). Baseline system invocation
was run directly and has no `run.json`; its command, console, stderr and exit
are retained. Summary JSONs distinguish per-configuration rows from the join
of unique `(file, function)` verdicts; no configurations disagree.


## Reconciliation

The [record](reconciliation.json) applies all ten integrator items with checked
file assertions. The residual sweep changes **102 → 106** candidates: the
retired CTraces decision (discharged here), plus the three completed card titles
newly present in the increment record. None is a newly orphaned gap
([comparison](census/residual-comparison.json)); the eight actual proposals are
registered. The historical deferred-decision registry is carried without
adjudication, not treated as current measurement. This is a task integration,
not a wave close or a new cadence/stability review.


## Standing and provider gate

The [gate record](gate.json), [exact steps](gate/steps.json) and
[orchestrator environment](gate/command.json) record the merged run at
`eb913c38`. Core Rust: **324 units / 336 summaries, 6,029 passed / 0 failed /
11 ignored**, exit **0**; per-binary logs:
`private/test-binaries/20260927T022512Z-4165084/`. Provider Rust: **129 passed /
0 failed / 1 ignored**, 43 summaries, exit **0**. Lean **212** audits and
provider Python **15** tests pass. Submodule setup, core/provider formatting
and clippy, both reevaluation examples, contract inventory checks, all five
package builds/verifications and Clang recording exit **0**.

The initial root Python suite exits **1**, **363 passed / 1 failed** out of
364: the updated matrix was serialized with escaped Unicode instead of the
generator's byte format. Regeneration fixes it; the exact failed test passes
**1/1**, exit **0**, at `private/w46-contract-gaps/matrix-format-repair.log`.
The complete post-regeneration Python suite and final index/reconciliation
checks are recorded in [final checks](gate/final-checks.json): **364/364 Python
tests pass**, and index, reconciliation, attribution and staged diff checks
all exit **0**. No second Rust
run is needed for this evidence-format repair. The original orchestration
exit **1** and each per-step exit remain in the logs; no all-green first
attempt is claimed. Private gate logs: `private/w46-contract-gaps/integration-gate/`.
The initial reconciliation checks also refused missing carried decisions and
then the wrong `open` spelling; the final record uses the supported `deferred`
verdict and passes without changing the historical decision registry.

Card-local failed attempts are preserved in each implementation report:
sandbox sanitizer/confinement failures, the corrected CMetrics profile digest,
and Fluent/libc exact-count and index repairs. The fresh merged Rust/provider
run passes all those tests. Per-card `integration/gate.json` files identify
each merge and explicitly borrow this run's Clang recording; they record one
shared run, not three additional measurements. The merged gate's implementation
bytes are the measured tip; later edits are test comments and report, status,
matrix and reconciliation evidence only. No native observation campaign,
foreign-host result or additional lifecycle proof is claimed.
