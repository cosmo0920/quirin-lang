# TYPE-GROUP-QUALIFIED-POINTER: prediction

Committed before the implementation commit. Base `7bfa0414b` (the tip of
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST, rule 74r, approved as r72, on
rule 74m's r68 and main `3ca85c7a6`). USER DECISION 2026-11-24: "launch the
TYPE-GROUP-QUALIFIED-POINTER card".

## What was known when this was written

No implementation was measured.

* **The gap.** `cast_semantics::type_group_open` accepts a group of
  identifiers followed only by trailing `*`s. `( long * const )`,
  `( char ** const )` and `( long * const * )` are not type groups, so rule
  74r's `dereferenced_through_groups` stops at them and
  `long w(void) { const long *p = 0; return *(long * const)p; }` is `Safe`
  (the finding card's end-to-end probe, its report §1), while
  `*(long *)p` is `Unknown` (`unknown_alias:p`). Rule 74m's through count and
  rule 12h(4)'s member and subscript count stop at the same group.
* **Consumers.** At the base `type_group_open` has two callers, both in
  family S of `legacy_return_family` (`stars_before` and
  `dereferenced_through_groups`). It is reached only for a `return`
  expression of a function whose definition declares a scalar result, after
  every older return route has declined.
* **Corpus census.** A `grep` of Fluent Bit 046accf6 `src/`, `plugins/`,
  `include/`, `lib/monkey` and the prepared monkey source for a parenthesised
  group of words with a qualifier (`const`, `volatile`, `restrict`,
  `__restrict`, `__restrict__`, `_Nonnull`, `_Nullable`, `_Atomic`, `__const`,
  `__volatile__`) after a `*` finds 63 lines. Most are parameter
  declarations (`MMDB_s *const mmdb`, `struct nameserver *const ns`). Seven
  are casts: `(const char * const *)tzname` (`src/flb_strptime.c:630`) and
  `(const char *const *const)path` (`plugins/filter_geoip2/geoip2.c:253`),
  both call actuals; `(const char *const *) ctx->args[1]`
  (`plugins/in_ebpf/traces/exec/bpf.c:151`, an assignment in a BPF program);
  three `(void *volatile *)l` in `winpthreads.c` (Windows only); and
  `(char *const *)child_argv` in libevent's `tinytest.c` (a test). **None is
  in a `return` expression.** A `return` that spans lines, or a cast a macro
  spells, is not covered by the `grep`.

## Planned change

`type_group_open` reads the C11 6.7.7 abstract declarator `pointer`: one or
more words (the specifier-qualifier list, as before), then any number of
`*`, each followed by a possibly empty type-qualifier list. A word after a
`*` must be a type qualifier (C11 6.7.3, the GNU spellings and Clang's
nullability qualifiers); any other word after a `*` keeps the group
unrecognised. Every group the base read is still read. Each consumer only
refuses more when it reads through more groups: `stars_before` counts more
`*`s (rule 12h(4)'s member root and rule 74m's starred root both need a
bounded count), and `dereferenced_through_groups` makes more bases (each an
obligation the flow check must discharge). `is_cast_group` and the other
detectors keep their readings.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe` -> `Unknown` (a new base on a possibly-null or
unknown binding), `Safe` -> `Unsafe` (not expected: an untracked root
cannot be released by this body), or an admission of family S withdrawn
(a may-bind root under a qualified cast group, then decided by the typed
return route or refused). Each needs a `return` with a qualified pointer
cast group on a dereference path, and the census finds none, so no move is
expected.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need `build/cycle19`
inputs, which this host does not have. The integrator should expect the same
result as on the opt-in route. Any move would be one this file does not
predict, and it would be reported.
