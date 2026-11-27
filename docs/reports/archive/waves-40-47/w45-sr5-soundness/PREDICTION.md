# W45-SR5-SOUNDNESS — prediction (frozen before any Rust or Lean change)

**Base:** local `main` at `33558c67` (STABILITY-REEVAL-5 had not landed on
`main`; its report, probe and entries were read from the local branch
`integrate/stability-reeval-5` at `653317ff`, without checking it out).

**BEFORE** (`measure/sr5s/before/`, release binary of `33558c67`, `--jobs 8`,
exit 0) reproduces the baseline
`docs/reports/w45-container-root-reach/integration/artifacts/`: the diagnostics
artifact is byte-identical (`baseline_identity.py`, exit 0), as are the source
and chunk-contract sidecars; the contracts sidecar differs in exactly the
top-level `refuted_contracts` key the brief names, and source-effects only in
its hex digests (`scripts/sidecar_check.py`, exit 0).

## What the one-off probe measured at the base

The probe (`probes/zz_sr5s_probe.rs.txt`, output `probes/probe-before.log`,
72 rows, run once as a temporary `crates/qcore-c-gateway/tests/` target) sorts
the brief's findings:

| Finding | At the base | Class |
|---|---|---|
| `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` | `Safe` for the SR5 shape, its null-tested form, through a wrapper `node_drop(n) { node_destroy(n); }`, with a declared contract for the unit-defined destructor, and for `q_nullable_free(n->next); node_destroy_next(n);`. The mirrored order, a caller alias and a callee alias are already `Unknown`. A different member (`n->buf` then a destructor of `n->next`) is `Safe` and should stay so | admitting |
| `HELD-PREFIX-SOURCE-FORGES-PENDING` | `Pending` for a source-spelled prefix, for a spelling that collides with a withheld name, for a unit-defined prefixed function, and as a unit verdict | forgery (non-Safe) |
| `PUBLICATION-ROOT-RELEASED` | `Safe` after `q_free(q)` and after a unit-defined `queue_destroy(q)` | admitting under an unstated premise |
| `COUNTED-USER-CONDITIONAL-OPERAND` | `u_ref(t); c ? u_unref(t, 0) : (void)0; return 0;` holds `restored` and the body is `Safe`; its caller is `Safe`. `&&` likewise. `pick(1)(t)` hides a call and the caller is `Safe` | admitting |
| `DECLARED-NEUTRAL-UNVERIFIED-AT-CALL` | a declared-neutral body that writes the counter in a loop is `Unknown` itself, and its caller is `Safe` | admitting |
| `CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER` | a link helper over `void *arg` is `Safe`, and a caller that frees the element after linking it is `Safe` | admitting |
| `SCALAR-READ-PARENTHESISED-DOUBLE-DEREF` | the reader admits `(*q)->len`, `(*q)[0]`, `**q`, `(q->next)->len` and `*(q->arr)` (read); no end-to-end `Safe` differs from its plain two-dereference neighbour, because a formal is untracked | premise breach, no measured admission |
| `PENDING-BYPASSES-UNIT-SAFE-GATES` | an `#if 0` or K&R definition beside a genuine held call is a `Pending` unit; the same unit without the hold is `Unknown` | forgery (non-Safe) |
| `GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK` | re-publishing under an alias head is `Unknown` | fail-closed |
| `STATIC-ROOT-SHADOW-READER` | three shadowing declarations containing `(` are `Unknown` | fail-closed |

## Prediction for AFTER

Every change refuses; none admits. So:

1. **No function or unit moves towards `Safe`**, and no `Unknown` becomes
   `Pending`.
2. **Held prefix, Pending gates:** no corpus source spells a reserved prefix and
   the corpus publishes no `Pending` (baseline Pending 0), so **0 movers**.
3. **5k:** the corpus's rule-1 bodies release through declared callees (`free`,
   `rd_free`, `mk_mem_free`; the unit-defined inline wrappers among them hand
   their formal only to `free`), so **0 movers**, and no new provisional
   `Unsafe` (the new double-release report needs a unit-defined destructor that
   releases the same member).
4. **10d root released, `container_of` over a formal, the counted-user walker,
   declared neutrality, the scalar reader:** each can only move `Safe` (or
   `Unsafe`) to `Unknown`. Predicted total **0 to 5 functions**, every one a
   correction named with its shape; leak, unknown_alias and unsupported may
   move by the same handful of leaves.
5. **Headline:** functions 398 / 9 / 4,427 → **393–398 Safe / 9 Unsafe /
   4,427–4,432 Unknown / 0 Pending**; units 5 / 8 / 168 / 4 unchanged unless a
   corrected function sits in one of the 5 Safe units; leak 34 unchanged.
6. **Fluent Bit slice** at `046accf6`: `resource_user_neutral()` is declared for
   `flb_task_retry_destroy` and `flb_output_thread_pool_flush`; if a slice unit
   defines one of them beside a caller, that caller's clause becomes undecided
   (OR-7 rests on the second). Predicted: **no movement** (the definitions
   live in their own units), confirmed or refuted by running the slice.
