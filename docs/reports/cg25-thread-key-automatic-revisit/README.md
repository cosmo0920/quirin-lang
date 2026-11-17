# CG25-THREAD-KEY-AUTOMATIC-REVISIT

Discharges `CG25-THREAD-KEY-AUTOMATIC-REVISIT`, registered by
CG25-VOID-CAST-GUARD-ESCAPE. USER DECISION (2026-11-17, verbatim):
"CG25-THREAD-KEY-AUTOMATIC-REVISIT -> Let's get started." The user chose to
take the revisit up, not keep or lift. Base `5c824e443`
(card/cg25-void-cast-guard-escape, on main `36b2cf931`).

The question: rule 11v (6) now refuses every mention of a tracked in-place
object outside its declarator, a `sizeof` operand or a carried custody
actual. Is it now sound to track an automatic `pthread_key_t`, which D3
refuses at creation (`THREAD_KEY_STORAGE_REFUSAL`)?

## 1. Soundness analysis (before any semantic change)

### Method

Every way a key's value can be used was written as a fixture shape and read
by an experimental build of the candidate lift. The candidate tracks an
automatic `pthread_key_t` like a `regex` object; the shapes are listed in
the table. The build was never committed and only fed this analysis
(`private/probe/zz_thread_key_probe.rs`, logs `private/probe/probe{1,2,3}.log`).
The candidate has these parts:

* **Tracking.** `&k`, with `k` one automatic object, at
  `pthread_key_create`'s initialiser position is tracked as a `thread_key`
  object.
* **One object.** The name must have exactly one `VarDecl` in the function,
  and every `DeclRefExpr` naming it must refer to that declaration.
* **Kinded getter.** `pthread_getspecific`'s key position is kinded as a
  by-value use.
* **Address versus value.** `&k` at a by-value position is never custody.

Each shape's verdict was read off the run. "Refused" means `Unknown` with a
refusal. "Candidate" means a reported `leak`, `double_release` or
`use_after_close`. A candidate is provisional, never an upstream finding.

### Key uses

