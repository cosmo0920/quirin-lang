# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4: prediction

Committed before any implementation commit. Base `9b107caf1`. USER DECISION
2026-11-21 (through the integrator's brief): follow up the indirect-access
refusals with a round 4 on the largest class, the callee contracts.

**What was known when this was written.** The base census (section 1) and
two unit-scoped probes, run before this file: a `current --selection` of
seven librdkafka units with the `rd_jitter` declaration pinned (one move,
`rd_kafka_broker_update_reconnect_backoff`), and one of `rdxxhash.c` with
the xxHash finalizer declarations pinned (no move: the decayed `U32[4]`
member actual stays refused; a held-out fixture located the refusal in rule
35's element test, section 2). The prediction below is for the full opt-in
route, which neither probe measured.

## 1. Census (step a)

Opt-in `quirin-audit current --monkey-linked` over Fluent Bit 046accf6, base
binary `6acb901c…` of `9b107caf1`: **1,150 Safe / 2 Pending / 3,645 Unknown
/ 7 Unsafe**; 2,154 Unknown rows carry the access refusal (9,066 leaves),
**222** carry nothing else. Round 3's scripts, unchanged (180 of 180 units
sha256-matched, 9,082 leaf groups, 0 unbound): the class
`uncontracted-callee-unread-actual` blocks **40** functions alone (56
mentioning), the same 40 as round 3. `scripts/rank.py` reads each callee's
body: **39 of the 40 have a callee whose honest contract is not neutral**
(an allocating out-parameter, an owned constructor into a member, a
container, a timer callback, a reference count, the premised logger), so
no declaration this card may write frees them; one (`rd_jitter`) is
neutral. The xxHash finalizers (`XXH32_finalize`, `XXH64_finalize`) are
`static` in their callers' unit, whose bodies are `Unknown` (pointer
increments), and are neutral: a declaration and a reading together free
their callers.

## 2. Planned changes

* Declarations (`librdkafka/memory.qv`, pinned at `66910c10` and
  `046accf6`): `rd_jitter`, `XXH32_finalize`, `XXH64_finalize`, every
  formal `resource_borrowed`, scalar result `resource_return_none()`.
* Gateway rule 66 (`typedef_scalar_element.rs`, a hook in
  `rule_hosts.rs::decayed_address`, facts in `typed_access_path.rs`): rule
  35's second arm reads a decayed one-dimensional array member only when its
  element is spelled with builtin scalar words; Clang gives an array of a
  typedef (`U32 mem32[4]`) no desugared spelling, so the element was
  `Other`. Rule 66 reads the element `T` through the unit's `TypedefDecl`s:
  where all of them resolve to one builtin scalar, the actual is read as rule
  35 reads that scalar. Lean `QCore.TypedefScalarElement`.

## 3. Predicted movements (opt-in route)

**Unknown -> Safe:** `rd_kafka_broker_update_reconnect_backoff` (the
declaration alone), `XXH32_digest_endian`, `XXH64_digest_endian` (the
declarations and rule 66), and through their summaries `XXH32_digest`,
`XXH64_digest` (each holds only the two calls of its `_digest_endian`).
Range 3-5. Rule 66 elsewhere (a `uint8_t`/`uint32_t`/`U32` array member at a
`Borrowed` actual of a contracted callee, `memcpy(d, s->buf, n)`): 0-15
further rows, not enumerated.

**Leaves:** `call to uncontracted function \`rd_jitter\`` leaves leave about
ten rows that stay Unknown (re-attribution, measured in the probe); access
leaves fall by the actuals rule 66 reads (10-200).

**Invariants:** no Safe -> non-Safe move; no new Unsafe (a newly read
statement could expose a leak downstream; none is expected, and any would be
reported and investigated, not hidden); no Pending moved; access-alone
222 -> 219-221.

**Residue:** of the class's 40, 37-39 remain, each blocked by its callee's
custody question (section 1); registered as follow-ups.

```measure-prediction
route corpus
move rd_kafka_broker_update_reconnect_backoff Unknown -> Safe
move XXH32_digest_endian Unknown -> Safe
move XXH64_digest_endian Unknown -> Safe
route system
none
route slice
none
```

Pinned routes (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Corpus and twelve-home: the three
rows where the route analyses `rdkafka_broker.c` and `rdxxhash.c` (both
pinned revisions carry byte-identical bodies), possibly the two
`XXH*_digest` callers; system and slice: none.
