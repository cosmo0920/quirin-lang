# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`W36-CORPUS-CONTRACTS-3` (2026-09-18), wave 36's second arbiter — six
declarations chosen by function-cover yield, and the two places the method
broke.** Declarative: six `.qcore` modules and six manifest rows, no crate
source touched, so **one audit binary served both stages** and the only
difference between them is which modules the manifest selects. The BEFORE
reproduces `docs/reports/joined-provenance-cycles/artifacts/` byte for byte in
both verdict-bearing artifacts — the card brief named
`c-gateway-constant-do-while` instead, which was **stale by one**, checked
rather than assumed (the two archives' diagnostics and chunk-contracts
decompress identically, so the cover is the same either way).
The cover was re-derived rather than carried: **193** projected functions are
blocked *only* by `call to uncontracted function` leaves, and **122 of them
have exactly one blocker**, which is what makes a single declaration able to
move a function. Selection was by sole-blocker count and deliberately not by
leaf mass — `fprintf` (1,138 leaves) and `snprintf` (1,044) are the sole
blocker of one function and none, measured. Landed, each read line-cited from
the pinned checkout before it was declared:
`cfl/sds_lifetime.qcore` (`cfl_sds_destroy`, 8 functions),
`librdkafka/kafka_mem.qcore` (`rd_kafka_mem_free`, 4),
`cmetrics/context_lifetime.qcore` (`cmt_destroy`, 3),
`librdkafka/cjson.qcore` (`cJSON_Delete`, 3),
`monkey/logging.qcore` (`mk_print`, 3, the card's one variadic tail) and
`ctraces/context_lifetime.qcore` (`ctr_destroy`, 2) — the corpus's **first
`ctraces` family**. The nullability markers are deliberately **not uniform**:
`nullable` for `cfl_sds_destroy` (first statement `if (!s) return;`) and
`cJSON_Delete` (`while (item != NULL)` precedes every dereference, and all
three call sites *reach* that null path), `nonnull` for `cmt_destroy` and
`ctr_destroy` (no guard, first executable statement dereferences the
parameter). Prediction committed **before the AFTER run finished**: 204 leaves
and 23 functions. Measured: **204 leaves exactly**, and **20** functions, not
23. The three-way split — **20 leaf-free** (all `Unknown → Safe`), **3
exchanged for another refusal**, **0 unmoved**, so no declaration failed to
decide its own leaf and **none of the six was withdrawn** by constraint 8's
gates. The two breakages are the card's real findings. The three `cJSON_Delete`
functions are `cJSON_Delete(cJSON_DetachItemFromArray(...))`; decided leaf,
then a *new* refusal — `nested call … inside a modeled call is outside the
resource projection` — that was unreachable while the outer call was
uncontracted (`CJSON-DELETE-NESTED-DETACH`), which makes a sole-blocker cover
an **upper bound** and never a forecast (`W36-COVER-HIDDEN-BLOCKER`). And
`rd_kafka_mem_free`'s own defining function moved **`Safe → Unknown`** on the
corpus's first **`summary_conflict`**: W4 derives `Released{nonnull}`, the
declaration says `nullable`, and the declaration is the faithful one —
`rd_free(NULL)` reaches `free(NULL)`, and declaring `nonnull` to silence the
conflict would assert a precondition the body does not have, which constraint 4
forbids. That is constraint 8's gate working (reported, never silently
resolved), in the conservative direction, at a net of +4/−1 for that
declaration (`KAFKA-MEM-FREE-SUMMARY-CONFLICT`). Corpus-wide: **217 leaves
removed, 128 added, net −89**; projected functions **186 → 205 Safe**, Unsafe
**84 unchanged**, Unknown 4,564 → 4,545; **zero unit status moves**; `leak`
**272 → 272, unmoved**, so constraint 7's pin needed no re-pin at all;
`unsupported` 50,106 → 49,922, `unknown_alias` 4,214 → 4,308 — every added leaf
in the accusing direction, and the rising-alias/falling-unsupported shape is
what modelling a release *does*, not a regression. `chunk-contracts.json` is
byte-identical between the two stages. Read and **deliberately not declared**,
with the readings published: `add` (3 functions) — three distinct definitions
with three signatures, the cmetrics one `static inline` with no header
declaration for constraint 8's ABI gate to agree with (`ADD-NAME-COLLISION`);
and `memcpy`/`memset` (9 functions, 130 leaves) — both return `void *` aliasing
their first parameter and v1 has no result mode that says so, a **vocabulary
gap** rather than a judgement that they are undeclarable
(`LIBC-MEM-RESULT-ALIAS`). No Lean module is added, on
`W31-CORPUS-CONTRACTS-2`'s recorded ground and stated rather than assumed: a
grown premise set is not a semantic change, and the vocabulary, loader and
checker are all unedited. Corpus 76 → **82** modules, manifest-selected 21 →
**27**, resource-v1 declarations 43 → **49**, every constraint-5 pin updated
deliberately (`qcore_contract_corpus.rs`, `contract_loader.rs`'s
`(21, 43, 145) → (27, 49, 145)` and its explicit old-table parity rows,
`prod_contracts.rs`, `neighbours.rs`).
`docs/reports/w36-corpus-contracts-3/README.md`.

