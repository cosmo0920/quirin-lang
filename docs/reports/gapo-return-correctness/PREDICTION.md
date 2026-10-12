# GAPO-RETURN-CORRECTNESS: prediction (frozen before implementation code)

Base `1a399e6a1` (local `main` = the round-24 base). Baseline figures from the
round brief: corpus 935 / 9 / 3,890 (`bdf5f1c8…`), system 93 / 4 / 351
(`f596d64b…`), slice 52 / 1 / 347 / 4 Pending (`ee98ad3d…`); null property
corpus 1,113 / 1,435 / 2,355 / 18 Unsafe, system 84 / 125 / 245, slice
71 / 103 / 232. My own BEFORE is measured with the base's binary and its
digests are recorded in the report; a difference from these figures stops
the card.

## Task A alone (probe state restoration, rule 15x)

What changes: one snapshot of every parser field a classification may move
(`next_site`, `element_lowerings`, the list frames, the element origins, the
argument probe, the typed-position and widening counters, the variadic-tail
readings, the typed place origin, the cursor), restored by every declined
probe: the W4, B01 and W45-K probes of a refused return, rule 12h's
`uncast_call_actuals` (which re-classifies from the state before the first
classification), `variadic_tail_statement` when the widened result is not
admitted, the W43 descriptor guard and classify_simple's other guards, and
every existing `(next_site, element_lowerings) = saved` restore.

Prediction:
- **No verdict moves** on any route (corpus, system, slice), custody or null
  property. The return probes' synthetic destination is the reserved place,
  which no later statement names, so a leaked list-frame protection or element
  origin of that name is unobservable; an element lowering leaked by a probe
  can only make a `for` loop counted around a body whose return is then
  refused or lowered by a fallback that counts its own lowering.
- **Site numbering**: functions where rule 12h re-classified a statement, the
  variadic tail fallback declined after minting, or the descriptor guard
  refused an admission that minted sites, renumber their later sites (gaps
  close). Sites appear in the parity stream; I expect parity rows naming
  sites (`produced_sites`, instantiation rows) to differ in those functions
  only, and `resource-diagnostics.json` to be byte-identical unless it prints
  site numbers (then the class is attributed as renumbering only).
- Corpus, system and slice totals and verdict-row digests identical.

## Task A + B (return-position gaps, rules 15y and 15z)

- **15y, no-return path end at every lowered position.** A CFG pass after
  the build: any node whose instruction calls a callee declared
  `resource_no_return()` and that still has a successor becomes a path end
  when it is the discarded whole call and Clang resolves the callee
  `noreturn` (rule 11o's own premises), and is refused otherwise (nested,
  stored, embedded or unresolved). Prediction: few nodes in the corpus
  (0-30 nodes; abort/exit/__assert_fail lowered by W44, 10u, 10y, 11w or in
  a condition plan); per function either Unknown -> Safe (a continuation that
  refused is no longer reached; legitimate under 11o's premises) or
  Safe -> Unknown (an unresolved or embedded no-return call now refuses: a
  named correction). Estimate 0-5 functions move in the corpus, none in
  system or slice.
- **15z, return-position routes.** Rule 12h's H-U0 asked at the return
  position after the W4, B01 scalar-call, W45-K and 12i classifications.
  Only an admitted call is re-read, so a move is Safe -> non-Safe (a cast
  actual hid a place whose state now refuses) or no move. Estimate 0-2
  corpus functions. Rules 11w (tail, `va_*`) and 10u at the return position
  are recorded as out of the route's scope (the return keeps its refusal, or
  the flow's `arguments do not match resource contract` refusal), with a
  census row per refused return they would read; the census counts are
  measured, not predicted (expected tens of returns).
- Null property: moves only in functions whose custody verdict moves.
- Wall time: within noise (the snapshot clones small per-function sets).
