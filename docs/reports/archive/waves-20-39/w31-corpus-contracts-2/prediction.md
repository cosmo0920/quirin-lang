# W31-CORPUS-CONTRACTS-2 — prediction, written before the AFTER run

> **Recomputed 2026-09-13 after the baseline moved.** Everything below the
> horizontal rule is the ORIGINAL prediction, written against the wave-30
> baseline `docs/reports/w4-recursive-summaries/artifacts/` before this card
> was interrupted by a model usage limit. While it was down the user landed
> their own wave-31 card `C-GATEWAY-WAVE` (`2492825b`), which changed gateway
> analysis and **moved the corpus baseline** to
> `docs/reports/c-gateway-wave/artifacts/`. The original prediction is kept
> verbatim — a prediction is a commitment, not a draft — and the recomputation
> against the new baseline is §0 below. Neither this file nor
> `measure/runs/after` existed in its recomputed form before the recomputation
> was committed.

## 0. Recomputation against the `c-gateway-wave` baseline

`measure/predict.py` runs the same three inventories over whichever archive it
is given; `measure/prediction-comparison.txt` is the section-by-section
comparison of the two runs. The result is a finding in its own right:

**The gateway change moved the corpus totals substantially and moved nothing in
this card's candidate neighbourhood.**

| inventory | wave-30 baseline | `c-gateway-wave` baseline |
|---|---|---|
| unit histogram | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** |
| projected functions | 4,834 — 150 / 84 / 4,600 | 4,834 — **146 / 85 / 4,603** |
| `leak` / `unknown_alias` / `unsupported` / `double_release` | 266 / 4,344 / 49,760 / — | **272 / 4,208 / 50,155 / 1** |
| the 4 candidate leaves naming the two declared callees | 4, listed below | **byte-identical, same lines, same origins** |
| every leaf of the 4 carrying functions (127 lines) | — | **byte-identical** |
| the 6 sibling-callee leaves behind `W31-SIZE-SIBLINGS-REMAINDER` | 2 + 1 + 3 | **byte-identical** |

So the original prediction's candidate set, its per-site reasoning and its
per-site expectations all carry over **unchanged**, and the two predictions
**agree**: the same four uncontracted-call leaves, the same two "chancy actual"
sites (L2280, L19957), the same third call site at `:3118` whose L19962/L19963
refusals are argument-expression refusals rather than callee leaves.

The predicted movement is therefore the same, restated against the new totals:

* `unsupported` **50,155 →** between 50,151 (−4) and 50,153 (−2), by item 1.
* `leak` **272 unmoved**, `unknown_alias` **4,208** small-and-attributed,
  `double_release` **1 unmoved** — items 2 and 3, restated on the new pins.
  The `double_release` row is `C-GATEWAY-WAVE`'s own provisional
  `rd_kafka_jwt_b64_decode_payload` finding under its named follow-on
  `C-GATEWAY-CONSTANT-DO-WHILE`; this card declares nothing in librdkafka and
  predicts no interaction with it.
* Items 4, 5 and 6 unchanged: zero status moves of either kind, zero
  `SummaryConflict`, `contracts.json` 19 → 21 and 41 → 43.

One prediction is **added**, because the ground moved: the original run
observed a fifth moved leaf (L19962, the chained-actual argument-expression
refusal at `:3118`) as an attributed deviation in the deciding direction. That
observation was made under the *old* analysis. It is **not** promoted to a
prediction here — the honest statement is that the same mechanism is expected
to apply, because the leaf's text, line and origin are byte-identical in the
new baseline, so the range above is stated as −4 to −2 **on the same grounds as
the original**, and a −5 result would again be an attributed deviation in the
deciding direction rather than an expectation met.

---

Committed before `measure/runs/after` exists. The BEFORE run has started and
its byte-identity against `docs/reports/w4-recursive-summaries/artifacts/` will
be checked before any analysis input moves; the manifest edit selecting
`chunkio/chunk_sizes.qcore` and `fluent_bit/input_chunk_total_size.qcore` has
not yet been applied.

## The candidate set, bounded from the baseline

