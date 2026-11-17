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
