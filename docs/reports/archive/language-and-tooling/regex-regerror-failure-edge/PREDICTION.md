# CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE: prediction

Committed before the implementation commit. Base `3ca85c7a6` (main, r65).
USER DECISION 2026-11-24: "launch them all".

## What was known when this was written

No implementation was measured. Known from reading the base:

* Rule 74i (`report_uncertified_key_uses`,
  `crates/qcore-c-gateway/src/legacy_in_place_custody.rs`) refuses every use
  of a tracked in-place object that is not certified, and reads no callee by
  name. `regerror`'s `preg` is `resource_borrowed` + `resource_kind(preg,
  regex)` in `regex.qv`, exactly as `regexec`'s is.
* `rd_regex_match` (`lib/librdkafka-2.15.0/src/rdregex.c`) calls
  `regerror(r, &re, errstr, errstr_size)` on the failure edge of its
  `regcomp`. CG25-IN-PLACE-REGEX measured it Safe -> Unknown on opt-in
  `current` (base `2c60986b0`, tip `a7e8bea5b`) with one leaf, the rule 74i
  refusal at that line. r65 changed nothing that reads in-place objects, so
  the row is expected `Unknown` at this base for that one reason.
* It is the only tracked `regex` object in any measured row
  (CG25-IN-PLACE-REGEX's census).

## Planned change

A contract clause `resource_kind_failure_use(p, KIND)` stating that the use
at `p` is defined on an object whose in-place initialiser may have failed.
Rule 74i admits such a use where the object is bound to at most one site of
its latest initialiser, not unknown, not released on any reaching path: on
the failure edge and before the status test. It is still refused before
the initialiser and after the finaliser, and a use without the clause
(`regexec`) keeps today's reading. `regex.qv` applies the clause to
`regerror`'s `preg` only.

## Predicted movements

* `rd_regex_match`: **Unknown -> Safe** on every route that analyses
  `rdregex.c` (corpus, twelve-home, opt-in `current`). Its only leaf is the
  rule 74i refusal at `regerror`, which the clause discharges; its other
  uses of `re` (`regexec` after the zero edge, `regfree`) were already
  certified.
* No other row moves: no other measured row tracks an in-place `regex`
  object, and no other declaration carries the clause. No new `Unsafe`, no
  `Pending` change.

Opt-in `current` (this card's arbiter): Safe +1, Unknown -1 against the base
run, the one moved row `rd_regex_match`, its one rule 74i leaf removed.

```measure-prediction
route corpus
move rd_regex_match Unknown -> Safe
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as above.
