# CELL-PROTOCOL-RELEASE-WINDOW-FRAMES prediction

Committed before any implementation, on base `c8c7edf21` (origin/main). The
rows are relisted at `baseline:c8c7edf21` (`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`
in the main checkout).

> **USER DECISION 2026-11-04** (verbatim, on the entry's options (a) callee
> frame declarations, (b) a member-actual narrowing, (c) keep the refusal):
> "(a) Callee frames".

## What the card will change

1. **A callee frame clause** (grammar G05k, new; contract specification
   C04.6.4, new). Hosted beside a record's `quant cell` protocol:
   `quant cell CALLEE: TAG |-> #untouched;` states that `CALLEE`, with
   everything it calls, neither reads nor writes any member a declared
   protocol of `struct TAG` names (its cells and its guards), on every object
   of that type, however the callee reaches it (`container_of` included).
2. **Its reading** (gateway rule 38g, new). At a contracted call that may
   reach a record whose protocol the path does not keep (the release window:
   an end closed, its sentinel not stored), the call no longer refuses
   `descriptor-cell-protocol-broken` where the callee's frame over the
   record's type stands:
   - **body-checked** in the analysed unit (the unit defines the callee's
     body and every body it calls, and none names a protocol member of the
     record, or reads the record whole, or reinterprets a pointer, or calls
     out of the unit): the fact passes the call unchanged;
   - **declared** only (no body here, or one the check cannot read): the call
     is held for human review (`awaiting-human-review`, `Pending`), never
     `Safe`, and the fact passes the call unchanged for the rest of the path;
   - **refuted** (a checked body names a protocol member): the call refuses
     (`descriptor-cell-frame-refuted`);
   - **none**: the call refuses as today.
   A record that keeps its protocol at the call is read as today. Lean parity
   in a new module.
3. **Declarations.** On the existing host `flb_config_set_program_name`
   (`fluent_bit/teardown_destroy.qv`), re-pinned at the three trees:
   `mk_list_del`'s frame over `flb_output_instance` and `flb_input_instance`,
   the window's only contracted callees. Its body (`mk_list.h:139-150`,
   `__mk_list_del` and `mk_list_del`) is defined in every unit that calls it
   and touches only `struct mk_list`'s `prev` / `next`, so the frame is
   body-checked there. The window's other callees
   (`flb_output_free_properties`, `flb_storage_input_destroy`,
   `flb_tp_destroy`, `flb_processor_destroy`, ...) are uncontracted on these
   routes: their calls refuse as uncontracted whatever a frame says, so no
   frame is declared for them in production (fixtures cover them).

## Rows the change can reach

Every `descriptor-cell-protocol-broken` leaf at the baseline (system and
slice; the corpus has none):

| Route | Row (units) | Baseline | Predicted |
|---|---|---|---|
| system | `flb_output_instance_destroy` (×2) | Unknown, 26 | **Unknown, 25.** `protocol-broken` at :631 (`mk_list_del(&ins->_head)`) goes: `mk_list_del`'s frame is body-checked in the unit. The unlink refusal at :631 and every other leaf stay. |
| slice | `flb_input_instance_destroy` (×2) | Unknown, 61 | **Unknown, 59.** `protocol-broken` at :1330 (`mk_list_del(&collector->_head)`) and :1340 (`mk_list_del(&ins->_head)`) go, for the same reason. |
| slice | `input_instance_channel_events_init` (×2) | Unknown, 8 | **Unchanged.** :1387's `protocol-broken` is the creator's failure edge (`flb_error(…, flb_input_name(ins))`), whose callees state no frame. |
| slice | `input_thread_instance_destroy` (×2) | Unknown, 7 | **Unchanged.** Its window's one call, `flb_tp_destroy(thi->tp)` (:225), is uncontracted. |

- **No `Safe` move.** Every reached row keeps leaves of its own (uncontracted
  calls, the access gate, unresolved identities).
- **No `Pending` move.** No production frame is declared-only where it is
  read: `mk_list_del`'s body is in every reading unit.
- **No `Unsafe` move.** Rule 38g never accuses.
- The host's contract text changes (it gains two frame clauses), so a row
  depending on it may show a changed contract or assumptions digest without a
  leaf change.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Where this could be wrong.** If Clang's unit index holds more than one
`mk_list_del` definition, or the body check meets a construct it does not
read in `mk_list.h`'s two bodies, the frame stands declared only and the two
call sites are held (`awaiting-human-review`) instead of refusing: still no
status move, since every row keeps other `Unknown` leaves.
