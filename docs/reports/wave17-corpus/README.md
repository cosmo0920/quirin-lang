# Wave-17 corpus consolidation — one fresh measurement, deliberately re-pinned

**Date:** 2026-09-10. **Tree:** main `ad58360` (all eight wave-17 cards merged,
including the derived-fact reconciliation after LOADER-ABI-MODES).
Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

A pinned Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` clone and a
forced-Libevent Monkey were prepared fresh in this worktree with
`scripts/prepare_cycle19_clang_audit.py` (explicit `/usr/bin/clang`, Ubuntu
clang 21.1.8); `third_party/libevent` at the pinned `d82464a2`. All five
provider packages were **rebuilt and verified at this tree** — schemas
**14.0.0**, ChunkIO **0.13.0** — before the run. Canonical pinned route, exit 0,
**zero gateway failures** in every library. Log: `corpus-run.log.gz`. Card
CORPUS-REPIN's recorded deviation 2 is reproduced (the prepared CMake build
lives outside the clone, so its `compile_commands.json` was copied verbatim to
`<clone>/build/compile_commands.json`, where the pinned route reads it).

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged. Zero unit status moves, zero function status moves across all 4,834
projected functions, zero unit `reason_code` moves, and no newly surfaced
provisional Unsafe.** Nothing is promoted.

Per library: cmetrics `0/16/15`, ctraces `1/2/13` (+1 data-only), librdkafka
`0/10/81` (+3), chunkio `0/2/11`, fluent-bit-chunk `0/1/0`, monkey-libevent
`1/11/17`.

### The one number that moves: leak 271 → 272, deliberately, in the accusing direction

`fluent-bit-chunk/flb_input_chunk.c`, `flb_input_chunk_get_direct_routes`,
preprocessed line **19399**: `leak: live obligation acquired at preprocessed
line 19357`. **This is a deliberate pin movement, not a promotion**, and card
CONSTRUCTOR-RESIDUALS recorded it in advance as `CONSTRUCTOR-RESIDUALS-LEAK-271`
(its §6.4):

* **It moves in the refusing direction.** The leaf moves `Unknown` → `Unsafe`;
  the function was already `Unsafe` and the unit was already provisional
  `Unsafe`. Nothing became `Safe` at any level anywhere in the corpus.
* **It is the seventh instance of a claim the same function already published
  six times.** The body carries the shape seven times; the errno refusal at
  L19396 used to poison every pointer in the reaching state, so the release at
  L19397 could not settle and the exit at L19399 could support no claim. With
  that refusal admitted, the release settles and the existing claim fires on
  one more path.
* **It stays provisional in the documented sense.** The claim rests on
  `flb_input_chunk_destroy_direct_routes` being uncontracted; that callee very
  plausibly frees `result` and the projection cannot see it. Invariant 2 — a
  provisional `Unsafe` is never presented as an upstream defect — governs this
  leaf exactly as it governs the six that already stood.

`AGENTS.md` standing constraint 7 is updated to 272 with this record named.
`CONSTRUCTOR-RESIDUALS-LEAK-271` stays open: deciding it needs either a
contract for that callee or an unresolved-effect rule for uncontracted callees
that would *withdraw* the six, which is a downgrade no card in this wave is
scoped to make.

## Attribution — every changed leaf against the wave-16 baseline

`chunk-contracts.json` is **byte-identical** (`c4f438e4…`). The other four move,
and every leaf is attributed:

| Artifact | Movement | Attribution |
|---|---|---|
| `diagnostics.json` | 12 added / 60 removed, 21 units | **byte-identical to CONSTRUCTOR-RESIDUALS' archived AFTER** |
| `contracts.json` | 2 modules added, 1 changed | CONSTRUCTOR-RESIDUALS §7.1 |
| `source-effects.json` | 48 leaves on 3 of 96 rows | package identity (schemas 14.0.0 / ChunkIO 0.13.0) and path provenance |
| `source.json` | 17 leaves | path-literal provenance |

### The diagnostics diff, attributed by identity rather than by argument

Three of the wave's eight cards touched the gateway's decision surface, and
only one measured a non-empty corpus diff. `triangulate.py` asserts that on
committed bytes rather than arguing it (`triangulation.txt`):

```
wave-16 baseline -> CONSTRUCTOR-RESIDUALS archived AFTER: added 12, removed 60
wave-16 baseline -> composed wave-17 measurement:          added 12, removed 60
composed diff == CONSTRUCTOR-RESIDUALS' diff: True
unattributed added leaves: 0
unattributed removed leaves: 0
zero leaves attributable to CTWIN-RESIDUALS or LOADER-ABI-MODES: True
unit status moves: 0
function status moves: 0
```

The attribution is in fact stronger than leaf equality: the composed
`diagnostics.json` is **byte-identical** to CONSTRUCTOR-RESIDUALS' archived
AFTER (`412905bb…`, 11,124,610 bytes). **CTWIN-RESIDUALS** and
**LOADER-ABI-MODES** each measured all five artifacts byte-identical between
their own BEFORE and AFTER runs; both claims hold on the composed tree, because
no leaf of this measurement is attributable to either. **EXTERN-MODES**,
**STD-MODULES**, **CT40-LANE** and **GEN-CTREASONS** each predicted corpus
neutrality by construction, and no leaf here is attributable to any of them.

The 72 moved leaves are CONSTRUCTOR-RESIDUALS' four classes, re-measured here
by `census.py` (`census.txt`) and matching its §6.3 leaf for leaf:

| Δ | class |
|---|---|
| **−24** | `call to uncontracted function \`__errno_location\`` across 12 units — the declaration alone |
| **−31** | `indirect memory access` — 18 `flb_errno_print((*__errno_location ()), …)` sites and 13 `mk_list_add(&X->F, &Y->G)` sites |
| **+11** | `unresolved resource identity NAME` — **refusals replacing refusals** at 11 of those 13 `mk_list_add` sites, in the fail-closed direction |
| **−5** | cascades the two errno roots were poisoning |
| **+1** | the leak above |

