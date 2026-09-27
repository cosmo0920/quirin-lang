# W46 contract gap expansion

Work in progress. Astra owns the gap census and integration; Sol 6 at medium
reasoning implements the libc/Libevent/Monkey, metric/trace and Fluent Bit event
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

The implementation reports and final gate evidence will distinguish additions
from these open boundaries. No contract/lifecycle coverage gain is claimed by
this baseline report.
