# REFUSED-STORE-OF-CALL-RESULT-ESCAPE: hypothesis refuted

Round 26 implementation card (USER DECISION 2026-11-07, verbatim: "Register
four as implementation cards"). Base `82ec0e80b` (integrate/r6: origin/main
`532e03c97` plus the CURRENT-UNSAFE-TRIAGE records). The card was to confirm,
with the `QCORE_LEAK_WITNESS` census, that `cmt_map_metric_get`'s provisional
Unsafe leak of `calloc` at `cmt_map.c:210` (stored into
`metric->hist_buckets` in `metric_prepare_storage`) comes from an acquisition
result stored through a refused destination, which the refused hand-on rule
cannot withdraw. Only then would it add a rule.

**The census refutes the hypothesis.** The leak is not a custody-projection
leak at all. It is the source-validated counterexample that
CMT-GETTER-OWNERSHIP-CLOSURE appends after ordinary function analysis
(`legacy_conditional_embedded_storage::diagnose`, `legacy_c.rs:3542`). That
card validated a reachable hash-zero path natively
([its report](../cmt-getter-ownership-closure/README.md)). No rule was
implemented and no analyzer, contract or Lean code changed. Every Unsafe stays
provisional. Nothing is presented or filed as an upstream defect.

## 1. Census

Two runs of the census binary, built at the base with
`--features qcore-c-gateway/indirect-census`, both over the unit's own
preprocessed text.

* **Token level** ([evidence/token-census.json](evidence/token-census.json)).
  `analyze_legacy_c_translation_unit` ran over `cmt_map.c -E -P` with
  `cycle19-resource-contracts.json`, which is the manifest the user's run read
  (`profile_manifest`). The text was written by
  [scripts/preprocess_unit.py](scripts/preprocess_unit.py) from the run's own
  compile record. Its SHA-256 `3416ae7a…cbcaf` equals the report row's
  `source_identity`, so this is the analysed text: 5,916 lines, and line 5639
  is `metric->hist_buckets = calloc(…)`. The census did not hit
  CURRENT-CENSUS-UNIT-STREAM-GAP. The harness was a throwaway integration
  test, not committed, run with `EXPLORE_SOURCE`, `EXPLORE_ENTRY`,
  `QCORE_CLANG=/usr/lib/llvm-21/bin/clang` and `QCORE_LEAK_WITNESS` set:

  ```rust
  let contracts = load_resource_contract_manifest(&root.join("cycle19-resource-contracts.json"))?;
  let summary = analyze_legacy_c_translation_unit(&source, &entry, &contracts)?;
  for (name, function) in &summary.functions {
      println!("== {name} {:?}", function.status);
      for d in &function.diagnostics { println!("   {} {:?}", d.line, d.kind); }
  }
  ```

  [scripts/token_census_record.py](scripts/token_census_record.py) turned its
  output into the evidence file.
* **Route level** ([evidence/route-census/](evidence/route-census/)).
  `quirin-audit current` ran on the user's 046accf6 inputs with a one-unit
  selection ([cmt-map-selection.json](evidence/route-census/cmt-map-selection.json)),
  through an audit slot. ROUTE-CENSUS-RESULT

| Probe | `cmt_map_metric_get` | Witness records |
|---|---|---:|
| Report row, user's run | Unsafe: `leak` line 5639, acquired at `cmt_map.c:210`; `call to uncontracted function map_metric_get_unlocked` | — |
| Token census | Unsafe: `Leak { acquisition_line: 5639 }` **at line 5639**, plus the same uncontracted call and a CG30 refusal (no typed AST on this entry point) | **0** |
| Positive control (`char *p = calloc(1, 1);` leaked at `return 0;`) | `w` Unsafe: leak at exit line 7, acquired at line 3 | 4 |
| Guard mutation ([evidence/guard-mutation.json](evidence/guard-mutation.json)): `if (map->metric_static_set)` in `cmt_map_destroy` → `if (1)` | **Unknown**, no leak; its other two diagnostics unchanged | 0 |

