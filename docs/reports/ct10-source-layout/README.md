# CT10 source layout and concurrent field writes

Date: 2026-09-19. Card: `CT10-SOURCE-LAYOUT`. Branch checkpoint after the
standing-gate attempt and its focused manifest repair.

## 1. Result

This card makes the CT10 mechanism source-reachable without the byte encoding
that `CT10-BYTE-ENCODING-UNSOUND` rejected. A `.qcore` record can declare either
an addressable field

```text
left: field<BYTE_OFFSET, BYTE_SIZE>
```

or a bit-field

```text
left: bits<UNIT, BIT_OFFSET, BIT_WIDTH, BYTE_OFFSET, BYTE_SIZE>
```

and actual task spawns can capture those fields with `write object.field`.
The route derives a `LayoutContract`, one record allocation, one field handle
per worker, and every pairwise `AssertDisjoint` question. It then calls the
existing, unedited `DisjointFootprintsV1` checker and independently rechecks
the returned certificate.

The certificate is property-only. It answers field-footprint independence on
the path where every modeled spawn starts and every join returns. The checked
payload publishes `BitfieldTaskAssumption::SpawnAndJoinSucceed`; it does not run
the scoped-task ledger and therefore does not prove recovery or custody on a
propagated spawn or join failure. The CLI prints this exact boundary instead of
reporting the workers as generically checked functions.

The flagship has two workers live together in one lexical scope. Both are
spawned before either is joined, each worker body is exactly `write PARAM;
return;`, and their captures name adjacent, non-overlapping bit ranges of one
storage unit. The checker refuses at `Q661 SharedBitfieldUnit`. The bit ranges
are retained for reporting but never enter the disjointness decision, and
`DerivedFootprint::export` continues to return `SharedMemoryLocation` for them.

Two controls use the identical task shape. Distinct ordinary addressable fields
are admitted, and bit-fields in distinct storage units with disjoint bytes are
admitted. Therefore the refusal is attributable to shared storage-unit identity,
not to task syntax, field names, byte overlap or a blanket refusal of records.

## 2. The bounded source grammar is total

The route uses the ordinary parser, elaborator and structured task-expression
inventory. It does not re-lex source text. Selection requires both an explicit
layout spelling inside a parsed record and a parsed `write` capture of a direct
field, and occurs before the generic task route. A malformed selected layout
stays on this route and refuses.

The first revision is intentionally narrow. It admits exactly:

* one `std.concurrency` import and one `concurrency checked;` declaration;
* explicit-layout records only;
* one entry returning `unit`, with exactly `{spawn, join, mut}`;
* one object declaration before one writing thread scope, then `return;`;
* two or more spawn statements, all before any join, followed by one join per
  handle in spawn order;
* one direct `write OBJECT.FIELD` capture per spawn; and
* used worker functions with one `i32` parameter, exactly effect `{mut}`, and
  body `write PARAM; return;`.

Every unrelated declaration, import, entry statement, scope statement,
capture, worker statement or effect refuses at `Q600`. Name resolution and the
module-import gate still run first, so a use before the object declaration is
`Q013` and a missing concurrency import is `Q015`. Adversarial tests drive each
boundary: missing worker write, missing or duplicate join, duplicate write
captures, a spawn after a join, a shadowing spawn binding, an entry parameter,
an extra unsafe statement, use-before-object, an unrelated declaration,
malformed layout syntax and an inconsistent declared bit extent.

This exact grammar is the current scope claim. It is not a general record/task
composition.

## 3. No new reserved word and selector census

The syntax reuses `type`, `let`, `thread`, `write`, `spawn` and `join`. `bits`
and `field` are contextual only after a colon inside a record declaration and
are never added to the reserved-word table.

The constraint-14 census was:

```sh
rg -n --glob '*.qcore' --glob '*.qc' 'bits\s*<|field\s*<' .
rg -n 'bits<|field<' crates compiler examples contract
```

The first command found only this card's twelve fixture lines (the focused
fixtures and their CT10 harness copies). The second found 21 matches: those
lines plus this module's grammar documentation/diagnostic and test mutations.
No pre-existing source form selects the new route. Ordinary record
declarations remain on `source.record.v1`; ordinary task field captures still
reach the generic task route unless the record carries this explicit layout.

## 4. Lean parity and trust boundary

`QCore.BitfieldSourceConcurrency` models the two-worker kernel:

* simultaneous writes to any two fields of one bit-field unit refuse,
  universally over bit ranges and byte ranges;
* ordinary disjoint fields admit;
* distinct bit-field units with disjoint bytes admit; and
* sequentialized spawn/join windows are not a concurrent question.

`RustCorrespondence` is an explicit unproved premise that a Rust decision is
the modeled source decision. The conditional theorem uses it rather than
presenting the Lean transcription as extraction. The target ABI truth of the
declared layout is also a premise, matching standing constraint 28. There is no
Rust extraction, C ABI derivation or whole-C theorem.

