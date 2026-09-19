# CT10 source layout and concurrent field writes

Date: 2026-09-19. Card: `CT10-SOURCE-LAYOUT`. Branch checkpoint, before the
standing full gate.

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
boundary: missing worker write, missing join, a spawn after a join, an extra
unsafe statement, use-before-object, an unrelated declaration and malformed
layout syntax.

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

The first command found only this card's six fixture lines. The second found
those lines plus this module's grammar documentation/diagnostic and one test
mutation. No pre-existing source form selects the new route. Ordinary record
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

`AuditBitfieldSourceConcurrency.lean` registers five public theorems. The full
audit script passed 144 audits; all five new theorems use only `propext`.

## 5. Focused validation at this checkpoint

All commands used
`CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` where Cargo was involved.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned libevent `d82464a…` initialized |
| `cargo test -p qcore-compiler --test bitfield_concurrency --no-fail-fast` | 0 | 13 passed, 0 failed |
| `cargo clippy -p qcore-compiler --test bitfield_concurrency -- -D warnings` | 0 | clean |
| `cargo fmt --all` then `cargo fmt --all -- --check` | 0 | clean |
| `python3 -B scripts/check_lean_audits.py` | 0 | Lean build 190 jobs; 144 audits passed |
| `git diff --check` | 0 | clean including this report |

No standing full gate, probes, Python unit suite or Clang recording is claimed
at this checkpoint. The continuation agent owns those measurements and any
generated acceptance-matrix audit-count reconciliation.

## 6. Limitations

* The route is check-only. It emits no C, so there is no native or sanitizer
  observation and no claim about a compiler's bit-field ABI.
* The declaration is trusted target-layout input. This card validates internal
  consistency; it does not derive offsets or allocation units from Clang.
* Only the exact bounded grammar in section 2 is modeled. Multiple objects,
  nested scopes, conditionals, mixed captures, richer field types and arbitrary
  worker bodies refuse.
* The CT manifest remains untouched in this branch. There is deliberately no
  `compiler/tests/fixtures/ct/CT10` directory while CT10 is unmapped.

## 7. Integrator reconciliation list

1. Decide CT10 mapping after reviewing the exact bounded grammar and check-only
   boundary. If taken, add a CT10 fixture/harness mode that pins `Q661` and the
   addressable/distinct-unit controls; do not reuse a byte lease encoding.
2. Regenerate derived acceptance-matrix fields after the new Lean audit is
   integrated. Do not hand-edit the audit count.
3. Retire `CT10-BYTE-ENCODING-UNSOUND` only with the mapping or with an explicit
   decision that this source route satisfies its discharge condition. The
   implementation derives `FieldStorage::BitfieldUnit`; it never treats a
   bit-field as a byte interval.
4. Resolve `CT10-REASON-ROW` with the mapping decision. If CT10 maps, the old
   reason row is unnecessary; if mapping is deferred, any row must say that
   the mechanism exists but its bounded/check-only evidence was not accepted,
   rather than repeating the now-false “no source bit-field declaration” claim.
5. Add the standing constraint/increment/status/manifest changes only at
   integration. This branch intentionally edits none of those global ledgers.
