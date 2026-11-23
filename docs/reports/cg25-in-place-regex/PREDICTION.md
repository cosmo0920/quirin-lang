# CG25-IN-PLACE-REGEX: prediction

Committed before the implementation commit. Base `2c60986b0` (main). USER
DECISION 2026-11-23: the "soundness first" set of four implementation cards,
this one discharging `CG25-IN-PLACE-REGEX-USE-CERTIFIED` and
`CG25-IN-PLACE-REGEX-SHADOWED-NAME`.

## What was known when this was written

No implementation was measured. Known from reading the base and from the
pins `crates/qcore-c-gateway/tests/in_place_regex_certified.rs` run at base
(`private/card-logs/pins-at-base.log`: 4 passed, 5 failed, every failing
pin `Safe` at base):

* Probes R2, R3 and R4, a use before any `regcomp`, and `regerror(r, &re,
  …)` on the failure edge are all `Safe` at base.
* The only tracked `regex` object in any measured row is `rd_regex_match`'s
  `re` (CG25-VOID-CAST-GUARD-ESCAPE census: corpus and `current` both list
  four `rdregex.c` rows, `rd_regex_match` the only one tracking an object).
* **The registered follow-up's premise does not hold for the real
  function.** It says `rd_regex_match` "uses `re` only after the zero edge".
  The source (`lib/librdkafka-2.15.0/src/rdregex.c:124-128`) is:

  ```c
  r = regcomp(&re, pattern, REG_EXTENDED | REG_NOSUB);
  if (r) {
          if (errstr)
                  regerror(r, &re, errstr, errstr_size);
          return 0;
  }
  ```

  `regerror`'s `preg` is kinded `regex` and borrowed, exactly as
  `regexec`'s is (`regex.qv`), so it is a use on the failure edge. The
  fixture called "verbatim" in `tests/resource_kinds.rs` replaced this
  `regerror` with `k_wipe(errstr, …)`.

## Planned change

* Rule 11v (7)'s certified-use check (`report_uncertified_key_uses`) is
  generalised to every in-place kind: a use at a kinded borrowed position of
  an object a creation of this CFG tracks is admitted only where the
  settled state certifies it (one identity, not unknown, not possibly
  failed, site live or released). Otherwise it is refused (`Unknown`).
* Every in-place actual must name the one automatic object every
  `DeclRefExpr` of the name designates (`every_reference_is`, now for every
  kind, not only `thread_key`).

The contract language has no clause that tells `regerror` (defined on a
failed object: POSIX passes it the `regex_t` of the failed `regcomp`) from
`regexec` (undefined there), and providers are out of scope. So the check
reads no callee by name, and `regerror` on the failure edge refuses with
`regexec`. That is an over-refusal, and it costs `rd_regex_match`.

## Predicted movements

* `rd_regex_match`: **Safe -> Unknown** on every route that analyses
  `rdregex.c` (corpus, twelve-home, opt-in `current`), refused as an
  uncertified use at its `regerror` line. This undoes CG25-RESOURCE-KINDS'
  one `Unknown -> Safe` move. No other row moves: no other measured row
  tracks an in-place `regex` object, and no row tracks an automatic key.
* No `Unknown`, `Pending` or `Unsafe` row becomes `Safe`. No new `Unsafe`.
* The shadowing check moves nothing measured: `rd_regex_match` shadows
  nothing.

Opt-in `current` (this card's arbiter): Safe -1, Unknown +1 against the base
run, the one moved row `rd_regex_match`; leaf counts change only by its
refusal.

```measure-prediction
route corpus
move rd_regex_match Safe -> Unknown
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as above: corpus
`rd_regex_match` Safe -> Unknown, system and slice unchanged (neither calls a
regex function, CG25-RESOURCE-KINDS §5).
