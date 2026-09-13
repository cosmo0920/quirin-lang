# W31-CORPUS-CONTRACTS-2 — prediction, written before the AFTER run

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