The baseline (`docs/reports/w4-recursive-summaries/artifacts/diagnostics.json`)
carries exactly **4** `unsupported` leaves naming the two callees this card
declares, in the two corpus units that call them:

* `call to uncontracted function \`cio_chunk_get_content_size\`` — 2 leaves:
  * `chunkio/cio_stream.c`, `cio_stream_size_chunks_up` (`Unknown`), L2280
    (origin `cio_stream.c:268`) — the actual is `ch`, a local bound by the
    refused `mk_list_entry` cast at L2279, so its identity is expected
    **unresolved** at the admitted call.
  * `fluent-bit-chunk/flb_input_chunk.c`, `flb_input_chunk_map` (`Unknown`),
    L18822 (origin `flb_input_chunk.c:1606`) — the actual is `chunk`, the
    function's own formal parameter (`void *chunk`), the exact shape all
    thirteen of card W29-CORPUS-CONTRACTS' sites had; clean removal expected.
    (That card's follow-on text described this receiver as the chained
    `ic->chunk`; the pinned source at `:1606` reads
    `bytes = cio_chunk_get_content_size(chunk);` — the formal itself.)
* `call to uncontracted function \`flb_input_chunk_total_size\`` — 2 leaves,
  both in `fluent-bit-chunk/flb_input_chunk.c`:
  * `flb_input_chunk_set_limits` (`Unknown`), L19889 (origin `:2974`) — the
    actual is `in`, the function's own formal; clean removal expected.
  * `flb_input_chunk_set_up_down` (`Unknown`), L19957 (origin `:3108`) — the
    actual is `in`, a local bound from the member read `in = ic->in;` at
    `:3105` (no leaf of its own), so whether the identity resolves at the
    admitted call is **not predicted**; both outcomes are allowed for.

Every carrying function is `Unknown` and carries other refusals besides; the
third `flb_input_chunk_total_size` call site (`:3118`, chained actual
`ic->in`) carries **no** leaf naming the callee — its L19962/L19963 refusals
are indirect-access refusals about the argument expression and are expected to
stay.

## Predicted movement

1. The 4 uncontracted-call leaves go to **0** in their current form. At the
   two sites whose actual may not resolve (L2280, L19957), the leaf may be
   **replaced** at the same line by the reaching-state refusal the call rule
   reports next — the `unknown_alias: unresolved resource identity` shape
   cards CONSTRUCTOR-RESIDUALS and AOM-BINDING-POISON recorded — or the site
   may be marked through the unresolved-effect mechanism without a new leaf.
   Net `unsupported` **−4 ± replacements** (not better than −4, not worse
   than −2).
2. `unknown_alias` may move by a small number in either direction: each
   admitted call was previously a refused statement, and a refused statement
   poisons bindings (bounded by constraint 10a); an admitted one does not, so
   a downstream identity may now resolve (down), while replacement leaves
   (1.) push it up. No prediction beyond "small, attributed leaf by leaf".
3. **Leak pin 266 unmoved.** The declared bodies are read-only size queries:
   no acquisition, no release, no custody transfer. If the pin moves anyway —
   through an unmasked stale-scalar fact or a spared binding reaching
   `report_leaks` — that is a finding to attribute under constraint 7's full
   narrative, not to absorb.
4. **Zero unit status moves, zero function status moves.** A `Safe` appearing
   anywhere is a red flag to investigate, not a win to report.
5. **Zero `SummaryConflict` movements.** Both declared functions are defined
   in corpus units (`chunkio/cio_chunk.c`, `fluent-bit-chunk/flb_input_chunk.c`)
   whose bodies contain constructs the projection cannot model, so W4 derives
   no summary for either and the constraint-8 mode gate compares nothing; the
   ABI gate reads declarations that match the module spellings
   (`ssize_t` → `isize`, `size_t` → `usize`, one object-pointer level each).
6. `contracts.json`: sources 19 → 21, v1 declarations 41 → 43,
   audit_selection unchanged at 5. `chunk-contracts.json`, `source.json`,
   `source-effects.json`: byte-identical between the two runs.
