# W48 pointer return: prediction before implementation

Base: `fa04aab0fd43b242d2a996da2c6c507f1eb2a113`. The shared, read-only
corpus arbiter was run from `/tmp/qcore-rk-outer-base` at that exact HEAD,
against pinned source `66910c10`, with `--jobs 4` and exit 0. Its
`run.json` records release binary SHA-256
`336609c835dad7762e3729bfc699af561a61d1704f318e2db0f6fbb57e145ff7`.
The diagnostic SHA-256 is
`fd731251a789cde595ff8f5d7b66bbe9cdf9424c7bb1c64bc7bbff51eeec1701`:
**941 Safe / 9 provisional Unsafe / 3,884 Unknown** over 4,834 functions.
This card will confirm with its own worktree binary after implementation.

The current bottom-up plan, rerun on that artifact, has **264** Unknown
functions waiting on exactly one in-corpus callee body (and 8 with no body
left under the hypothetical cross-unit closure), rather than the 338 in the
older W48 report. Taking every in-corpus body Safe would free 372 under that
plan's upper bound. The plan's largest individual body is now
`cJSON_DetachItemViaPointer` (6 callers, 7 own leaves). Its seven leaves
need several independent effects. The read-only call-actual cluster is the
largest coherent single-rule candidate with one return leaf per body.

## Predicted movement

Three `rd_kafka_admin_result_ret_*` bodies remain Unknown solely on the
pointer return after `rd_list_cnt(&rko->rko_u.admin_result.results)`:
`acl_bindings`, `acl_results`, and `delete_acl_result_responses`. Each has
one caller waiting only on that body. A checked read-only frame for the
exact `rd_list_cnt` actual, joined with every other binding cause, should
make those **six functions Unknown → Safe**. `rd_kafka_ConfigResource_configs`
has the analogous `rd_list_cnt(&config->config)` and one return leaf but no
freed caller; it may add **one** direct Safe if its other return premises
hold. Thus corpus **+6 to +7 Safe**, with no Safe → non-Safe movement, no
new leak or violation. The upper end is 948 / 9 / 3,877.

System and tail-pause slice contain no librdkafka admin body in this shape;
no status movement is predicted there. A call whose body is missing or does
not validate as a read-only scalar-field return, an indirect call, a
different actual, or a root named by any other binding event continues to
refuse. The ordinary callee summary remains required for the call itself.

The exact-base source artifact's location and the private planning JSON are
recorded in the card report. The corpus and system/slice arbiter comparison,
Rust and Lean audit, gate, ledger and status updates follow implementation.
