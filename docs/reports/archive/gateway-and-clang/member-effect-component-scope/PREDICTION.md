# MEMBER-EFFECT-COMPONENT-SCOPE prediction

Rows are relisted against `baseline:7b726587b`
(`private/baselines/7b726587bdf6a8213f451fa1c78297bb5e476ef7/` in the main
checkout), which already carries rules 38i and 38j. Committed before the
semantic change, after the diagnosis (report §1) and census (report §2).

## Revision 1 (current), committed before the implementation

**The change** (rule 38j amended in place). The diagnosis found no component
premise: `cmt_opts_exit` is refused because its members are spelled through
a pointer typedef (`cfl_sds_t ns;`), which the lexical call-actual recogniser
refuses as an untyped member, so rule 1's admission of a member handed to a
nullable release never fires. The amendment asks Clang's typed reading of
that one actual exactly where the lexical recogniser refused it as an
untyped member, and only when all of these hold:

- the actual is spelled exactly `B -> F`, at a `Released { nullable: true }`
  position of the call's contract in force;
- `(B, F)` is a place rule 5k's P1-P4 or rule 38j's body check admitted in
  this body (`AdmissibleReleases::admits`), so every lexical premise of the
  member release is unchanged;
- the lexical reader already typed `B` as a pointer to a record (the refusal
  is `UntypedMember`, not `UntypedBase`: the typedef-spelled base stays out);
- Clang's actual is the **value** of the member (`LValueToRValue` through
  parentheses, never `ArrayToPointerDecay`) of an arrow `MemberExpr` named
  `F` over the parameter `B`, whose type is an object pointer (a `*`, no
  function type, no `volatile`/`_Atomic`), on a base that is no `union`.

Then the actual reads exactly as the lexical recogniser reads `B->F` over a
`*`-spelled pointer member (base `B`, a pointer value, no chain), and rule
1's lowering is unchanged (the position read `Borrowed`). Nothing else asks
the typed reading at a release position; the base arm of
W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS (`rd_kafka_toppar_t *rktp`,
`mk_ptr_t *p`) is not touched.

### Rows (`baseline:7b726587b`)

From the census (report §2): the functions whose every leaf is a rule-38i
release leaf, with the member spelling read from the source.

**Corpus.** Five such functions:

| row | body | predicted |
|---|---|---|
| `cmt_opts.c::cmt_opts_exit` | five `if (opts->F) { cfl_sds_destroy(opts->F); }`, `F` a `cfl_sds_t` member, `opts` a `struct cmt_opts *` kept | Unknown -> Safe (rule 38j, `stale` x5) |
| `snappy.c::rd_kafka_snappy_free_env` | `vfree(env->F)` then `clear_env(env)` (a whole-base call with no member effect: P1) | stays Unknown |
| `rdkafka_offset.c::rd_kafka_offset_file_close` | typedef-spelled base `rd_kafka_toppar_t *` | stays Unknown (base arm) |
| `rdkafka_mock_sharegrp.c::rd_kafka_mock_sgrp_record_release` | typedef-spelled base | stays Unknown (base arm) |
| `mk_core/mk_memory.c::mk_ptr_free` | typedef-spelled base `mk_ptr_t *` | stays Unknown (base arm) |

**Justification of the new Safe, per function.** `cmt_opts_exit(struct
cmt_opts *opts)`: `opts` occurs only as `opts->F` (P1); each of the five
places has one event, `cfl_sds_destroy(opts->F)` at the nullable release
position of `cfl/sds_lifetime.qv`'s `resource_released(s, nullable)`, and its
only other occurrence is the `if (opts->F)` null test before it (P2); no
loop, no `goto` (P3); no `R(opts)`, so the body keeps `opts` and states the
five paths `stale` (rule 38j). Clang types each `opts->F` as the value of a
`char *` member (`cfl_sds_t`), so rule 1 admits each release with the
position read `Borrowed`; the body then has no other leaf. Its member-effect
summary is withheld from the function table (rule 38j), so its callers in
other units (`cmt_counter_destroy` and siblings, which pass `&counter->opts`)
still see an uncontracted function: none moves. `cmt_opts_init`, the only
other function of the unit, does not call it and keeps its store leaves; the
unit stays Unknown.

The 16 other corpus functions with a pointer-typedef member release (for
example `ctr_id_destroy`, `destroy_label`, `ctr_span_destroy`) keep other
leaves: each may lose the release leaves whose place rule 5k or 38j admits,
and **none moves**. A function whose every leaf is the gate's own text (not a
rule-38i name) on such a statement would move too; the census found none, and
it would be a finding to explain before landing.

**System and slice.** No function has only rule-38i release leaves (system:
6 functions with such leaves, 1 with a pointer-typedef member,
`destroy_chunk_raw`; slice: 6, 2 with one, `dormant_destroy` and
`flb_tail_file_remove`). Each keeps other leaves: no move.

No row may move to `Unsafe`; a move to `Safe` other than this one is a
finding to explain before landing.

```measure-prediction
route corpus
move cmt_opts.c::cmt_opts_exit Unknown -> Safe
delta Safe +1
route system
none
route slice
none
```

### The user's `current` run (Fluent Bit 046accf6)

The census's estimate over the run (report §2) finds the same sole-blocker
function: **`lib/cmetrics/src/cmt_opts.c::cmt_opts_exit` Unknown -> Safe**;
the other pointer-typedef member releases (21 leaves in 17 functions, the
ctraces destructors and setters, `destroy_label`, `decode_metric_unit`,
`cmt_map_destroy`, `destroy_chunk_raw`) lose at most their admitted release
leaves and keep other leaves. No callee leaves the uncontracted list
(`cmt_opts_exit`'s summary is withheld from the function table, so its
cross-unit callers still name it uncontracted), no function leaves the
"only uncontracted" set, and no unit or library changes status
(`cmt_opts.c` keeps `cmt_opts_init` Unknown).