`SuccessfulTaskWindow` mirrors the Rust payload's explicit
all-spawns-and-joins-succeed premise. It prevents the abstract overlapping
worker intervals from being read as a failure-path task-lifetime proof.

`AuditBitfieldSourceConcurrency.lean` registers five public theorems. The full
audit script passed 144 audits; all five new theorems use only `propext`.
`QCore.lean` imports the module, so `AuditAll.lean` also checks those theorems
inside the compiled project closure rather than only through the dedicated audit.

## 5. Validation at this checkpoint

All commands used
`CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` where Cargo was involved.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned libevent `d82464a…` initialized |
| `cargo test -p qcore-compiler --test bitfield_concurrency --no-fail-fast` | 0 | 20 passed, 0 failed |
| `cargo clippy -p qcore-compiler --test bitfield_concurrency -- -D warnings` | 0 | clean |
| `cargo fmt --all` then `cargo fmt --all -- --check` | 0 | clean |
| `python3 -B scripts/check_lean_audits.py` | 0 | Lean build 191 jobs; 144 audits passed; `AuditAll` checked 9,322 public theorems |
| `git diff --check` | 0 | clean including this report |

The standing gate was also run with private logs under
`private/ct10-final-gate/`. Its workspace Rust step exited 101 after 4,774
passed, one failed and ten ignored tests across 242 result summaries. The sole
failure was `ct_manifest_and_fixture_set_agree_and_mapped_cases_execute`:
the newly introduced footprint-stage cells on the still-unmapped CT24 row were
incorrectly `not_applicable` rather than `unassessed`. That failed run is
preserved in `workspace.log` and `workspace.exit`; it is not presented as a
clean standing gate.

The defect was confined to manifest metadata. CT24 and CT41 now leave every
stage unassessed, while mapped cases outside CT10 mark the two structurally
absent footprint stages not applicable. After that correction, the complete
`ct_corpus` target passed all six tests, the stage checker reported 48 mapped
and two unmapped cards, and the full Python suite passed all 305 tests. The
acceptance matrix was regenerated without `--gate`, so its previous-gate fields
remain explicitly carried rather than being attributed to this red Rust run.

All later steps in the original standing-gate attempt completed successfully:
format, full clippy, probes, provider follow-ups, the full 144-audit Lean check,
the 305-test Python suite and Clang recording each exited zero. The recorded
compiler is Ubuntu clang 21.1.8 (6ubuntu1). The combined integration branch
still requires a clean full standing gate after the card branches are merged.

## 6. Limitations

* The route is check-only and property-only. It emits no C, produces no
  reference execution, and runs no scoped-task ledger. Spawn/join failure-path
  custody remains unproved under the explicit successful-window premise.
* The declaration is trusted target-layout input. This card validates internal
  consistency; it does not derive offsets or allocation units from Clang.
* Only the exact bounded grammar in section 2 is modeled. Multiple objects,
  nested scopes, conditionals, mixed captures, richer field types and arbitrary
  worker bodies refuse.
* CT10's proof stage remains unassessed. The audited Lean rule is not a
  case-bound execution of the fixture.

## 7. Integrator reconciliation list

1. CT10 is mapped as denominator growth on the dedicated
   `bitfield_concurrency` route. Its executed stages are `parse`, `elaborate`,
   `footprint_lowering` and `footprint_check`; no `task_ledger_check` claim is
   made. Preserve that vocabulary at integration.
2. Regenerate derived acceptance-matrix fields after the new Lean audit is
   integrated. Do not hand-edit the audit count.
3. Retire `CT10-BYTE-ENCODING-UNSOUND`: the mapped artifact derives
   `FieldStorage::BitfieldUnit` and never treats a bit-field as a byte lease.
4. Resolve `CT10-REASON-ROW` as superseded by the mapping. No lane evidence
   record held an `unmapped_reasons` row for CT10, so the new stage record
   correctly declares no retirement of a frozen row.
5. Add the standing constraint/increment/status changes only at integration.

Proposed exact standing text: “`source.bitfield-concurrency.v1` derives one
declared record layout and actual `write OBJECT.FIELD` spawn captures into
`DisjointFootprintsV1`. Its certificate proves only field-footprint
independence on the explicit `SpawnAndJoinSucceed` premise; it runs no
scoped-task ledger, emits no C and publishes no reference/native execution.
Adjacent bit-fields in one declared storage unit refuse at `Q661`; ordinary
disjoint fields and distinct bit-field units are admitted controls. Declared
target layout and Rust-to-Lean correspondence remain premises.”

Proposed §L21/Q600 clarification: all bounded source-shape, worker-body,
capture, join and successful-window grammar refusals remain `Q600` under the
Concurrency pass. `Q661` remains the unedited `SharedBitfieldUnit` model
refusal; this card allocates no code.

Proposed Q661 amendment: “On `source.bitfield-concurrency.v1`, `Q661` is
reachable from two actual simultaneous worker write captures whose fields have
the same declared bit-field storage-unit identity. Bit offsets and widths are
retained but cannot establish disjointness.”
