# LOG-PRINT-BODY-CONTRACTS prediction

Committed after the step 1 diagnosis (`e5229556a`) and before any contract,
code or lock change. Base for the rows: `baseline:c8c7edf21` (the main
checkout's `private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`). No
baseline for this card's base `945c0db1b` is published; the integrator
measures against whichever is.

**What changes.** Two pinned resource-v1 declarations, each stating the five
fixed parameters borrowed, the context's log callback member invoked with
`(ctx, level, file, line, _)`, and no result: `cmt_log_print` in
`qcore-provider-cmetrics` (`cmetrics/logging.qv`, member `cmt.log_cb`) and
`cio_log_print` in `qcore-provider-chunkio` (`chunkio/logging.qv`, member
`cio_options.log_cb`). No variadic tail is declared (the loader refuses one
beside a callback clause), so a call passing an actual after `fmt` stays
refused. No analyzer code changes.

**Why nothing is decided today.** Rule 12n decides a member invocation over
the run's closed set of the member's stores. On every route at the base
the member pre-pass leaves 36 units unread (34 preprocessing failures of
ungenerated sources such as `lib/luajit-cmake/lj_vm.S` and the bison/flex
outputs, and 2 C++ units), and an unread unit opens every member: the
corpus baseline's sidecar and the user's run list every invoked member as
"36 unit(s) of the run may name it but were not read"
(S3-PREPASS-UNREAD-GRAPH-ISOLATION). So `cmt.log_cb`, which no read unit
stores into, is open too, and `cio_options.log_cb` is open on its own
account as well (the setter's `void *` formal). Every call of either printer
stays refused, now as `callback-member-open` (or the arity refusal for a
call with a tail actual) instead of "call to uncontracted function".

**Affected rows at the base.** Only the corpus route (`66910c10`) analyses a
caller: 24 cmetrics functions (61 sites) and 23 chunkio functions (55
sites), all Unknown. The system and slice selections analyse none (0
sites), so their rows change only in contract-bundle digests. On the corpus
the 47 rows trade leaves (content-changed), none moves: each call is still
refused, so no state after it is newly analysed and no masked leak can
surface.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`, the same 36 unread units), rerun by the
integrator after landing:

- Both callees leave the uncontracted list: 2,790 -> 2,788 callees and
  7,264 -> 7,217 (function, callee) pairs. Their 116 leaves become
  `callback-member-open` refusals (and arity refusals at tail sites).
- The "blocked only by uncontracted calls" set loses the functions whose
  only other leaves were uncontracted calls, because the replacing refusal
  is not an uncontracted call: the 13 cmetrics and 4 chunkio functions of
  the diagnosis leave it, 508 -> 491, with no status change. None was
  decidable by these two callees alone (each has another uncontracted
  callee).
- No unit or function changes status: 8 Safe, 7 provisional Unsafe,
  167 Unknown, 4 data-only.

**When the run's set closes** (after S3-PREPASS-UNREAD-GRAPH-ISOLATION), the
43 `cmt_log_print` calls without a tail actual are decided (the member has
no store, so the call invokes nothing); the fixture test shows it. The
chunkio calls additionally need MEMBER-CLOSURE-VOID-POINTER-SETTER-FORMAL
and candidate contracts for the three static `log_cb` functions.
