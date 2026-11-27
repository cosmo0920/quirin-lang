# CONNECT-LIBC-EFFECT-CLAIMS: libc effect claims held outside quirin-provider-libc

Card of round 41 (user decision of 2026-11-27), base `9903b027a`. It discharges
the `named_followups` entry CONNECT-LIBC-EFFECT-CLAIMS, which
CONNECT-ADOPT-DECLARED-DEPENDENCIES measured at `1ecaf97e9`.

**State: applied (Option A).** The card first stopped before applying, under
the brief's third stop condition (see [Why the card stopped](#why-the-card-stopped)),
and handed back four [options](#options). USER DECISION (2026-11-27, verbatim):
"Option A". What changed is under [Applied: Option A](#applied-option-a). The
user accepted that two brief fixtures change with Option A: "linked and
resolving" becomes "resolves from libc", and "stale link digest holds Pending"
becomes "a changed libc module refuses the audit" (Unknown).

The census and the classification below are the hand-back's, measured at the
base. In the census, row numbers 1–5 still name the claims as they stood
there.

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

These `cl_` formulas are the hand-back's classification. They are not validator
terms. The rule that is enforced is stated in the checked CX block (C04.18.9),
which `guarded_proposition` validates (see
[Applied: Option A](#applied-option-a)). It uses the `cx_` prefix because every
id in that block must carry it (`every_formula_is_a_validated_typed_rendering`).

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

## Applied: Option A

**Contracts.**

- `effect_send`, `effect_perror` and `effect_errno_location` moved verbatim,
  with no clause changed, to a new quirin-provider-libc module,
  `providers/quirin-provider-libc/contracts/libc/callee_effects.qv`.
  - It is a literal-fact module. The resource loader refuses it, so it is in no
    view and has no `contracts.lock` entry.
  - It is listed in the crate's `CONTRACTS` index and in its README.
- libevent's `monkey/callback_task_handoff.qv` no longer states them. Its header
  says where they went.
- `monkey/timeout_teardown.qv` marks `path_call_sequences` and `path_calls` as
  observations of `cb_timeout`, owned by libevent. Only comments were added
  there; no clause changed.

**Reader** (`qcore_verification::callback_task_handoff`).

- `declared_callee_effect_in` reads each callee's effect from its owner:
  - libevent's policy for `event_del`, `event_free`, `evutil_closesocket` and
    `mk_mem_free`;
  - `LIBC_CALLEE_EFFECTS` (the libc module) for `send`, `perror` and
    `__errno_location`.
- It fails closed:
  - a declaration whose callee is a libc name (`libc-names.txt`), held outside
    quirin-provider-libc, refuses (`cx_refuse_libc_effect_elsewhere`);
  - a libc-held declaration of a non-libc callee refuses;
  - a callee with zero declarations, or more than one across both owners,
    refuses;
  - a non-zero `task_spawn` or `defer` refuses.
- The source projection carries `libc_effects_digest`.

**Audit** (`quirin-audit callback-task-handoff`). The configuration schema is
now 2, with two required fields, `libc_callee_effects` and
`libc_inventory_lock`.

- Before any source is read, `check_libc_callee_effects_identity` checks two
  things, and either mismatch refuses (Unknown):
  - the configured module's bytes equal the verifier's copy;
  - that copy's SHA-256 is the one quirin-provider-libc's
    `contract-inventory.lock` records for `libc/callee_effects.qv`.
- The report gains `libc_effects_module` and `libc_effects_digest`.
- Both new inputs are protected from report aliasing and reread before
  publication.
- A schema-1 configuration refuses.

**Enforcement** (`providers/contracts.py check`, `QL010`).

- `check_libc_effects` runs over the shipped and user load paths. It refuses a
  declaration that states `task_spawn` or `defer` for a `callee == "F"` with F
  a libc name outside quirin-provider-libc, and one in quirin-provider-libc
  whose callee is not a libc name.
- A missing names list refuses.
- Observations (no `callee` key) are not this check's to refuse.
- The shipped tree passes with 3 libc effect declarations, all in
  quirin-provider-libc.

The rule is stated as CX formulas in `qcore_schemas::connect::FORMULAS`. Each
is validated by `guarded_proposition` and has a Lean theorem in
`QCore.MeasureConnect`, audited in `AuditMeasureConnect.lean`.

```
cx_refuse_libc_effect_elsewhere:          forall (#b: cx_bundle) { forall (#g: cx_fn) { forall (#h: cx_fn) { (((#cx_declares(#b,#g) && #cx_effect_callee(#b,#g,#h)) && (#cx_libc_name(#h) && !#cx_libc_owner(#b))) => #cx_effect_refused(#b,#g)) } } }
cx_refuse_foreign_effect_in_libc:         forall (#b: cx_bundle) { forall (#g: cx_fn) { forall (#h: cx_fn) { (((#cx_declares(#b,#g) && #cx_effect_callee(#b,#g,#h)) && (!#cx_libc_name(#h) && #cx_libc_owner(#b))) => #cx_effect_refused(#b,#g)) } } }
cx_effect_refused_is_a_misplaced_effect:  forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_effect_refused(#b,#g) => exists (#h: cx_fn) { ((#cx_declares(#b,#g) && #cx_effect_callee(#b,#g,#h)) && ((#cx_libc_name(#h) && !#cx_libc_owner(#b)) || (!#cx_libc_name(#h) && #cx_libc_owner(#b)))) }) } }
cx_libc_owner_effect_accepted:            forall (#b: cx_bundle) { forall (#g: cx_fn) { ((#cx_libc_owner(#b) && forall (#h: cx_fn) { (#cx_effect_callee(#b,#g,#h) => #cx_libc_name(#h)) }) => !#cx_effect_refused(#b,#g)) } }
cx_observation_accepted:                  forall (#b: cx_bundle) { forall (#g: cx_fn) { forall (#h: cx_fn) { ((#cx_observes_call(#b,#g,#h) && !exists (#k: cx_fn) { #cx_effect_callee(#b,#g,#k) }) => !#cx_effect_refused(#b,#g)) } } }
cx_effect_refusal_fails_the_check:        forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_effect_refused(#b,#g) => !#cx_effect_check_passes(#b)) } }
cx_failed_effect_check_has_a_refusal:     forall (#b: cx_bundle) { (!#cx_effect_check_passes(#b) => exists (#g: cx_fn) { #cx_effect_refused(#b,#g) }) }
```

- The first two are the refusals and the third is their converse: a refusal
  happens only for a misplaced effect.
- `cx_libc_owner_effect_accepted` and `cx_observation_accepted` are the converse
  refusals. The libc crate's effects of libc callees are accepted, and so is an
  observation that states no effect.
- The last two tie the refusals to the check's outcome.
- Four new CX terms: `cx_effect_callee`, `cx_observes_call`,
  `cx_effect_refused`, `cx_effect_check_passes`.
- Counts: the CX block has 94 terms and the formula table 80 formulas.
- Six of the Lean theorems use no axioms. `formula_cx_failed_effect_check_has_a_refusal`
  uses only `Classical.choice`, `propext` and `Quot.sound`. There are no
  project axioms or placeholders.

**Fixtures.**

| Fixture | Where |
|---|---|
| A libc effect declaration outside libc is refused | `providers/tests/test_contract_load_path.py` `CalleeEffectCheck.test_a_libc_effect_declaration_outside_libc_is_refused` (QL010); `qcore-verification` `a_libc_effect_declaration_outside_libc_is_refused` (reader) |
| An observation in the observing crate is accepted | `CalleeEffectCheck.test_an_observation_in_the_observing_crate_is_accepted`; the shipped `timeout_teardown.qv` (`test_the_shipped_and_user_load_paths_pass`) |
| Resolves from libc (replaces "linked and resolving") | `qcore-verification` `libc_callee_effects_resolve_from_libc`, `the_projection_binds_the_libc_module_digest`; `CalleeEffectCheck.test_the_moved_declaration_is_accepted_in_libc` |
| A changed libc module refuses the audit (replaces "stale link digest holds Pending") | `qcore-verification` `a_changed_libc_module_refuses_the_audit` (changed bytes, a stale inventory digest, another crate's lock); `qcore-audit` `callback_task_handoff::tests::a_changed_libc_callee_effect_module_refuses_the_audit` (the configured audit returns its refusal, printed as `rejected (Unknown)`) |
| Also covered | libc holds no other owner's effect (`libc_holds_no_effect_of_another_owners_callee`, both layers); a missing names list refuses; a schema-1 configuration refuses; the reviewed module passes its identity check |

**Lock and manifest diff.** These were regenerated by `python3 providers/contracts.py sync`.

- `providers/contract-load-path.json` lists `libc/callee_effects.qv` among
  libc's modules (19). Its resource, bounds and outer views are unchanged.
- `providers/quirin-provider-libc/contract-inventory.lock` adds that file and
  its three declarations.
- `providers/qcore-provider-libevent/contract-inventory.lock` drops the three
  declarations and records the two changed module digests.
- `providers/qcore-provider-libevent/contracts.json` records the new digests of
  `callback_task_handoff.qv` and `timeout_teardown.qv`.
- No `contracts.lock` changed, and no pin, `linkage-reliances.json` entry or
  `contract_dependencies` entry changed.
- The user load path's crates (libcurl, onigmo, openssl) are unchanged.
  `providers/contracts.py check` validates the user load path against
  `quirin-audit plugin views` (`check_user_crates`) and passed, so no
  `plugin views --write` was needed.

## Checks (no card gate)

Logs are under the worktree's `private/r41/`.

| Check | Exit | Counts |
|---|---|---|
| `python3 providers/contracts.py sync` | 0 | libc 19 modules (resource view 17, bounds view 1) |
| `python3 providers/contracts.py check` | 0 | linkage 15 recorded, 19 links, 11 inherit links; callee effects: 3 libc effect declarations; 95 contracts; user crates 3 |
| `cargo test -p qcore-contract -p qcore-audit -p qcore-verification -p qcore-frontend -p qcore-schemas` (run 2) | 101 | 1832 passed, 1 failed, 4 ignored, 207 binaries. The failure is `the_libcurl_ranges_follow_the_audited_curlver_header`, the known libcurl `curlver.h` host failure. |
| `cargo test -p qcore-compiler --test qcore_contract_corpus` (run 2) | 0 | 6 passed |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | 182 passed, 1 ignored, 52 binaries |
| `cargo clippy` (qcore-audit, -verification, -frontend, -schemas; providers: quirin-provider-libc, qcore-provider-libevent), `--all-targets -D warnings` | 0, 0 | |
| `cargo fmt --check` (root, providers) | 0, 0 | |
| `python3 scripts/gate_lean.py --module QCore.MeasureConnect --audit AuditMeasureConnect.lean` | 0 | 1 target built, 1 audit passed |
| `python3 -m unittest discover -s providers/tests` | 0 | 38 tests |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |
| `python3 scripts/build_docs_index.py --check`, `python3 scripts/gen_rule_docs.py --check` | 0, 0 | 289 rule files |
| `python3 providers/package.py build` / `verify` (qcore.chunkio, .cmetrics, .ctraces, .libevent, .librdkafka) | 0 / 0 ×5 | |
| `python3 scripts/apply_amendments.py integration-amendments.json --check` | 0 | 1 retired, 2 registered |

Run 1 of the root tests failed in two places. `qcore_contract_corpus` had 3
failures because the corpus reads tracked files only and the new module was not
yet tracked. The new audit unit test raced on a shared temporary directory.
Both were fixed and the tests re-run (run 2). Run 1 of
`scripts/tests/test_contract_corpus.py` failed on the libc placement test,
because the libc crate's declared names must be libc names. That test now
checks an effect declaration by its callee, and it passes (9 tests).

## Follow-ups

- **CALLEE-EFFECT-RESOURCE-VOCABULARY** (option B, registered): the long-term
  home for task/defer effects is a resource-v1 predicate on the libc
  declarations themselves (`send`, `perror`, `__errno_location`). That would
  let the effect carry a `contracts.lock` digest and be linked by a dependent.
- **CALLEE-EFFECT-OWNER-GENERAL** (registered): `cx_refuse_libc_effect_elsewhere`
  covers libc callees only. Placing every callee effect declaration in its
  callee's owner crate would generalise it. libevent's other four effect
  declarations (`event_del`, `event_free`, `evutil_closesocket`,
  `mk_mem_free`) name libevent or Monkey functions, so they already sit with
  their owner.
