# Wave 39 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`AGGREGATE-OWNED-FIELD` (2026-09-18).** An IR card before a route card, on
base `e878b362`. The cycle-3 ownership IR now addresses an owner by a **path**
— `ownership::Place::{Name, Field}` with `FieldPath { base, field }`, one level
— and lands the four rules constraint 15k had recorded as missing, each in the
IR and none in a route: `Compose` moves each named buffer into its field (the
source is a use after move from then on); `MoveOut` is the partial move,
leaving a hole a second read refuses; `Drop` of an aggregate releases every
live owned field **in declaration order and nothing twice**, tolerating a
hole; `NormalPathLeak` names an unreleased field by its path; and `Copy` of a
record declaring an owned field refuses whatever its fields' state. Every
existing `OwnershipOp` variant is unchanged and a `String` owner is always
`Place::Name`, so a program that composes no aggregate builds the operation
list it always built — measured over the whole tree by a two-tree census
(`docs/reports/aggregate-owned-field/README.md` §3, §6) rather than argued.
Lean parity is `QCore.AggregateOwnedField` (25 theorems, registered in
`AuditAggregateOwnedField.lean`; headline
`drop_through_releases_exactly_the_live_fields` and `…nothing_twice`), with
`QCore.SourceRecord`'s scalar-record theorems unchanged and their prose amended
to say which half they are. On the route, `source.record.v1` admits a field
declared **`bytes`** — the acquisition's own reserved word; the task route's
`Buffer` names a different resource and was measured and rejected — moved in
at construction, standing at every custody position a buffer name does
(`RECORD.FIELD[I] = V;`, `let X = RECORD.FIELD[I];`, `extent(RECORD.FIELD)`,
`drop RECORD.FIELD;`), moved out by the field read, and released by
`drop RECORD;`; a record of scalars is exactly what `AGGREGATES-V1` measured.
**Zero words reserved and no code allocated**: `Q055` is **narrowed** (a buffer
at a field refuses only at an `i32` field — the deliberate lift), `Q051`–`Q054`
widen by condition extension stated in §L21, `Q103` gains vocabulary no
sequential program reaches (`OWNED-FIELD-SEQUENTIAL-UNREACHED`), `Q100` is
untouched. Two shapes are refused by name rather than modelled:
`OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY`. Emission holds the
buffer pointer as a struct member, the designated initializer *is* the move,
and the whole-record release frees the members still held in declaration
order; constraint 15k's "no acquisition, no prelude" argument is re-derived
with the owned field enumerated (a construction moves, it does not acquire).
The published example `examples/records/owned_field.qcore` runs clean under
ASan/UBSan/LSan and matches its reference execution byte for byte
(`compiler/tests/aggregate_owned_field.rs`, 22 tests; `examples_suite`
extended). `AGGREGATE-OWNED-FIELD` retired by exact name;
`AGGREGATE-VIEW-FIELD`, `AGGREGATE-NESTED`, `AGGREGATE-TASK-CAPTURE`,
`OWNED-FIELD-LEND-WINDOW`, `OWNED-FIELD-LOOP-BODY` and
`OWNED-FIELD-SEQUENTIAL-UNREACHED` registered with discharge conditions.
Constraint 15k amended deliberately. Report:
`docs/reports/aggregate-owned-field/README.md`.
