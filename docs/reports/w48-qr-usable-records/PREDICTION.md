# W48-QR-USABLE-RECORDS: prediction

Frozen before any source change, at base `ca634dc1` (= local `main` = origin/main;
the worktree's HEAD was already that commit, so no reset or fast-forward was
needed). Taken first: the BEFORE fixture freeze
(`artifacts/fixtures-before.json`, card 0's `freeze_fixtures.py`, debug
`quirin` built at the base, 634 files — check 361 exit 0 / 273 exit 1, emit
154 / 480) and the Step-0 survey (`artifacts/survey-base.json`, 55 cells over
five routes, written by `make_survey.py` and pinned by
`compiler/tests/usable_records_survey.rs` at commit `83a96357`, which changes
no source).

## What the survey says is missing

| Operation | buffer | scalar-calls | extern | task | ffi |
|---|---|---|---|---|---|
| construct, field read | admitted | never reached (a record declaration selects the buffer route, which refuses the call, `Q051`) | refused (`Q025`) | `Q500` | admitted |
| field write | `Q051` (not a statement) | `Q051` | `Q025` | `Q500` | `Q026` |
| nested field | `Q051` (`AGGREGATE-NESTED`) | `Q051` | `Q025` | `Q500` | `Q051` (listing) |
| parameter / result | `Q051` (entry only) | `Q051` | `Q026` | `Q500` | `Q026` |
| copy / move | copy admitted; move of an owned record `Q054` | `Q051` | `Q025` | `Q500` | `Q026` |
| loop body / beside a lend | admitted for scalars; owned `Q051` (by name) | `Q051` | `Q026` / `Q025` | `Q500` | `Q026` |

Found beside it: a scalar field **read** of a record with an owned field after
`drop f;` is admitted at the base (`private` probe; the survey's
`buffer_write_released` is the write).

## Plan the prediction is about

1. **Field writes.** `RECORD.FIELD = VALUE;` on `source.buffer.v1` and on
   `source.ffi.v1` (listed fields only; an opaque one `Q051`), the value
   checked against the field's type by the route's existing construction
   typing (`Q052`/`Q053`). On a record with an owned field, a scalar-field
   access needs the record live (an IR check), so a write — and, as a named
   correction, a read — through a released record refuses (`Q054`); a whole
   write of an owned field refills a hole only (a new IR operation, custody
   rules unchanged).
2. **Nesting.** A field declared with another record type of the module, the
   declaration graph acyclic (`Q051` by name for a cycle), read and written
   through `a.b.c`; the IR's one-level `FieldPath` generalised to a path, the
   aggregate's owned fields its owned **leaves** in declaration order at every
   level; `AGGREGATE-NESTED` retired.
3. **Records across calls.** A record of scalars as a `source.scalar-calls.v1`
   parameter and result, by value (copy): a module that declares a record and
   a function besides the entry reaches that route (every such module was
   refused before). A record with an owned field across a call stays refused
   by name unless the resource-calls route's `own`/`borrow` forms take it.
4. **FFI beyond one flat entry.** `.qr` helper functions on `source.ffi.v1`
   taking and returning imported structs by value, and field writes, every
   result still carrying the unsafe-FFI assumption.
5. **Stretch** only if the above lands.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze AFTER vs BEFORE (634 files, `check` and `emit`, exit + both streams) | byte-identical but for new files; any changed existing entry is a previously refused program whose refusal message or admission changes, each attributed | 75% |
| existing tests changed | only rows pinning a lifted refusal (`AGGREGATE-NESTED`, the entry-only record refusal, the FFI route's flat-entry refusals): 3–8 rows | 60% |
| survey at the tip | write, nested, param/result cells of buffer/scalar-calls/ffi admitted; task cells unchanged (`Q500`, card 3); extern cells unchanged | 70% |
| negative controls (wrong-type write, opaque/unlisted FFI write, write through a released owner, nested path through a non-record, custody-mode mismatch, FFI record in a non-FFI file) | each refused, none admitted | 85% |
| native runs (`-O0`/`-O2` UBSan, plain `-O2`, `-Wall -Wextra -Werror -pedantic`) incl. an FFI example writing and passing an imported struct with a real C library | clean, streams equal the reference, 0 sanitizer reports | 75% |
| Lean | one new module and audit, ~15–25 theorems, no placeholder or axiom | 75% |
| corpus arbiter `resource-diagnostics.json` | identical to the base's archive, card 1's AFTER `c012e785…` (586 / 10 / 4,238) — no C-audit path is touched | 95% |
| standing gate first run | green | 50% |
| items completed | 1 (85%), 2 (60%), 3 (55% by value; 25% with owned fields), 4 (55%), 5 (15%) | |