| # | Use of an automatic key `k` (after `r = pthread_key_create(&k, 0); if (r) return;`) | What decides it | Verdict |
|---|---|---|---|
| 1 | `pthread_key_delete(k)` | 11v (1)-(3): finaliser of the certified object | Safe (discharged) |
| 2 | `v = pthread_getspecific(k)` between create and delete | the kinded by-value use, carried with its place | Safe |
| 3 | `v = pthread_getspecific(k)` / `pthread_getspecific(k);` after the delete | use of a released site | candidate `use_after_close` |
| 4 | `(void) pthread_getspecific(k)` (before or after the delete), `if (pthread_getspecific(k))`, `return pthread_getspecific(k);` | 11v (6): the statement fallback lowers the call without the place, so the actual is uncarried | refused |
| 5 | `pthread_setspecific(k, v)` | 11v (6): the setter's declaration is withheld from the function map (rule 12t), so `k` there is no custody actual | refused |
| 6 | `pthread_key_t k2 = k;`, `k2 = k;` | 11v (6): stray mention (also (4)) | refused |
| 7 | `use_key(k)` (uncontracted), any unkinded position | 11v (6): stray | refused |
| 8 | `h->k = k;`, `global_key = k;` | 11v (6): stray | refused |
| 9 | `return k;` | 11v (6): stray (a statement keyword is never a declarator) | refused |
| 10 | `&k` anywhere but the create (`p = &k`, `memset(&k, …)`) | 11v (6): stray; the formal-address gate | refused |
| 11 | `pthread_key_delete((pthread_key_t) &k)`, `&k` at any by-value position | 11v (6) as amended: `&k` is not custody at a by-value position | refused |
| 12 | `k == 0`, `k + 0` as an actual | 11v (6): stray | refused |
| 13 | `sizeof k` | 11v (6) admits the operand; 11v (4)'s statement guard refuses the declaration statement | refused (over-refusal) |
| 14 | create in a loop, one delete after it | loop re-acquisition of a live site | candidate `leak` plus a refusal |
| 15 | create and delete in each iteration; `while` with `break` | strong update per iteration | Safe |
| 16 | create, `continue` past the delete | the next iteration's re-creation of a live site | candidate `leak` |
| 17 | create twice without a delete between | the first site is live at the second | candidate `leak` |
| 18 | delete twice | 11v (1): second finaliser of a released site | candidate `double_release` |
| 19 | delete without create (`pthread_key_t k = 0; pthread_key_delete(k);`) | `k` is not tracked, so the finaliser has no place | refused |
| 20 | create on one path, delete after the join | the delete's binding is not certain | refused |
| 21 | delete before the status test | 11v (3): finaliser of a possibly failed site | refused |
| 22 | early return before the delete; delete in one arm only | certified site live at an exit | candidate `leak` |
| 23 | a missing delete | as 22 | candidate `leak` |
| 24 | status discarded, or tested in the creating expression | 11v (3): no witness | refused |
| 25 | status written after its test (`r = 5;`) | the test already certified | Safe |
| 26 | `pthread_once(&once, init_fn)` between create and delete | the callback is another function, with its own frame; it can reach `k` only through an escape, which is stray | Safe |
| 27 | a local `k` shadowing a file-scope `k`, every reference to the local | the local is the one object | candidate `leak` (the local is never deleted) |
| 28 | a local shadowing a file-scope key, with any reference to the file-scope key in the same function (before the block, after it) | the `DeclRefExpr` check: two declarations behind one name | refused (`THREAD_KEY_STORAGE_REFUSAL`) |
| 29 | `&k` with `k` a formal, a member, through a pointer, `register` (Clang rejects), block `static` | not one automatic object (block `static`: D3's static reading) | refused (static: D3) |
| 30 | `free(pthread_getspecific(k))` | the nested value is hoisted with its place; the result owns nothing | refused (unresolved identity) |
| 31 | **`v = pthread_getspecific(k)` before the status test** | 11v (1)-(6) **admit it**: the use reads no witness | **Safe without a checked clause** |
| 32 | **`v = pthread_getspecific(k)` on the failure edge** (`if (r) { v = pthread_getspecific(k); return; }`) | 11v (1)-(6) **admit it**: the failure edge removes the site, and a use of an unbound place is the plain borrowed reading | **Safe without a checked clause** |
| 33 | **`v = pthread_getspecific(k)` before any create** | as 32: no binding yet | **Safe without a checked clause** |

### The destructor

`pthread_key_create`'s libc declaration carries
`resource_literal_argument(destructor, 0)` (§C04.5). It applies only when
the destructor is the null pointer constant (`0`, or `(void *) 0` by
§C04.5.1). Any other destructor leaves the call undeclared, so it is
refused (`a_non_null_destructor_leaves_the_call_undeclared`, and its
automatic counterpart in §3). No destructor ever runs on a tracked key, so
there is no callback for the run to decide. `pthread_key_delete` runs no
destructor (POSIX), and with no destructor registered none is owed.
`pthread_setspecific`'s retention of its value is rule 12t's, as for a static
key; tracking the key does not change it.

### Verdict

Rows 1-30 are refused or correctly decided by rule 11v (1)-(6) as they
stand. Rows 31-33 are not: POSIX leaves undefined a use of a key value that
was not obtained from `pthread_key_create`. A use before the status test, on
the failure edge, or before any creation is such a use, and 11v (1)-(6) would
admit it `Safe`. CG25's in-place `use` (`QCore.ResourceKinds.transferI
(.use …)`) checks only the `RELEASED` bit, and the `regex` kind has the same
reading (§5).

So the plain lift is **not sound**. A narrower lift is: a by-value use of a
tracked key is admitted only where the key is **certified**. That means the
binding is one identity, neither unknown nor possibly failed, with its site
live or released. A released site stays the `use_after_close` candidate.
With that clause (rule 11v (7), read from the settled states as the kind
candidates are), rows 31-33 refuse and nothing is admitted `Safe` without a
checked clause. The setter (row 5) stays refused: admitting it would need
rule 12t's retention route to carry the key's place, which is a follow-up.

Fail-closed settles the choice, so this card lifts the refusal, narrowed as
above.

## 2. Census and unit differential

The [prediction](PREDICTION.md) holds the method. In short:

* **Census.** Every `pthread_key_create` actual in the pinned corpus
  inputs, the user's `current` evidence tree and their bundled `lib/` trees
  is `&G` of a static object, a member, or reached through a pointer. **No
  evidence tree creates an automatic key**, so no measured row tracks one.
* **Kinded getter.** The getter change touches every body that calls
  `pthread_getspecific`. A **unit differential** checked it. The inputs are
  the 40 units of the two `current` compile databases whose source names a
  key call or a TLS macro (`private/probe/units/`, preprocessed with their
  recorded commands). Each unit was analysed with every loadable libc
  module, once by a base tree (`git archive 5c824e443`) and once by the
  change. Driver: [census/unit_probe.rs.txt](census/unit_probe.rs.txt).
  Preprocessing: [preprocess_units.py](census/preprocess_units.py) and
  [entries.py](census/entries.py).
  Result: **identical**. 40 units and 19,619 function rows (4,413 Safe,
  15,204 Unknown, 2 Unsafe) have byte-identical status and diagnostics at
  base and tip. The whole output's SHA-256 is
  `552a46b349bb32200a71cdc1fbc8970fef6650c0656e38c6bf13967ddabec797` on both
  sides. Per unit: [unit-differential.tsv](census/unit-differential.tsv)
  (the unit list is [units.tsv](census/units.tsv); full rows in
  `private/probe/diff-{base,tip}.tsv`). Fluent Bit's units were analysed
  with libc contracts only, not their provider modules. This is a
  differential of the change, not a route measurement.
* **Synthetic shapes, both trees.** The shape probe
  ([census/shape_probe.rs.txt](census/shape_probe.rs.txt)) ran at base and
  tip ([shapes-base.txt](census/shapes-base.txt),
  [shapes-tip.txt](census/shapes-tip.txt)). Every automatic-key shape
  refuses at base with D3's refusal. Every static-key and untracked shape is
  identical at base and tip: rows 31, 32, 46, 54 and 55 of the probe.

## 3. The change (rule 11v (7))

* `legacy_in_place_custody.rs`:
  * `InPlaceNames::read` tracks a non-static `&k` key.
  * `in_place_actuals` admits it only through `automatic_key`:
    `one_automatic_object` plus `every_reference_is`, which requires every
    `DeclRefExpr` naming `k` to designate its one `VarDecl`. Any other
    storage keeps `THREAD_KEY_STORAGE_REFUSAL`, text unchanged.
  * A by-value position never takes `&k` as its place, in
    `in_place_actuals` or in `body_mentions`.
  * `verify_cfg` exempts a nested by-value use
    (`has_in_place_custody_position`).
  * `report_uncertified_key_uses` refuses a by-value use of a tracked key
    the settled state does not certify.
* `legacy_transfer.rs`: one call, beside `report_kind_violations`.
* `threads.qv`: `pthread_getspecific` gains `resource_kind(key,
  thread_key)`, a by-value use. The contract lock is regenerated in the same
  commit.
* Rule 11v (5) is amended where it said no key is tracked; (7) is new.
  Contract specification §C04.6.1 gains rule 6.

## 4. Fixtures and Lean

`crates/qcore-c-gateway/tests/thread_key_in_place.rs` covers README §1:

| Fixture | Shapes | Verdict |
|---|---|---|
| `an_automatic_key_created_and_deleted_is_safe` | create/delete; a certified getter; parenthesised actuals; `r = 5` after the test; the `r == 0` form; per-iteration loop; two keys | Safe |
| `a_getter_after_the_delete_is_a_candidate_or_refused` | assigned and plain getter after the delete; `(void)`, condition forms | `use_after_close` candidate; refused (uncarried) |
| `an_uncertified_getter_is_refused` | before the test, on the failure edge, before the creation | refused (11v (7)) |
| `a_missing_delete_is_a_leak_candidate` | no delete, early return, delete in one arm, create twice, `continue` past the delete | `leak` candidate |
| `a_double_delete_is_a_candidate` | delete twice | `double_release` candidate |
| `copy_and_escape_forms_are_refused` | copies, uncontracted callee, global and member stores, address, comparison, expression actual, setter, `return key` | refused (11v (6)) |
| `an_unwitnessed_or_undecided_key_is_refused` | discarded or consumed status, delete before the test, create on one path | refused |
| `only_the_one_automatic_key_object_is_tracked` | `&formal`, a shadowing local beside a file-scope reference (two shapes); a shadowing local alone | `THREAD_KEY_STORAGE_REFUSAL`; `leak` candidate |
| `an_automatic_key_with_a_destructor_is_undeclared` | non-null destructor | refused (§C04.5) |
| `an_untracked_getter_keeps_the_borrowed_reading` | static key getter, formal key getter | Safe, as at base |

The lib unit test `a_by_value_getter_of_a_tracked_key_is_custody_only_whole`
replaces `a_by_value_getter_of_a_tracked_key_is_stray`, as a deliberate
update. The whole getter is now custody; `&key` and `key + 0` at the getter
stay stray, and so does `&key` at the delete.

Lean, `QCore.ThreadKeyStatic` (`AuditThreadKeyStatic.lean`):

* `KInstr` gains `automaticCreate` (CG25's `init`), `otherCreate` (refused)
  and `keyUse`.
* The use theorems: `a_certified_use_changes_nothing`,
  `a_use_after_delete_is_a_candidate`, `an_uncertified_use_is_refused`,
  `a_use_changes_no_custody`.
* `diagnostics_only_grow` gives `a_safe_verdict_used_only_certified_keys`:
  every key use of a `Safe` run met a certified key.
* Eight witness programs.
* D3's `an_automatic_creation_is_refused` and
  `an_automatic_creation_tracks_nothing` become
  `another_creation_is_refused` and `another_creation_tracks_nothing`. The
  automatic creation is no longer refused.

`QCore.InPlaceMentionGuard.the_void_cast_getter_is_refused` is restated:
the getter is kinded now, so the cast form is refused as uncarried. The old
statement survives as `an_unkinded_by_value_mention_is_refused`. Both are
placeholder-free, with no project axioms (the `#print axioms` lines list
only `propext`, `Quot.sound` and `Classical.choice`).

