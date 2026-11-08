# OUT-PARAMETER-SCALAR-FRAME-PREMISE

Follow-up of [OUT-PARAMETER-BODY-SUMMARY](../out-parameter-body-summary/README.md)'s
stop. Branch cut from that card's tip `bb9412c2a`; measurement base
`rev:41d1b48a5`. User decision, 2026-11-07, verbatim: "A: effective-type".
[PREDICTION.md](PREDICTION.md) was committed before the semantic change
(`d36671a8f`).

## The rule: 11r(7)

Rule 11r(6)'s success class of the chunkio shape did not validate because
`*size = n` after `*buf = data` may write the caller's pointer object through
a second formal. Rule 11r(7) reads it under the listed premise
`effective-type:` (C11 6.5p7: a non-character integer or enumeration lvalue
cannot modify an object whose effective type is a pointer type):

- **Where.** Only when rule 11r(4)/(6)'s per-value inference of an undeclared
  function, or an inferred relation's per-class row (rule 12s(2)), published
  nothing; it then runs once more with the premise available to that
  function's analyses. A class or row read that way is kept only when an
  analysis relied on the premise. A declared disposition's body check never
  reads it.
- **What it admits.** In a unit whose build does not pass
  `-fno-strict-aliasing`, a typed scalar store with no scalar destination
  spares from its refused-write frame each sanctioned slot `*X` whose formal
  Clang types as a pointer to an object pointer, when the store's line holds
  exactly one write (assignment, compound assignment or increment) and that
  write is `*Y = V` / `*Y op= V`, `Y` a formal other than every such `X`, its
  left operand typed by Clang as a non-character integer or an enumeration.
- **What it refuses.** A character lvalue (`char`, `signed char`,
  `unsigned char`), a floating, `volatile`, `_Atomic` or unread lvalue type, a
  base that is not such a formal, a line with another write, every call
  (`memcpy`, `memmove`, `memset` included), a `-fno-strict-aliasing` unit,
  and every refusal of rule 11r(6) (undecided paths, a slot written on some
  exits only, a store after release, two slots given one site).
- **How it is stated.** Every row resting on the premise carries it in
  `rule_obligations` (rule `11r`), which the audit folds into the row's
  verdict-contract assumptions: the function's own row, and every in-unit row
  whose derivation reaches the function (attributed to its name through the
  unit frame, `arm_obligations::carry_declaration_premise`). The premise text:
  "effective-type: a non-character integer or enumeration store through
  another out formal does not modify a pointer out slot (C11 6.5p7)".

Code: `crates/qcore-c-gateway/src/legacy_effective_type.rs` (the scope, the
AST frame), hooks in `legacy_transfer.rs` (the frame's sparing),
`legacy_summary.rs` (`infer_exit_class_premised`, the caller carriage),
`legacy_relational_forwarding.rs` (`relational_row`'s premised retry),
`arm_obligations.rs` and `typed_access_path.rs` (`strict_aliasing_in_force`,
`IndirectCallUnit::is_enum`). Rule text: `docs/agents/rules/11r.md` (7).

## Flag census

[`census/flags.py`](census/flags.py), [`census/flags.json`](census/flags.json):

| Database | Entries | `-fno-strict-aliasing` |
|---|---:|---|
| 046accf6 Fluent Bit production (the `current` run) | 1,419 | none (chunkio 15, librdkafka 106: no aliasing flag) |
| 046accf6 Monkey build | 142 | 27, all Monkey's bundled libevent sources (`buffer.c`, `event.c`, `http.c`, …) |
| corpus route (mirrored wave pin, Fluent Bit) | 1,415 | none |
| corpus route (mirrored wave pin, Monkey) | 142 | the same 27 libevent sources |
| system route | 1,419 | none |
| slice route | 1,421 | none |

So the premise holds for chunkio and for every Fluent Bit unit of the census,
and is withheld for those 27 libevent units (whose rows stay refused with the
frame), none of which carries this card's shape.

## Fixtures and Lean

[`crates/qcore-c-gateway/tests/out_parameter_scalar_frame_premise.rs`](../../../crates/qcore-c-gateway/tests/out_parameter_scalar_frame_premise.rs),
11 tests asserting each function's status, complete diagnostics and carried
premises:

- the three chunkio shapes (`cio_utils_read_file`: `char **` and `size_t *`;
  `cio_file_content_copy`: `void **`, a member-read size and a copy;
  `cio_memfs_content_copy`: `*out_size = ch->len`) certify, callee and testing
  caller both listing the premise;
- an enumeration store through the second formal certifies;
- a `char *` store (`*tag = 'x'`), a `double *` store and a unit compiled with
  `-fno-strict-aliasing` stay refused;
- a store after release beside the scalar store stays refused (rule 11r(6));
- two writes on one line stay refused (the store is not told apart);
- a body that writes the scalar before the slot certifies with no premise;
- a `memcpy`-like copy into the second formal: see the deviation below.

`QCore.OutParameterScalarFramePremise`
([`proof/QCore/OutParameterScalarFramePremise.lean`](../../../proof/QCore/OutParameterScalarFramePremise.lean),
audited by `proof/AuditOutParameterScalarFramePremise.lean`):
`wide_store_keeps_pointer_object` (under the hypothesis an integer or enum
store leaves a pointer-typed object unchanged), `character_store_may_write`,
`no_premise_may_write`, `spares_sound`, `character_refuses`,
`floating_refuses`, `opaque_refuses`, `no_strict_aliasing_refuses`.

## Deviation: a copy into the second formal

The brief asked that a `memcpy` into the second formal stay refused. The
premise never spares a slot for a call, as asked, but the fixture body
`*buf = data; x_copy(size, &n, sizeof n); return 0;` is already certified
without the premise: a contracted call with `Borrowed` positions writes no
binding, so nothing poisons `*buf` in the first place (the same holds for an
ordinary W4 `Out` summary). That is a pre-existing reading, wider than this
card (any `Borrowed` call actual pointing at writable memory), so the fixture
pins it with no premise carried, and the question is registered as
OUT-SLOT-CALL-WRITE-ALIASING rather than changed in passing.

## Prediction

Corpus `cio_utils_read_file`, `cio_file_content_copy`,
`cio_memfs_content_copy` Unknown -> Safe (`delta Safe +3`); system and slice
none; on `current` the same three rows `Safe` with the premise listed, no unit
status change. Measurement:

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=rev:41d1b48a5 --side tip=rev:aa3999333 \
  --declared docs/reports/out-parameter-scalar-frame-premise/PREDICTION.md
```

## Records

[integration-amendments.json](integration-amendments.json): base `bb9412c2a`;
retires OUT-PARAMETER-SCALAR-FRAME-PREMISE, registers
OUT-SLOT-CALL-WRITE-ALIASING. `apply_amendments.py --check` exits 0 once
OUT-PARAMETER-BODY-SUMMARY's amendments are applied (alone it refuses the
retirement of an id that card registers).

## Gate

`python3 scripts/gate_scope.py --base bb9412c2a --out <worktree>/private/gate-3`
at tip `7a6276b58` (full mode: the diff reaches `qcore-c-gateway`; Lean all):
**exit 0**. Submodules, fmt, Clippy, Rust suite (617 units, 9,302 passed,
0 failed, 16 ignored, 135 filtered), both reevaluation probes, Lean (416
audits), Python (685 tests). Record [gate.json](gate.json).