`record` writes one witness for every `Leak` that a CFG's custody fixed point
reports, and it opens its file on the first record. The unit produced none,
while the control did, so no CFG analysis in `cmt_map.c` reported this leak.
The leak's line is also its acquisition line. `report_leaks` publishes the
exit node's line, and `metric_prepare_storage`'s allocation is not an exit of
`cmt_map_metric_get`. The adapter, by contrast, emits exactly
`{function: "cmt_map_metric_get", line: L, Leak {acquisition_line: L}}`, where
`L` is the line of the `calloc` token in `metric_prepare_storage`. The
mutation removes one edge the adapter checks (the flag that guards
`metric_release_storage(&map->metric)`). That edge sits in `cmt_map_destroy`,
whose own analysis is separate from the getter's. Even so, the getter's leak
disappears and `cmt_map_metric_get` is the only function in the unit whose
diagnostics change. The custody projection cannot explain that, because an
internal call is resource-neutral at the call site. The adapter's
fail-closed decline does explain it.

## 2. The real cause

CMT-GETTER-OWNERSHIP-CLOSURE (STATUS: "validates the pinned labeled hash-zero
counterexample and moves exactly corpus `cmt_map_metric_get` Unknown to
provisional Unsafe") found the following on the pinned corpus source:

1. a NUL-free metric name whose XXH3 digest is zero;
2. `metric_hash_lookup` selects the embedded `map->metric` for that digest;
3. `metric_prepare_storage` allocates its histogram storage (`:210`);
4. the labeled path never sets `metric_static_set`;
5. `cmt_map_destroy` frees the embedded storage only under that flag.

A native witness observed that path. The adapter fails closed: it emits the
leak only after the whole unit re-establishes the XXH3 model, the secret
prefix, the selection, the allocation, the list, index and cache writes, the
flag condition, the lock/get/unlock wrapper and the guarded cleanup. On
046accf6 it emitted, so the unit's source still has that shape. The guard is
at `cmt_map.c:509-513` of 046accf6 (read at the run's checkout).

What the triage inferred does not occur in the projection. A statement whose
destination is refused (`metric->hist_buckets = calloc(…)` → `indirect memory
access is outside the resource projection`) is classified as
`LegacyInstruction::Unknown`. `instruction_produced_sites` gives an `Unknown`
node no site, so no live, unbound site exists for a rule to mark.
`metric_prepare_storage` itself reports no leak in all three probes.

CURRENT-UNSAFE-TRIAGE's row for `cmt_map.c` therefore reads, corrected here
beside the frozen report (constraint 21): **validated counterexample
(CMT-GETTER-OWNERSHIP-CLOSURE), provisional Unsafe**, not a modelling defect.
Its totals become 8 modelling defects in 2 units (scram 1, snappy 7). The
other 33 findings in 6 units are true positives or validated, and all stay
provisional Unsafe.

## 3. Options (a user-owned choice; the card stops here)

The acceptance asked for "cmt_map.c leave Unsafe with no Safe move". That
holds only if a modelling defect causes the finding, and none does.

* **A (recommended): retire the card as refuted.** No rule. `cmt_map_metric_get`
  stays provisional Unsafe, as the getter card left it. The
  [amendments](integration-amendments.json) do this.
* **B: add the rule anyway, as a latent guard.** A transfer-refused store
  whose own instruction minted a site would mark that site unresolved. Its
  population is zero:
  * no leak leaf at `baseline:7b726587b` has that shape (corpus 41, system 0,
    slice 0, listed in [PREDICTION.md](PREDICTION.md));
  * a classifier-refused store mints no site (§2).
  It would move nothing, `cmt_map.c` would stay Unsafe, and it would be a rule
  for a cause no census confirmed. Not recommended.
* **C: withdraw the getter counterexample.** This would move `cmt_map.c`
  Unsafe → Unknown. It reverses a validated, natively witnessed finding of
  another card, which fail-closed does not license (`Unsafe` only for a
  validated reachable violation, kept as such). It is the user's call alone.
  Not recommended.

Observation, not registered: the 046accf6 row rests on the adapter's
source-level re-validation. The native hash-zero witness was run at the corpus
pin. Mapping and native observation are separate evidence (AGENTS.md); a
046accf6 native rerun would be the matching observation, if wanted.

## 4. Records

* [PREDICTION.md](PREDICTION.md): no semantic change, so `none` on every
  route and no change to the current run.
* [integration-amendments.json](integration-amendments.json) retires
  REFUSED-STORE-OF-CALL-RESULT-ESCAPE as refuted.
* [gate.json](gate.json): the scoped card gate.
* Shared files: none. The card commits only this directory.
