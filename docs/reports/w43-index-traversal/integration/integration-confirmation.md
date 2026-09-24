# Integration confirmation (combined tree)

Integrator re-run, 2026-09-25, on merge commit `b89e0a45` and one test re-pin
commit `b79aa07e`. This card had base `ec70709a`, with W43-INDIRECT-ACCESS-GENERAL
merged in, and was measured against `w43-indirect-access-general`. It was merged
onto main `48732424`, after W43-LIBC-RESTRICT-PROTOTYPES and W43-RESULT-ALIAS-RETURNED.
The integrator ran it in its own worktree with `CARGO_INCREMENTAL=0` and
Clang 21.1.8. Figures are in [gate.json](gate.json), and private logs are in the
integrator session's scratch `gate-integrate-traversal/` and `gate-integrate-traversal-2/`.

## Soundness hole closed

W43-INDIRECT-ACCESS-GENERAL, already on main and pushed at `0a07cb57`, admitted
`out[m++] = c` without forgetting `m`. For `int m = 0; out[m++] = 'a'; if (m) return;`
the analyzer kept `m == 0`, dropped the `return`, and could certify a leaking
function as `Safe`. That breaks hard invariant 1 (fail-closed).

This card closes the hole. Subscripts may no longer write, and `B[X++]` is admitted
again only with the forget. The card measured that no corpus verdict relied on the
hole. The combined run agrees: the three published integrations since `0a07cb57`
compose leaf for leaf with their cards' own measurements, and no function left
`Safe` here.

## Gate

The first run (`gate-integrate-traversal/`, on `b89e0a45`) had **2 failures**,
both W43-RESULT-ALIAS-RETURNED tests for a mutated local copy:

* `legacy_summary::derived::tests::a_mutated_local_copy_is_named`
* `a_derivation_through_a_mutated_local_copy_is_refused`

**Cause.** Traversal now tracks `q = d; q++` exactly, as an interior pointer strictly
after `d`'s start, where before the classifier had dropped the copy. So the
wrapper is admitted as `Derived{0, Interior}` instead of refused.

**Why the admission is sound.** Freeing the result is an `invalid_release`
candidate, and using it after `d` is freed is `use_after_release`.

**Re-pin, commit `b79aa07e`.**
* Forward steps are admitted. The wrapper and a read-only caller are `Safe`.
* A caller that frees the result is `Unsafe(invalid_release)`.
* Backward steps (`q = q - 1`, `q--`) keep `NoFormalBase`, and their caller sees
  an uncontracted call.

The core gate was then rerun in full (`gate-integrate-traversal-2/`, on `b79aa07e`):

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,301 passed / 0 failed / 11 ignored**, 286 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **174** audits |
| Python before regeneration | 1 | only the derived-facts check (Lean 173 vs 174) |
| `build_matrix.py --gate`, then Python | 0 / 0 | 324 follow-ups; **330** tests |

The provider gate, package steps and corpus run come from the first run. They read
no gateway test file, and all of them exit 0:

* 124 provider tests passed / 0 failed / 0 ignored, over 39 summaries;
* the isolated mutation control passed;
* all five package builds and verifications passed;
* 13 package Python tests passed;
* the release corpus run succeeded.

## Merge

`legacy_c.rs` had 11 hunks against W43-RESULT-ALIAS-RETURNED. Both cards had
extended `Derivation::Derived`: `nullable` (the relative nullness of a derived
result) and `stepped` (traversal's pointer arithmetic). The merge keeps both fields:

* traversal's `step` keeps a derived binding's `nullable`, and starts it at `false`
  from an identity;
* traversal's identity + stepped join takes the stepped side's `nullable`;
* derived-derived joins OR the `nullable` values and take the larger `stepped`.

One integration rule is new. In `observe_exit`, a stepped pointer that may sit at
its object's start (`Stepped::Any`) is **not** reported as `ReturnedValue::Derived`.
An inferred summary therefore never publishes it, and no caller can be accused of
releasing what may be the formal's own allocation. The re-pinned tests above show
this rule working.

The ledger was merged by entry and field. It retired 1 follow-up and registered 12,
so the count went **313 → 324**. The corpus row was re-derived from this run.

## Composition

This integration introduced composition check v2 ([compose_check.py](compose_check.py)).
It computes the expected combined leaves as the previous baseline, minus the card's
removed leaves that are still present there, plus its added leaves. That lets
overlapping and complementary cards compose. Re-run on the two earlier
integrations, v2 still reports `COMPOSES`.

For this card ([composition-check.txt](composition-check.txt)) the combined run
equals the expectation exactly:

* **card delta:** 1,479 leaves removed, 130 added, 11 functions moved;
* **overlap:** 1 leaf, the `ret` identity at `cio_utils.c:96`, which
  W43-LIBC-RESTRICT-PROTOTYPES had already removed by declaring `stat`;
* **complementary:** 2 functions, `cio_os_isdir` and `cio_file_native_get_size`.
  Each was left with only a condition leaf once the earlier cards had removed its
  other blocker, and this card removed that last leaf. They are `Safe` only on the
  combined tree;
* **combined:** 13 functions move Unknown → Safe, with no unit moves and no
  violation candidates.

The combined run is archived as the new baseline in [artifacts/](artifacts/),
captured in the integrator worktree
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da`.
Its figures:

| Measure | Value |
|---|---|
| Functions (Safe / provisional Unsafe / Unknown) | **287 / 84 / 4,463** |
| Units (Safe / provisional Unsafe / Unknown / data-only) | 3 / 42 / 136 / 4 |
| leak | 271 |
| unknown_alias | 4,320 |
| unsupported | 46,855 |