`leak: 271 → 272 (+1); unknown_alias: 4399 → 4405 (+6); unsupported: 49890 →
49835 (−55)`, over 21 units — the census's own totals, and CONSTRUCTOR-RESIDUALS'.

**The composed constructor count is measured, not inferred.**
`flb_input_chunk_create` reports **6** diagnostics and stays **`Unknown`**;
`flb_input_chunk.c` keeps its provisional `Unsafe`. The six are the
`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` family and its reaching state: L19628
(`msgpack_packer_init`), L19629 (`mk_list_add`'s poisoned base), L19637 and
L19639 (`pthread_mutex_lock`/`_unlock`) and the two at L19647. Five of its six
exits Discharge, and a withdrawal experiment in that card's §4.3 pins that
removing those three statements leaves 0 diagnostics and all six exits
discharged.

### `contracts.json` — two modules added, one changed

Itemised by HYGIENE-16's `diff_contracts` (document `schema_version: 2`),
`diff-wave16-to-wave17.json.gz`:

```
sources: {added: 2, removed: 0, changed: 1, before: 12, after: 14}
  added   qcore/libc/errno.qcore              3713 bytes
  added   qcore/monkey/intrusive_list.qcore   6051 bytes
  changed qcore/fluent_bit/logging.qcore      7745 -> 9258 bytes
manifest.modules: {added: 2, removed: 0};  audit_selection.libraries: {}
keys: {}
```

Exactly CONSTRUCTOR-RESIDUALS' §7.1 diff: `__errno_location`, `mk_list_add`,
and `flb_errno_print` added to `logging.qcore` with that module's own
"deliberately left undeclared" entry for it retired in its text. Both raw keys
are published and `key_collisions` is absent, so the path normalisation stays
auditable. **The contract corpus is 69 modules**, 67 → 69 by that card's two
deliberate additions, and `compiler/tests/qcore_contract_corpus.rs` pins it.

### `source-effects.json` — 48 identity leaves, no verdict

Exactly the three `chunkio/cio_chunk.c` and `fluent-bit-chunk/flb_input_chunk.c`
rows the package-identity class touches, and the status split is unchanged at
**3 checked / 93 Unknown** with **every per-result status equal**.
`residues.py` classifies every leaf (`residues.txt`):

```
changed leaves: 48; by field: {build_digest: 6, configuration_digest: 5,
  obligation_id: 3, package_digest: 3, preprocessed_source_digest: 1,
  provider_version: 3, task_digest: 3, bounds re-keys: 24}
outside the identity/path classes: 0
per-result statuses equal: True
```

`provider_version` moves `0.12.0 → 0.13.0` on all three rows — P04-COMPOSE-
UNSUPPORTED's stated expectation, "no change from this card beyond the
package-identity residue class" — and `package_digest` carries that plus the
schemas-14.0.0 rebuild. The contract-environment digests move because
CONSTRUCTOR-RESIDUALS changed a contract module's bytes and added two more:
card G01's compile-task identity behaving as designed. **No result's verdict,
coverage shape or bounds value moved** — the `bounds` map is re-keyed by the
new obligation id and its value is byte-equal on all three rows.

### `source.json` — path-literal provenance only

All 17 changed leaves are a path string literal or the `char[N]` extent derived
from one (this worktree's clone path differs from the wave-16 worktree's),
including the recorded `compile_entry` command carrying the same prefix. Zero
callback rows moved. This is the residue class waves 9, 10 and 16 recorded; it
touches no verdict.

## Re-pin

`artifacts/` here is the current 185-unit baseline; `manifest.json` carries
each artifact's uncompressed byte length and SHA-256. The wave-16 archive is
untouched as that record and carries a supersession note. The sidecar
reason-inventory test (`scripts/tests/test_sidecar_reason_inventory.py`)
retargets here, as it did at the wave-15 and wave-16 re-pins.

`diff-wave16-to-wave17.json.gz` is the complete machine-readable diff,
`census.txt` the leaf-level census, `triangulation.txt` the attribution and
`residues.txt` the residue-class check. `triangulate.py`, `residues.py`,
`census.py` and `archive.py` are the read-only tools that produced them.
`triangulate.py` and `residues.py` read only archived artifact sets, so
`python3 -B docs/reports/wave17-corpus/triangulate.py` and
`python3 -B docs/reports/wave17-corpus/residues.py` reproduce their transcripts
byte-identically from committed bytes — the attribution is checkable without
re-running the audit.

## Ledger reconciliation

`docs/reviews/r0_3/acceptance_matrix.json` was regenerated whole-file through
its owned generator with this wave's measured gate
(`build_matrix.py . --gate docs/reports/wave17-corpus/gate.json`), after the
hand-maintained sections were edited in place. `named_followups` moves 26 → 27:
`STD-MODULES-O` is new; `EXTERN-CALL-F`, `P04-NEXT-O`, `CT-CTWIN-O`,
`C02-P-ALIAS-O`, `PROD-CONTRACTS-O`, `LOADER-ABI-O`, `T03-CT-REST`,
`P02-FLAKE`, `D02-BUCKETS-STALE` and `DIFF-TOOL-CONTRACTS` absorb this wave's
closures and new residuals; four of them are retitled because the residual
their title named has closed. `versions.qcore_schemas_version_cause` is
extended to 14.0.0 by hand, and `corpus.cycle19_raw_units` re-points at this
baseline with the leak movement stated in its own note.

`ct_conformance` regenerates at **33 mapped / 17 unmapped** and this wave's
derived `newly_decidable_not_mapped_reasons` map is emitted by the generator
itself — GEN-CTREASONS closed `D02-GEN-CTREASONS`, so no carry-forward step
runs and `readd_ct_reasons.py` self-retires.

### One pre-existing divergence recorded, not repaired

`corpus.contract_modules` derives from disk and now reads **69**, while
`corpus.contract_modules_per_directory` and `corpus.contract_clause_pin` derive
verbatim from card B00's **frozen** ledger and still describe the 59-module
checkpoint (their sum is 59, and the clause note still says "walks all 59
modules"). That is standing constraint 21 working as designed — the archive is
not edited — but the matrix publishes a snapshot beside a live count with
nothing saying so. Recorded as `D02-B00-LEDGER-SNAPSHOT` inside
`D02-BUCKETS-STALE`; repairing it means publishing provenance beside the frozen
values, not editing the archive.

## Gate at this checkpoint

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2,902 passed, 0 failed, 8 ignored** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --no-fail-fast` | **122 passed, 0 failed** (see the note below) |
| `python3 -m unittest discover -s scripts/tests` | **186 tests, OK** |
| `python3 scripts/check_lean_audits.py` | **77 audits**, no new axiom |
| `scripts/ct_stage_summary.py . --check` | exit 0, 33 mapped / 17 unmapped |
| `cargo fmt --all -- --check` / `cargo clippy --all-targets -- -D warnings` | clean, both workspaces |
| both reevaluation examples | exit 0 |
| all five provider packages | rebuilt and verified at schemas 14.0.0 / ChunkIO 0.13.0 |
| 185-unit pinned audit | exit 0, zero gateway failures |

**One environment-class failure, reported honestly.** The first providers run
failed `real_packages_preserve_unknown_and_reject_mutations` with
`Os { code: 26, kind: ExecutableFileBusy }` — the test copies and mutates a
provider executable while a sibling test in the same binary is executing it.
It passes in isolation (6/6 in 62.6 s) and passed the `--no-fail-fast` rerun.
No assertion is weakened; it is recorded beside the three existing load-flakes
in `P02-FLAKE`. Neither `vp04_vp07_vp08` nor either confined-fixture test was
hit at this gate.
