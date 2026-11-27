# CONNECT-LIBC-EFFECT-CLAIMS: libc effect claims held outside quirin-provider-libc

Card of round 41 (user decision of 2026-11-27), base `9903b027a`. It discharges
the `named_followups` entry CONNECT-LIBC-EFFECT-CLAIMS, which
CONNECT-ADOPT-DECLARED-DEPENDENCIES measured at `1ecaf97e9`.

**State: census and classification done. Stopped before applying** under the
brief's third stop condition: quirin-provider-libc has no vocabulary for
task/defer effects, and the recommended resolution cannot be added as a small
extension (see [Why the card stopped](#why-the-card-stopped)). No contract,
lock, manifest, Rust, Python check or Lean file changed. The options are under
[Options](#options).

## Census

Scope: every module of every crate on the shipped load path
(`providers/contract-load-path.json`) and the user load path
(`providers/user-crates-load-path.json`), except quirin-provider-libc. That is
13 crates, 166 modules and 1003 declarations. A clause is a hit when it names a
libc/POSIX function from `providers/quirin-provider-libc/libc-names.txt` (the
list rule 10j's corpus test reads) in one of three places: inside a string
literal, as a bare identifier compared with `==`/`!=`, or as the framed callee
of `quant cell F: T |-> #untouched;`. The script is
[scripts/census.py](scripts/census.py) and its raw output is
[census.json](census.json) (15 hits). Run
`python3 docs/reports/connect-libc-effect-claims/scripts/census.py . > census.json`
to reproduce it.

| # | Crate / module | Declaration, clause | libc names | Effect claim? | Class | Read by | Can a Safe/Unsafe rest on it? |
|---|---|---|---|---|---|---|---|
| 1 | libevent `monkey/callback_task_handoff.qv` | `effect_send`: `callee == "send"; task_spawn == 0; defer == 0` | `send` | yes | **declaration** | `qcore_verification::callback_task_handoff::declared_effect` (`EFFECT_DECLARATIONS`), from `path_calls`, from `project_callback_task_source`, from `quirin-audit callback-task-handoff` (`qcore_audit::callback_task_handoff`) | A positive bounded certificate rests on it ("bounded obligations checked"; assumption "declared callee effect contracts are trusted"). No pinned route reads it. Never an Unsafe: a missing or nonzero effect refuses the audit (Unknown). |
| 2 | same | `effect_perror`: `callee == "perror"; task_spawn == 0; defer == 0` | `perror` | yes | **declaration** | same | same |
| 3 | same | `effect_errno_location`: `callee == "__errno_location"; task_spawn == 0; defer == 0` | `__errno_location` | yes | **declaration** | same | same |
| 4 | libevent `monkey/timeout_teardown.qv` | `cb_timeout_source`: `path_call_sequences == "send,__errno_location,perror,…;…;send"` | `send`, `__errno_location`, `perror` | yes (calls on paths) | **observation** | `qcore_verification::timeout_teardown` (source projection), then `quirin-audit timeout-teardown`, `cycle18_composition` and `cycle18_callback_obligations` | It pins the observed shape of `cb_timeout`'s body and says nothing about what the libc callees do. A mismatch refuses (Unknown). |
| 5 | same | `cb_timeout_teardown_path`: `path_calls == "send,__errno_location,perror,…"` or the variant without `perror` | `send`, `__errno_location`, `perror` | yes (calls on a path) | **observation** | `qcore_verification::timeout_teardown` (selects the teardown path indices) | same as 4 |
| 6 | libevent `monkey/native_platform_matrix.qv` | `linux_poll_asan_profile`: `backend == "poll"` | `poll` | no: names a libevent backend, not a call | — | — | — |
| 7 | same | `linux_select_asan_profile`: `backend == "select"` | `select` | no: names a libevent backend | — | — | — |
| 8 | libevent `monkey/event_payload.qv` | `contextual_payload_access`: `offset == index * element_size` | `index` | no: a local variable name | — | — | — |
| 9–15 | fluent-bit `fluent_bit/input_instance.qv` (`flb_service_set`, `flb_config_set_property`), `fluent_bit/lib_engine_thread.qv` (`flb_start`, `flb_stop`, `flb_destroy`, `flb_input_thread_instance_exit`, `flb_output_thread_pool_destroy`) | `resource_review_pending(…, "<review reason>")` | `read`, `write`, `exit`, `log`, `shutdown` | no: ordinary English words in review-reason prose | — | — | — |

No crate other than libevent makes an effect claim about a libc symbol. No
frame (`quant cell`) in any non-libc crate names a libc function. The user load
path crates (libcurl, onigmo, openssl) have no hits.

**What quirin-provider-libc states for these symbols.** `send`
(`libc/socket.qv`), `perror` (`libc/stdio.qv`) and `__errno_location`
(`libc/errno.qv`) are resource-v1 declarations, `flexible` in its
`contracts.lock`. They state custody only: `resource_borrowed` for each
parameter, `resource_return_none()`, and for `__errno_location` the reading
premises. None of them says anything about task or defer effects, and nothing
in the crate does. The one occurrence of "spawn" is a comment in
`libc/pthread_sync.qv` explaining why `pthread_create` is left undeclared. The
crate has **no task/defer vocabulary** in any form.

## Classifying rule

A claim is a clause `c` of a declaration in crate `crate(c)` that names a libc
function `f ∈ LibcNames` as the value of an effect key
(`callee`, `task_spawn`/`defer` beside a `callee`, `path_calls`,
`path_call_sequences`, `*_calls`). `subject(c)` is what the clause states a
fact about. A claim is one of two things:

- A **libc declaration**: its subject is the libc callee itself, and it holds
  at every call site.
- A call-site **observation**: its subject is a body the observing crate
  owns, and `f` appears only as an element of that body's calls.

```
cl_effect_claim:     forall (#c: claim) { (#names_libc_effect(#c) <=> exists (#f: fn) { (#f ∈ LibcNames && #effect_value(#c, #f)) }) }
cl_declaration:      forall (#c: claim) { (#libc_declaration(#c) <=> (#subject(#c) = #callee(#c) && #callee(#c) ∈ LibcNames && forall (#s: site) { (#calls(#s, #callee(#c)) => #holds(#c, #s)) })) }
cl_observation:      forall (#c: claim) { (#observation(#c) <=> exists (#b: body) { (#subject(#c) = #b && #owner(#b) = #crate(#c) && #owner(#b) != quirin_provider_libc && forall (#f: fn) { ((#f ∈ LibcNames && #effect_value(#c, #f)) => #f ∈ #calls(#b)) }) }) }
cl_owner_libc:       forall (#c: claim) { (#libc_declaration(#c) => #crate(#c) = quirin_provider_libc) }
cl_owner_observer:   forall (#c: claim) { (#observation(#c) => #crate(#c) = #owner(#subject(#c))) }
```

The converse refusals:

```
cl_refuse_declaration_elsewhere: forall (#c: claim) { ((#libc_declaration(#c) && #crate(#c) != quirin_provider_libc) => #refused(#c)) }
cl_refuse_observation_foreign:   forall (#c: claim) { ((#observation(#c) && #crate(#c) != #owner(#subject(#c))) => #refused(#c)) }
cl_classes_disjoint:             forall (#c: claim) { (!(#libc_declaration(#c) && #observation(#c))) }
cl_unclassified_stops:           forall (#c: claim) { ((#names_libc_effect(#c) && !#libc_declaration(#c) && !#observation(#c)) => #hand_back(#c)) }
```

The two classes are disjoint because a libc declaration's subject is in
`LibcNames`, while an observation's subject is a body that quirin-provider-libc
does not own. Applied to the census, rows 1–3 are libc declarations: their
subject is `callee(c)`, and nothing in the clause depends on the calling body,
path or revision. They are therefore misplaced under `cl_owner_libc`. Rows 4–5
are observations of `cb_timeout`, a Monkey body libevent's policy owns, so they
are correctly placed under `cl_owner_observer`. Rows 6–15 are not effect claims
(`!#names_libc_effect`). No claim is unclassified, so the first stop condition
does not hold.

The formulas are stated here and are not yet in the checked CX block (C04.18.9).
Adding them, under the `cl_` prefix through `guarded_proposition`, belongs to
applying the resolution.

## Why the card stopped

The brief recommends this resolution:

1. Move the libc declarations into quirin-provider-libc.
2. In the libevent declarations that relied on them, link them with
   `linkage_contract_v1` (`quant #link libc::F`).
3. Show with a fixture that a stale link digest holds the dependent `Pending`.

Three facts block a small version of it:

1. **quirin-provider-libc has no task/defer vocabulary.** Its declarations of
   these symbols are resource-v1 and state custody only. The resource-v1
   loader is total-or-reject over a closed predicate set (C04 item 3), so
   `ensures normal task_spawn == 0` inside `send`'s declaration would reject
   the whole module. A resource-v1 task/defer predicate would be a new C04
   vocabulary item, needing loader, schemas, spec, grammar and Lean parity.
2. **`linkage_contract_v1` cannot link a literal-fact declaration.** C15.2
   defines links on resource-v1 declarations. A link's target identity is the
   target's `contracts.lock` `contract_sha256`. `sync` refuses a target the
   lock does not record (`QL007`), and that lock holds only resource-v1
   pinned/flexible declarations. A moved `effect_send` would be a literal-fact
   declaration with no lock digest, so `quant #link libc::effect_send;` fails
   `QL007`. C15.2 also fixes v1: "a change is `linkage_contract_v2`, never a
   widening". Linking literal declarations is therefore a v2 vocabulary, not a
   small extension.
3. **The reader is outside link resolution.** The effect declarations are read
   by `qcore_verification::callback_task_handoff` from an `include_str!` of the
   libevent module. The audit `quirin-audit callback-task-handoff` neither
   loads the load path nor resolves dependencies, so a stale link digest could
   not hold its certificate `Pending` without wiring that audit to
   `qcore_schemas::connect::resolve`. The dependents that could carry a link
   (`selected_callbacks`, `bounded_no_task_observation`) are literal-fact
   policy declarations, not resource-v1 declarations.

Moving the claims changes no verdict on any option below. No pinned route
(corpus, twelve-home, system, slice) reads `callback_task_handoff.qv` or
`timeout_teardown.qv`: neither module is in libevent's resource view, and the
pinned route reports each Monkey callback's Cycle 18 composition as `Unknown`
with a reason, without running it. The second stop condition therefore does not
hold, and only the third does.

## Options

**A. Move the literal declarations, with the owner read directly and no link
(recommended).**

- Add a non-view libc module `libc/callee_effects.qv`. It holds `effect_send`,
  `effect_perror` and `effect_errno_location` verbatim, in the same
  literal-fact vocabulary, and adds no new vocabulary. libevent already keeps
  12 such non-view modules.
- Remove the three declarations from libevent's `callback_task_handoff.qv`.
- `declared_effect` reads each callee from its owner: libevent's policy for
  `event_del`, `event_free`, `evutil_closesocket` and `mk_mem_free`, and the
  libc module for the three libc callees. A callee must still have exactly one
  declaration across both texts. The report binds the libc module's digest
  beside the policy digest, and the audit refuses a configured libc module
  that differs from the verifier's copy.
- After the move, no libevent declaration relies on a libc clause. The audit's
  own code reads the owner, which is not a contract-to-contract reliance, so
  `linkage-reliances.json` gains no row and there is nothing to `#link`.
- Mark rows 4–5 in `timeout_teardown.qv` as observations of `cb_timeout`.
- Enforce the rule in `providers/contracts.py check`, beside the linkage check:
  a declaration outside quirin-provider-libc whose `callee == "F"` names
  `F ∈ LibcNames` next to a `task_spawn` or `defer` fact is refused
  (`cl_refuse_declaration_elsewhere`, with a new `QL` code).
- Fixtures: (a) a libc effect declaration placed in a non-libc crate is
  refused; (b) a `path_calls` observation in the observing crate is accepted;
  (c) the moved declaration resolves from libc and the projection is
  unchanged; (d) a changed libc effect module refuses the audit as a policy
  identity mismatch (Unknown).
- Sync: the libc and libevent `contract-inventory.lock` files, both crates'
  `contracts.json` digests, and the module list in `contract-load-path.json`.
  No `contracts.lock` entry changes, because the three declarations are not
  pinned.
- The cost is about one working session. Two brief fixtures, "moved
  declaration linked" and "stale link digest holds the dependent Pending", are
  replaced by (c) and (d). This is a stated deviation for the user to accept.

**B. A resource-v1 task/defer vocabulary in quirin-provider-libc.**

- Add an admitted predicate, for example
  `ensures normal resource_task_effect(none);`, to `send`, `perror` and
  `__errno_location`. Teach the loader, `qcore-schemas` and the C04 spec and
  grammar to admit it, with Lean parity.
- Rewrite `declared_effect` to read the loaded libc environment (the pinned
  libc identity) instead of literal facts, and drop libc callees from the
  libevent policy.
- The effect then lives on the libc function's own declaration, with its lock
  digest, so a later linked dependent becomes possible.
- This is a multi-session vocabulary card. It is the long-term home if task
  and defer effects grow beyond these three callees.

**C. Option A plus `linkage_contract_v2` for literal-fact declarations.**

- Add per-declaration digests for non-pinned declarations, for example from
  `contract-inventory.lock`.
- Extend `QL007`, the Rust manifest reader (`QI014`) and the resolver to
  accept those digests.
- Allow `requires linkage_contract_v2; quant #link libc::effect_send;` in
  libevent's policy declarations, and wire `quirin-audit callback-task-handoff`
  to resolve dependencies so that a stale digest holds its certificate
  `Pending`.
- This meets the brief's fixtures literally, but it changes a fixed versioned
  vocabulary (C15.2) and needs a user decision and Lean parity for the new
  resolution rules.

**D. Keep the claims in libevent as an accepted exception.**

- Keep rows 1–3 in libevent and amend the follow-up with this census.
- Add only the refusal from option A, with the three existing declarations
  grandfathered by name, so that no new libc effect declaration can appear
  outside libc.
- This is the smallest change, but it leaves the libc facts with two owners in
  effect, contrary to rule 10j.
- Restating rows 1–3 as deferred-review (`Pending`) observations instead
  (decision of 2026-09-28) would turn the audit's positive certificate into
  `Pending`, which is a result move. This variant is not recommended.

## Follow-ups

- The integrator should return the chosen option to this card. Option A or D
  is a resume on this branch. Option B or C is a new card.
- Generalising `cl_owner_libc` to every owner is out of scope. That rule would
  place a callee effect declaration in the callee's owner crate. libevent's
  other four effect declarations (`event_del`, `event_free`,
  `evutil_closesocket`, `mk_mem_free`) name libevent or Monkey functions, so
  they already sit with their owner.