## 5. Follow-ups

* `CG25-THREAD-KEY-SETTER-PLACE`: admit `pthread_setspecific` on a tracked
  key. Rule 12t's retention route would have to carry the key's place.
* `CG25-IN-PLACE-REGEX-USE-CERTIFIED`: for the `regex` kind, a use before
  the status test or on the failure edge is admitted `Safe` (probe R3, R4 at
  base and tip). Rule 11v (7) closes the shape for keys only. No published
  row rests on it: `rd_regex_match` uses `re` only after the zero edge.
* `CG25-IN-PLACE-REGEX-SHADOWED-NAME` (soundness): a block-scope `regex_t`
  that shadows a file-scope one is tracked across both, so `{ regex_t g;
  r = regcomp(&g, …); if (r) return 0; } regfree(&g);` is certified `Safe`
  (probe R2, at base and tip). Keys now require `every_reference_is`; the
  `regex` kind does not. `rd_regex_match` shadows nothing, so no published
  `Safe` rests on it.
* Over-refusal, recorded and not registered: `n = sizeof k;` is refused by
  rule 11v (4)'s statement guard, although (6) admits the operand. The
  `regex` kind is the same.

The integrator measures with base `rev:5c824e443` against
[PREDICTION.md](PREDICTION.md).

## 6. Gate

GATE_SECTION
