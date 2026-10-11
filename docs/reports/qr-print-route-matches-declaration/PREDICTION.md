# QR-PRINT-ROUTE-MATCHES-DECLARATION: prediction (frozen before implementation)

Base: local `main` at `da6c1f448` (the brief's base). Committed before any
change to `crates/`, `providers/` or `proof/`.

## The fix

* **Primary guard (route selection reads a call, not a name).**
  `source_scalar_calls::has_print_calls` becomes a classification of every
  `print (` token pair:
  * outside every source function body (a declaration name, an `extern fn
    print(...)`, a contract clause, a type or record field): **ignored**;
  * inside a source function body at a statement head (the previous token is
    `{`, `;` or `}`): a **call**, which selects `source.scalar-calls.v1` as
    before;
  * inside a body anywhere else (`let x = print(1);`, `f(print(1));`), or a
    call in a module that also declares something named `print`:
    **ambiguous**, refused at the selection point (`Q020`), never routed.

  Comments are not tokens and a string literal is not a word, so neither can
  select today; the classification keeps that.
* **Secondary guard (defence in depth).** A `module contract;` module is
  refused at the entry of `compile_scalar_calls` (all three selection sites
  reach that one function), naming the reason, instead of being lowered there.
  It is secondary because it fixes only the contract-module instance of the
  defect: a library or executable module that declares `extern fn print`
  would still be misrouted by a token test.

## Census (every tracked `.qr`/`.qv`/`.qcore`/`.qcorelib`, two configurations)

* A text scan found **718** tracked sources, and **1** with a `print (` pair
  outside a comment: `examples/scalar/printing.qr:9`, `print(f);` at a
  statement head. Prediction: the token-level census agrees (1 file).
* `census-before` (base) against `census-fix` (selector and guard, no
  contract change): **0 moved rows** of 1,436; every row's route, outcome and
  emitted C byte-identical. No file is a correction.
* `census-fix` against `census-after` (the `print` declaration added):
  exactly the two rows of `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv`
  move, on the declaration count only (+1), still admitted in `Contract`
  mode on the same route.
* Inline `.qr` programs inside Rust tests: a scan of tracked test sources
  finds every `print(` program occurrence at a statement head; the full
  suite passes unchanged (no pinned diagnostic edited).

## Corpus, system, slice (release `quirin-audit`)

* **a0** (the fix's binary, contracts unchanged): identical to BEFORE on all
  three routes (933 / 9 / 3,892; 93 / 4 / 351; 52 / 1 / 347, 4 Pending), the
  corpus `resource-diagnostics.json` digest equal to BEFORE's.
* **a1** (the fix's binary, `print` declared as round 3 recorded it):
  corpus **935 / 9 / 3,890**, the two movers `cJSON_Print` and
  `cJSON_PrintUnformatted` (`librdkafka/cJSON.c`), Unknown -> Safe, each
  losing exactly ``call to uncontracted function `print` ``; no Safe ->
  non-Safe move; `print`'s own row does not move. System and slice: 0 moves
  (`cJSON.c` is in neither selection).
* Null property (`QCORE_NULL_PROPERTY`), corpus/system/slice: identical
  BEFORE and AFTER (the declaration is a resource contract, not a bounds
  contract).
* The new clause's standing in the corpus: a declared assumption (the body is
  not a Safe row of its own unit), never validated.

## Contract

* `print` re-declared in `librdkafka/cjson.qv` byte-for-byte as recorded in
  round 3's withdrawal commit (`214d1334f`), its body read at both revisions
  (`cJSON.c:1234-1299`, byte-identical; the whole file identical).
* `quirin-audit pin` at `66910c10` and `046accf6` records one definition site
  (`lib/librdkafka-2.15.0/src/cJSON.c`), with the same declaration and body
  digests round 3 recorded (`c36c98da…`, `f1b509f8…`); `pin --check` for the
  crate exits 0 at both; the whole-tree check keeps its one base row not in
  force (`mk_vhost_set`, tip mode).
* `qcore_contract_corpus` compiles `cjson.qv` in `Contract` mode; its pins
  move only where `cjson.qv`'s clause count is pinned.

## Lean

`QCore` has no model of source route selection (`ScalarCalls.lean` models the
checked route's semantics, not its selection). Prediction: a small new module
models the classification and proves that a `print` token outside every
function body, or a declaration name, never selects the route, and that an
ambiguous occurrence refuses; registered in an `Audit*.lean`.
