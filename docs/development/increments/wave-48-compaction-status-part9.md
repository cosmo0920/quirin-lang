# Wave 48 compaction checkpoint: status page, part 9 of 11 (`STATUS-COMPACT`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Current status](../../STATUS.md) · [Part 8](wave-48-compaction-status-part8.md) · [Part 10](wave-48-compaction-status-part10.md)

> **Preserved checkpoint text, not a current authority.** The fenced block is
> `docs/STATUS.md` lines 419–420 at `590f7d9f1`, verbatim (2,001 bytes), part 9 of the
> whole page that `STATUS-COMPACT` replaced with a current-state page.
> `git show 590f7d9f1:docs/STATUS.md` reproduces it byte for byte, and
> [`checkpoint.py check`](../../reports/status-compact/scripts/checkpoint.py) asserts
> that the parts concatenated equal that page. Where this text and the current
> page disagree, **the current page wins**; figures here describe their recorded
> checkpoints. Relative links inside keep their original base, `docs/`.

~~~~~text
| Route selection reads no declaration name (`.qr`) | The five token-stream selectors (the `Q015` gate's token loop, `has_source_tasks`, `has_source_epoch`, `has_source_mutex`, both lexical tests of `has_scalar_calls`) skip a declaration's own name, a parameter's name, a contract clause and a library binding (`route_selection::SelectorScope`); a record's name and fields stay read (constraint 14's `AGGREGATE-SELECTOR-WORDS`), and so do types and effect and `throws` rows. A use naming a declared selector word refuses at selection (`Q020`); a `module contract;` module is never compiled on a source-only route (`Q020`). Declaration-name census (62 words × 3 modes × 2 bases × 7 positions, 2,658 rows against controls): **321 → 0** selections moved in name positions, the base's 105 kept in record-name and type positions; tracked route census 1,436 rows byte-identical; corpus, system, slice and null streams byte-identical (QR-LEXICAL-SELECTORS-DECLARATION-NAMES, Lean `QCore.RouteSelection`; [report](reports/qr-lexical-selectors-declaration-names/README.md)). |
| Route order written once (`.qr`) | `compile_elaborated_source` selects one row of `driver::source_routes::SOURCE_ROUTES` (22 rows in the former if-chain's order: name, selector, lowering entry, source-only, closure rule) and reads it for the closure refusal (`Q600`), the contract guard (`Q020`, naming the row) and the lowering; the restated guard list, the closure if-chain, the completeness check after routing and the guard in `compile_scalar_calls` are deleted (the last two unreachable by construction). A contract module that dispatches and sockets is refused naming `source.socket.v1`, not `source.event.v1` (the list had missed that arm). Tracked route census 1,436 rows and synthetic selector census 2,658 rows byte-identical; corpus, system, slice and null streams byte-identical (ROUTE-SELECTION-ORDER-RESTATED, rule 15v, Lean `QCore.RouteSelection`; [report](reports/route-selection-order-restated/README.md)). |
~~~~~
