# CALLBACK-CANDIDATE-LITERAL-PREMISE: prediction

Committed before any gateway, test or Lean change. Base main `062974679`
(r33). User decision 2026-11-19: launch the named follow-up
`CALLBACK-CANDIDATE-LITERAL-PREMISE` (recorded by
OPENSSL-BIO-READER-INVOCATIONS).

## The plan the prediction is about

A literal-argument premise (C04.5) restricts a declaration to the call sites
whose named actual is the named literal. Route G enforces it at a direct
call by renaming an unmet call to an undeclared callee
(`legacy_literal_premise::restrict_to_declared_literals`). A callback
candidate is not a call the rewrite sees: it is a function designator handed
to an invoker, and the library chooses the actuals it is called with. The
callback route classifies it by its contract alone:

* a function-map declaration through `callback_contracts::classify`
  (rule 13u's `classified_in`; rule 13w's registration, which stores the
  candidate's contract; rule 12o's constructed member registration, which
  does the same);
* a withheld declaration whose callback clauses are all member invocations
  through `candidate_composition` (rule 13u amended), which a declaration
  with slot invocations beside a premise now qualifies for
  (`d2i_PKCS12_bio`, r33).

**The change** (rule 57 amended in place): a candidate whose declaration
carries a literal-argument premise, and every candidate under a poisoned
premise set, has no contract in force wherever it is a candidate. The
callback route reads the premise set route G reads, once per function, and
classifies such a candidate `Missing` before any other reading, so a
synchronous, table, member or slot invocation refuses it
(`callback-uncontracted`, naming it), a registration stores no contract for
it (so a registry holding it is undecided), and a constructed member
registration stores none either. The premise is not checked against the
invocation's actuals: a `_` or a supplied object never satisfies it, and a
parameter actual is the caller's, which the declaration does not bound.
This can only refuse candidates that were applied before; it never applies
one that was not.

**Lean.** A new `QCore.CallbackCandidatePremise` beside
`QCore.CallbackLiteralPremise`: a premise-carrying or poisoned candidate is
`Missing`, a premise-free one is classified as before, the guarded decision
applies only where the old one did, and an applied invocation or a
releasing registry has no premise-carrying candidate.

## The survey

Every literal-argument premise in the shipped provider crates, with how the
callback route classified each before this card (`classify` over a plain
contract: every position borrowed or owned and no result is the identity):

| Crate | Declaration | Premise | Before, as a candidate |
|---|---|---|---|
| Fluent Bit | `flb_hash_table_add` | `val_size = 0` | identity (5 actuals), applied |
| Fluent Bit | `flb_task_users_dec` | `release_check = 0` | effectful (a counted-user drop) |
| cfl | `cfl_kvlist_insert_bytes` | `referenced = 0` | identity (5), applied |
| cfl | `cfl_array_append_bytes` | `referenced = 0` | identity (4), applied |
| libc | `fcntl` | `cmd = 2` | identity (2), applied |
| libc | `prctl` | `option = 15` | identity (1), applied |
| libc | `pthread_join` | `thread_return = 0` (also a thread-synchronization obligation) | identity (2), applied |
| libc | `pthread_key_create` | `destructor = 0` | effectful (an out and a kind) |
| librdkafka | `rd_kafka_topic_partition_destroy0` | `do_free = 1` | releases its element at an `element` position, takes at a `supplied_owned` one: applicable as a releasing or taking candidate |
| OpenSSL | `PEM_read_bio_X509`, `PEM_read_bio_PrivateKey` | `x = 0` | withheld (synchronous and slot invocations), not composable: uncontracted |
| OpenSSL | `d2i_PKCS12_bio` | `p12 = 0` | withheld, composable (slot invocations only); its owned result makes it effectful |

libcurl and the other provider crates declare no literal-argument premise.
The in-tree test sets (`hash_contracts_generic.qv`, the `contract_inherit`
fixtures, the inline sets of `callback_literal_premise.rs` and the other
premise tests) are fixtures, not a measured route.

Seven declarations could have been applied as a candidate. A row relied on
that only if one of them is a candidate of an invocation in it. Fluent Bit
`046accf6` (`git grep` over `*.c`, `*.h`) names none of the nine non-OpenSSL
functions as a designator outside a direct call, except a cast `fcntl` in
SQLite's `aSyscall` table (a cast actual, which rule 11p opens) and
`REG_NATIVE_FUNC(pthread_join, …)` in WAMR, whose macro names the
`_wrapper` function.

## What can move

**Pinned routes.** The pinned modules carry the Fluent Bit, cfl and libc
premises, but no pinned row is predicted to rest on one as a candidate. A
move, if any, can only be from `Safe` or `Pending` to `Unknown`.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The opt-in `current` run** (this card's arbiter; OPENSSL-SSL-CTX-CALLBACK-SLOTS'
script, `--monkey-linked`, Fluent Bit `046accf6`, base `062974679`): r33's
tip figures, 1098 / 2 / 3699 / 5 with 113 bindings all `Pending`, on both
sides. No row moves and no row changes content.

The rows most likely to prove this wrong: a row whose callback invocation's
candidate set names `flb_hash_table_add`, a cfl insert or a libc call through
a resolved function pointer (rule 11p), which would move `Safe` or `Pending`
to `Unknown`. Any such move is the fix working (the candidate was applied
outside its premise) and would be reported row by row.
