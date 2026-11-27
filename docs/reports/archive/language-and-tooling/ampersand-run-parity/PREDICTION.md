# AMPERSAND-RUN-PARITY-OTHER-READERS: prediction

Committed before the implementation commit. Base `9193b196a` (main). USER
DECISION 2026-11-23: "launch the ampersand parity card".

## What was known when this was written

No implementation was measured. Known from the census of the base's token
readers (README §1 lists every site):

* The importer's lexer emits one token per `&`. Rule 74d (C) read a maximal
  run of `k` `&` tokens before a name as an address exactly when `k` is odd,
  in rule 10a's frame only.
* Seven readers take an address only after a run of exactly one `&`
  (`texts[k-1] == "&" && texts[k-2] != "&"`) or read any `& &` before a value
  as a logical AND, so `p && &q` hides `&q`: `loop_owner_instance::address_taken`
  and its copy-member check, `loop_owner_holder_opaque`'s carrier `written`
  and `flow_of`'s `logical_before`, and three checks in
  `thread_entry_census` (the instance, the copy's holder, the copy). One
  call index (`loop_owner_coro_plugin`) drops a call after any `&`, so
  `x && f(i)` hides a caller. These are Fluent Bit loop-owner and
  thread-entry premise derivations (P-HOLDER-OPAQUE, the coroutine and
  instance premises), not the resource checker.
* Two readers admit `p && name)` as a whole `&name` actual
  (`only_holder_mentions`, `only_registry_mentions`): the admitting direction.
* About twenty readers read any `&` before a name as an address (`p && q`
  takes `q`'s address), the fail-closed direction: refused, not admitted.

## Planned change

One shared helper (`ampersand_runs.rs`) answers "how long is the run of `&`
tokens ending just before this index" and "does it end in the unary `&`"
(odd). Every site above that decides address-taking reads it; each keeps its
own binary/unary distinction by the preceding token where it has one. The
call index reads `& f (` as a call (an address of a call result is not an
lvalue).

## Predicted movements (opt-in route, base vs tip)

* **The unsound sites**: the loop-owner and thread-entry derivations do not
  feed the opt-in `current` route's resource rows as far as the base shows;
  expected **no** status move from them. If they do feed it, a fix can only
  withdraw a premise (Safe→Unknown), never admit one.
* **The admitting sites** (holder/registry mentions): a row whose holder is
  spelled after `&&` in an actual would lose its admission (Safe→Unknown).
  Not expected in the corpus (a holder is a record object, and `p && record`
  is not a conforming operand).
* **The fail-closed sites**: a name after an even run (`p && q`) is no longer
  read as address-taken, so a refusal that rested only on that reading can
  go. Expected: leaf-only changes in a handful of rows, and possibly a few
  Unknown→Safe where it was the row's sole obstacle; not predicted by name.

Expected totals equal to the base's, or within a few rows; every moved row is
reported, and a lost Safe row is shown with its address escape in the C
source.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines). Expected there: the system and
tail-pause slice routes are where the loop-owner and thread-entry premises
are derived; a premise that held only because `p && &x` was misread would be
withdrawn (Safe→Unknown), none is expected. Corpus routes: as the opt-in
route. Any move is unpredicted and reported.
