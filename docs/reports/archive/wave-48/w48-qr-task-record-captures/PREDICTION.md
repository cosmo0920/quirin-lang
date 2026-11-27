# W48-QR-TASK-RECORD-CAPTURES: prediction

Frozen before any source change, at base `bcdc13b9` (= local `main` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). Taken first: the BEFORE fixture freeze
(`artifacts/fixtures-before.json`, card 0's `freeze_fixtures.py`, debug
`quirin` built at the base, `e72a538d…`, 694 files — check 381 exit 0 / 313
exit 1, emit 172 / 522).

## Where the base stands

Card 2's survey (`docs/reports/w48-qr-usable-records/`) and a probe at the
base agree: **every** module that spawns and declares a record type refuses at
the declaration, `Q500`, naming `AGGREGATE-TASK-CAPTURE` — a record is never
constructed, read, written, captured, passed or returned on the task route.
The transferability judgment (`qcore_core::transfer`, `QCore.Transferability`)
has no aggregate kind; the scoped-task ledger is keyed by resource identity
and admits at most one lent and one leased resource at a time.

## Plan the prediction is about

1. **The representation.** A record binding on the task route is its
   **leaves** — one task slot per scalar or owned `Buffer<T>` leaf, depth-first
   in declaration order (card 2's leaf order) — so every custody question
   about a record is the existing per-resource question about its owned
   leaves, asked of the unedited ledger. A record parameter is its leaves as
   worker parameters; a `copy`/`move` capture of a record is its leaves'
   captures. The core gains only what a leaf list cannot say: a **record
   result** (`return r;` / `return move r;`), carried through completion and
   `join` as one outcome naming every owned leaf, unpacked into leaf slots by
   one instruction; and a **refill** that is admitted exactly into a hole.
2. **The judgment.** An aggregate kind whose class is decided from its
   leaves: every leaf duplicated ⇒ duplicated (crosses by `copy`); every leaf
   sendable and some leaf moved ⇒ moved (crosses by `move`, shared under no
   whole-record protocol); any leaf that never crosses ⇒ that leaf's reason.
   A refusal names the first leaf that fails its own crossing and the
   contract it lacks. `write` is refused by name before any leaf is asked.
3. **Field captures** (`copy r.f`, `move r.buf`, `read r.buf share N`,
   `lend r.buf LOW..HIGH`) are the existing captures of one leaf. Disjoint
   lends of two fields to two workers need the per-leaf reading of the
   one-leased-resource bound; overlapping lends of one field stay refused by
   the ledger.
4. **Record operations in any task function**, a field write requiring `mut`
   in the row (card 2: a write derives `mut`); `print`/`log` of a scalar field
   on the composed route.
5. Stretch only if the above lands.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze AFTER vs BEFORE (694 files, `check` and `emit`, exit + both streams) | byte-identical except card 2's ten `task_*` survey cells, whose refusal moves: construct, read, copy, param, nested and lend admitted; `write` refused for its missing `mut`; `result` refused at the rendering of a record; `move` (a `bytes` field) and `loop` still refused at `Q500` with other words | 70% |
| existing tests changed | card 2's survey pins for the task row, `owned_field_route_reach.rs`'s named row, and the task route's record-refusal wording: 2–5 files | 60% |
| negative controls (a record with an owned field under `copy`; a moved field used before refill; two overlapping lends of one field; a read share and a write lease of one leaf; a record into another record type's parameter; a record moved while a leaf is lent; spawn failure restoring a moved record; a `write r` capture) | each refused as stated, the restoration admitted and observed | 85% |
| native runs on real threads (`-O0`/`-O2` UBSan, plain `-O2`, `-Wall -Wextra -Werror -pedantic`, TSan on the concurrent examples through the existing pthread binding) | clean, streams equal the reference, 0 sanitizer reports | 60% |
| Lean | one new module and audit, ~15–25 theorems, plus the transferability module re-decided, no placeholder or axiom | 70% |
| corpus arbiter `resource-diagnostics.json` | identical to the base's archive (card 2's AFTER `c012e785…`, 586 / 10 / 4,238) — no C-audit path is touched | 95% |
| standing gate first run | green | 45% |
| items completed | 1 (70%), 2 (60%), 3 (65%), 4 (55%), 5 (15% the FFI struct, 10% `W48-QR-OWNED-RECORD-ACROSS-CALL`) | |
