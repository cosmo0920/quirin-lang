# Wave-16 corpus consolidation — one fresh measurement, deliberately re-pinned

**Date:** 2026-09-10. **Tree:** main `79c1cb8` (all seven wave-16 cards
merged, including LOADER-ABI's composed-tree reconciliation `c7e20ed`).
Integrator-run, closing the wave under hard invariants 3 and 5.

## Provenance

A pinned Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` clone and a
forced-Libevent Monkey were prepared fresh in this worktree with
`scripts/prepare_cycle19_clang_audit.py` (explicit `/usr/bin/clang`, Ubuntu
clang 21.1.8); `third_party/libevent` at the pinned `d82464a2`. All five
provider packages were **rebuilt and verified at this tree** — schemas
**13.0.0**, ChunkIO **0.12.0** — before the run. Canonical pinned route, exit 0,
**zero gateway failures** in every library. Log: `corpus-run.log.gz`. Card
CORPUS-REPIN's recorded deviation 2 is reproduced (the prepared CMake build
lives outside the clone, so its `compile_commands.json` was copied verbatim to
`<clone>/build/compile_commands.json`, where the pinned route reads it).

## Headline

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged. Zero unit status moves, zero function status moves across all 4,834
projected functions, zero unit `reason_code` moves, no newly surfaced
provisional Unsafe, and leak unchanged at 271.** Nothing is promoted.

Per library: cmetrics `0/16/15`, ctraces `1/2/13` (+1 data-only), librdkafka
`0/10/81` (+3), chunkio `0/2/11`, fluent-bit-chunk `0/1/0`, monkey-libevent
`1/11/17`.

## Attribution — every changed leaf against the wave-15 baseline

`chunk-contracts.json` is **byte-identical** (`c4f438e4…`). The other four move,
and every leaf is attributed:

| Artifact | Leaves | Attribution |
|---|---|---|
| `diagnostics.json` | 4 added / 14 removed, 4 units | **triangulated exactly** (below) |
| `contracts.json` | 1 module's retained text | LOADER-ABI §4.1 |
| `source-effects.json` | 36 leaves on 3 of 96 rows | LOADER-ABI's 33 + P04-COMPOSE-FS's 3 |
| `source.json` | 14 wrapper leaves + the `compile_entry` envelope | path-literal provenance |

### The diagnostics diff, triangulated in both directions

The two gateway cards each measured their AFTER **without** the other's diff,
so the composed tree must differ from each card's archive by exactly the other
card's leaves. `triangulate.py` asserts that rather than arguing it
(`triangulation.txt`):

```
union(CHAINED-TAIL, ALIAS-AGGREGATE) == composed measurement: True
unattributed leaves: 0
composed \ CHAINED-TAIL == ALIAS-AGGREGATE's leaves: True
composed \ ALIAS-AGGREGATE == CHAINED-TAIL's leaves: True
unit status moves: 0
```

* **NEIGHBOURS-CHAINED-TAIL** — exactly one leaf removed, zero added:
  `flb_input_chunk_create` **L19581** `indirect memory access`, the line that
  card predicted.
* **ALIAS-AGGREGATE** — 4 added / 13 removed across four units
  (`chunkio.c` +2/−2, `cio_chunk.c` +1/−1, `mk_fifo.c` +1/−1,
  `flb_input_chunk.c` −1 at L19607 plus the −8 `ic` cascade). **All four added
  leaves are refusals replacing refusals**, in the fail-closed direction.

**The composed constructor count is measured, not inferred.**
`flb_input_chunk_create` moves **21 → 11** diagnostics — ALIAS-AGGREGATE's 12
minus CHAINED-TAIL's L19581 — and stays **`Unknown`**;
`flb_input_chunk.c` keeps its provisional `Unsafe`. That composition is what
LOADER-ABI's §10.3 reconciliation predicted, and it is the number this
consolidation publishes.

### `contracts.json` — one module, verified past the recorded blind spot

The fifth artifact is itemised for the first time here, by HYGIENE-16's new
`diff_contracts` (document `schema_version: 2`), rather than by hand:

```
sources: {added: 0, removed: 0, changed: 1, before: 12, after: 12}
  changed qcore/fluent_bit/resource_memory.qcore  1423 -> 1853 bytes
manifest.modules: {added: 0, removed: 0};  audit_selection.libraries: {}
keys: {}
```

That is LOADER-ABI's `flb_sds_create_len` correction (`len: usize` → `i32`
against the pinned `const char *str, int len`), and the changed module's
after-digest `0590ec11…` equals the file on disk. Both raw keys are published
and `key_collisions` is absent, so the path normalisation stays auditable.
The **contract corpus is unchanged at 67 modules**: no module was added or
removed by this wave.

### `source-effects.json` — 36 identity leaves, no verdict

Exactly the three `checked_function_contract` rows LOADER-ABI named —
`cio_chunk_is_locked`, `cio_chunk_unlock`, `flb_input_chunk_release_lock` —
and the status split is unchanged at **3 checked / 93 Unknown**. Every leaf is
an identity digest or a key derived from one: `build_digest`,
`configuration_digest`, `obligation_id`, `task_digest`, `witness.task_digest`,
one `preprocessed_source_digest`, plus `package_digest` and
`provider_version`. **No result's verdict, coverage shape or bounds value
moved** — the `bounds` map is re-keyed by the new obligation id and its value
is byte-equal on all three rows.

LOADER-ABI measured 33 of these alone; the additional 3 are
`provider_version` `0.11.0 → 0.12.0`, P04-COMPOSE-FS's ChunkIO package bump.
33 + 3 = 36, attributed leaf for leaf. The contract-environment digests move
because §4.1 changed a contract module's bytes — card G01's compile-task
identity behaving as designed.

### `source.json` — path-literal provenance only

All 14 changed wrapper leaves are a `__FILE__` string literal and its derived
`char[N]` extent (`char[147]` → `char[150]`, this worktree's clone path being
three bytes longer), plus the recorded `compile_entry` command carrying the
same prefix. Zero callback rows moved. This is the residue class waves 9 and
10 recorded; it touches no verdict.

### The two byte-identity claims hold

**CTWIN-PUBLISH** claimed zero corpus reach — its scan's *value* conjunct
keeps the rule from firing anywhere in the pinned corpus — and **HYGIENE-16**
and **DOCS-COMPACT** claimed no corpus effect at all. No leaf in this
measurement is attributable to any of the three, in any artifact.
**P04-COMPOSE-FS** predicted "no change beyond the package-identity residue
class", and its three `provider_version` leaves are exactly that.

## Re-pin

`artifacts/` here is the current 185-unit baseline; `manifest.json` carries
each artifact's uncompressed byte length and SHA-256. The wave-15 archive is
untouched as that record and carries a supersession note. The sidecar
reason-inventory test (`scripts/tests/test_sidecar_reason_inventory.py`)
retargets here, as it did at the wave-15 re-pin.

`diff-wave15-to-wave16.json.gz` is the complete machine-readable diff,
`census.txt` the leaf-level census, `triangulation.txt` the two-direction
attribution, and `triangulate.py` / `archive.py` the read-only tools that
produced them. `triangulate.py` reads only archived artifact sets, so
`python3 -B docs/reports/wave16-corpus/triangulate.py` reproduces
`triangulation.txt` byte-identically from committed bytes — the attribution
is checkable without re-running the audit.

## Ledger reconciliation

`docs/reviews/r0_3/acceptance_matrix.json` was regenerated whole-file through
its owned generator with this wave's measured gate
(`build_matrix.py . --gate docs/reports/wave16-corpus/gate.json`), after the
hand-maintained sections were edited in place. `named_followups` moves 25 → 26:
`D02-BUCKETS-STALE` and `DIFF-TOOL-CONTRACTS` are retitled to record their
closure and carry their successors (`D02-GEN-CTREASONS`,
`DIFF-TOOL-CONTRACTS-BACKFILL`); `CT-LABELS-T00-REVERT` closes inside
`D02-BUCKETS-LABELS`; `PROD-CONTRACTS-O`, `C02-P-ALIAS-O`, `O04-ARMS-O`,
`P04-NEXT-O` and `CT-CTWIN-O` absorb this wave's closures and new residuals;
and `LOADER-ABI-O` is new. `validation` carries the measured
2,746 / 115 / 161 / 72 with `measurement_provenance: integrator_supplied`.

One recorded generator defect fired and was handled rather than absorbed:
`build_matrix.py` owns `ct_conformance` wholly and **drops**
`newly_decidable_not_mapped_reasons` on every regeneration, which is exactly
follow-on `D02-GEN-CTREASONS`. `readd_ct_reasons.py` carries the map forward
from the committed matrix and **refuses** to carry a row describing a case the
manifest reports as mapped — the same staleness HYGIENE-16 found by hand,
now caught mechanically at the one point a regeneration would lose it. The
follow-on stays open: this is a guard at the carry, not a derivation.

`scripts/ct_stage_summary.py . --check` exits 0 (32 mapped / 18 unmapped),
`compiler/tests/ct_corpus.rs` and `qcore_contract_corpus.rs` pass, and the
161 Python tests pass after every edit.
