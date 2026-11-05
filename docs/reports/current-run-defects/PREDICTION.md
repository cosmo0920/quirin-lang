# CURRENT-RUN-DEFECTS prediction

Base `945c0db1b`; measurement base side `baseline:c8c7edf21` (the main
checkout's `private/baselines/` holds no baseline at `945c0db1b`): `eight-home`
(corpus), `fluent-bit-system` (system) and `fluent-bit-tail-pause` (slice).

This prediction is committed after the defect-2 commit (`b8a4e7140`) and
the defect-1 preparation commit (`a03df3cf2`), and before the records. The
two code changes it covers were written after their diagnosis (README §1,
§2); nothing of defect 3 or the review markers changes code or contracts.

**What changes.**

1. `quirin-audit prepare clang-build` also builds the CMake targets that
   generate build-time sources and headers (`buildvm_arch_h`,
   `lj_gen_headers`, `lj_gen_folddef`, `libluajit`, `flb-ra-parser`,
   `flb-sp-parser`, `processor-sql-parser`, `backtrace`). The measured
   routes read fixed, digest-pinned prepared inputs
   (`crates/quirin-audit-plugin-measure/inputs/live.json`); none is
   re-prepared by a measurement, so no route sees the change.
2. A unit whose every raw-scan definition is compiled out by its own
   command publishes the stated cause as its reason instead of
   ``C function `X` was not found``. Its status (`Unknown`), its
   `not_projected` rows, its function rows (none) and its reason code
   (`incomplete-obligation-coverage`) are unchanged. At the base this is
   one unit: the corpus's `librdkafka` `tinycthread.c` (30 raw definitions,
   all under `#if !WITH_C11THREADS` with `WITH_C11THREADS 1`). Neither the
   system nor the slice selection holds a unit with that reason.

**Affected rows at the base: none.** No function row exists for the one
changed unit, so no function row moves; the unit row's `reason` text is the
only byte that changes, on the corpus.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`, rerun by the integrator after landing):

- Rerun on the existing prepared inputs, nothing about the pre-pass
  changes: the 34 Fluent Bit units stay unpreprocessable and the run's
  callback members stay open with the same unread count (36).
- Rerun after `quirin-audit prepare` on those inputs (the step builds the
  generated inputs into the existing build): the 34 Fluent Bit units become
  readable (LuaJIT 21 units, `lj_vm.S`, the six flex/bison outputs, the
  five units that include their headers or libbacktrace's). The units that
  still cannot be read are the ones that are not C and name a pre-pass
  token, measured in README §1. Unit status changes: `tinycthread.c` keeps
  `Unknown` with the stated reason; no other unit's status is predicted to
  move, and none is removed from or added to the 186-configuration
  denominator (the pre-pass reads units; it adds no configuration).
- No callee leaves the uncontracted list (2,790) and no function leaves the
  508 "blocked only by uncontracted calls" set: the card adds no contract.
