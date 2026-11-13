# LERI-HELD-AND-LINK-MEMBER-PREMISES: an element destroyer's link premises, checked

Card LERI-HELD-AND-LINK-MEMBER-PREMISES (named follow-up of
LIST-ENTRY-RELEASE-IDENTITY). Base `8fd7c8d0d`. Gateway rule **46** (new,
not switchable); rules 34 and 43 amended; contract specification §C04.8
amended; schema `ResourceFunctionContract::link_members` (qcore-schemas
**26.0.0**); Lean `QCore.ElementLinkPremises`; eleven destroyer declarations
name their link member. The [prediction](PREDICTION.md) and the base census
were committed at `b3d991abf`, before any tip measurement (the code had been
drafted, unmeasured; the prediction says so). Measured code: `875d5e6d4`.

## Headline

* **P-LERI-LINK-MEMBER: discharged.** An element destroyer names the member
  its body unlinks, `resource_unpublished(p, MEMBER)`; rule 34's destroyer
  takes a foreign element only when every `container_of` binding of it names
  that member. A declaration naming none, or an element bound through another
  member, refuses.
* **P-LERI-HELD-LINKS: discharged, decided from the body's own
  initialisation.** Rule 43's held arm takes an object only when this body
  initialised the declared member's links itself (a rule 36b
  self-initialisation of that one member whose writes cover `next` and
  `prev`), on every arriving path, with no statement since that may have
  written the object. A formal, a callee-constructed object and an object
  initialised any other way refuse.
* **P-LERI-CALLER-PUBLISHED: stated, its load measured** (section 7): on the
  three routes the only Safe rows that apply an `ElementsReleased` clause are
  corpus `mk_sched_threads_purge` and `mk_sched_threads_destroy_all`, whose
  bodies track no object at the call, so the premise holds there vacuously.
* **Measured (this card's preparations):** system custody 107 / 4 / 337 →
  **103** / 4 / **341** (`flb_output_task_queue_enqueue`,
  `flb_sched_timer_cb_destroy`, both configurations, Safe → Unknown), exactly
  the predicted moves; slice 59 / 1 / 340 / 4 Pending unchanged (one leaf
  trade, `flb_tail_file_purge`); corpus: CORPUS-HEADLINE. No Unknown → Safe,
  no leak, double release or Unsafe added or removed; the null property's
  tallies are unchanged. The four system rows are exactly the rows rule 43's
  held arm carried at base (rule 43 off moves the same four), each a
  **formal** handed to a destroyer: their callers' links are not the body's
  to decide, so the premise was load-bearing there and now refuses.

## 1. Setup

Toolchain: the host's private environment (`private/toolchain/env.sh`:
system Clang 21 at `/usr/lib/llvm-21/bin/clang`, pinned Lean 4.35.0). The
host has no published live-route inputs, so the three routes were prepared in
this worktree as LIST-ENTRY-RELEASE-IDENTITY's were: `quirin-audit prepare`
(base binary, `--fluent-bit` the host's local Fluent Bit clone) at `046accf6`
(system), `90978ac6` (slice; the upstream commit, not the synthetic export
`34155955`) and `66910c10` (corpus, its production compile database copied
into `fluent-bit-clang-source/build/`), and `providers/package.py build` for
`provider-packages-wave20` ([inputs](artifacts/inputs.json)). Absolute figures
therefore differ from the published baselines; every comparison is base
against tip on the same inputs. The base figures equal
LIST-ENTRY-RELEASE-IDENTITY's measured tip on system and slice (107 / 4 / 337,
59 / 1 / 340 / 4).

## 2. Census at base: where rule 43's premises carry a row

Base binary, rule 43 on against `QCORE_GATEWAY_RULES_OFF=43`
(`scripts/compare.py`, `private/census/`):

| Route | Rows rule 43 carries | Premise they rest on |
|---|---|---|
| system | `flb_output_task_queue_enqueue` ×2, `flb_sched_timer_cb_destroy` ×2 (Safe; Unknown off) | held arm, a formal (`retry`, `timer`) handed to `flb_task_retry_destroy` / `flb_sched_timer_destroy`: P-LERI-HELD-LINKS |
| slice | none | |
| corpus | CORPUS-CENSUS | |

The census found no Safe row whose foreign-element admission rests on
P-LERI-LINK-MEMBER being wrong: the bodies of the ten Fluent Bit destroyers
and `mk_http_thread_destroy`, read at the three trees
([unlinks.py](scripts/unlinks.py), [unlinks.json](artifacts/unlinks.json),
the same body digests LIST-ENTRY-RELEASE-IDENTITY read), each unlink `_head`
unconditionally at top level (`flb_task_destroy` inside its orphan test;
`mk_http_thread_destroy` frees with `mk_mem_free`, which the script does not
count). `flb_tail_file_remove` also unlinks `_rotate_head`, only while
`file->rotated > 0`. The one caller that iterates through another member is
`flb_tail_file_purge` (`files_rotated` through `_rotate_head`).

## 3. Mechanism

**(A) The member** (`contract_counted_users.rs`, §C04.8 and the implemented
grammar amended). `resource_unpublished(p, MEMBER)` is read into
`ResourceFunctionContract::link_members` (omitted when empty, so every other
contract serialises as before; not part of the custody projection; the
clause standing labels it `resource_unpublished(#i, MEMBER)`). A member on an
unlink that does not sit beside `resource_released` of the same parameter
rejects the module: a plain unlink's entry is the link itself. The ten
Fluent Bit destroyers (`teardown_destroy.qv`, `task_users.qv`,
`input_chunk_destroy.qv`) and `mk_http_thread_destroy`
(`monkey/intrusive_list.qv`) name `_head`; `flb_tail_file_remove`'s
conditional `_rotate_head` unlink is not stated.

**(B) The foreign element** (P-LERI-LINK-MEMBER). Rule 5l's binding now
records its `container_of` member (`ForeignOrigins`), rule 10f(2)'s
`Foreign::members` collects it over every binding of the element, and the
destroyer arm of `apply_with` admits the element only when
`link_member::foreign_admits` holds: a declared member and every binding
naming exactly it. Otherwise the unlink refuses with `unlink by D of E, whose
list links it through M…, while the declaration unlinks N` (or `names no link
member`), an unresolved effect on `E`.

**(C) The held object** (P-LERI-HELD-LINKS). `AbstractState::link_ready`
holds `(site, member)` records. An admitted rule 36b `TrackedSelfInit` of
`base` adds `(site, M)` when its path is the one member `M` and its writes
cover `next` and `prev` (`initialised_member`; the two fields
`mk_list_del` reads before writing through them). The join keeps a record
only when both paths have it. After every statement `retain` forgets the
records the statement may invalidate: statements that write no memory
(`NoOp`, `Acquire`, `Alias`, `Clear`, the three branches, `TrackedSelfInit`)
keep all; a contract call with no nested call keeps the records of objects
none of its actuals' root bindings may denote (an unbound or unknown root
forgets all); everything else forgets all; a call with a nested call forgets
all before it runs. Rule 43's held arm then needs `held_admits`: the
destroyer names a member and `(site, member)` is recorded. Otherwise the
unlink refuses with `unlink by D of E, which this function holds, but this
function has not initialised its M links` (or `the declaration names no link
member`). Rule 36b's two frame checks ignore `link_ready`, a fact about its
own writes rather than a custody relation, so a second self-initialisation is
admitted exactly as before.

Nothing here admits an entry rule 43 refused, and nothing accuses: every
change replaces an admission with a refusal. The rule has no switch (no
registry entry), since off would re-admit an unchecked premise.

**RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED / -CONTRACT-ROUTE.** No
projection-tracked `->` place, no `Stored`/`Removed` mode and no new binding
is created; the new state only removes admissions. Re-read, no amendment.

## 4. Controls

`crates/qcore-c-gateway/tests/link_member_premises.rs` (2 tests), the module
unit tests (the Lean vectors), and the amended `element_destroyer.rs` and
`element_release_summary.rs`:

| Fixture | Verdict | Refusal |
|---|---|---|
| held, `mk_list_init(&e->_head)` then the destroyer | Safe | |
| held, never initialised | Unknown | not initialised |
| held, half-initialised (`next` only) | Unknown | (no record) |
| held, initialised, then `q_use(e)` | Unknown | not initialised (forgotten) |
| held, initialised, then `q_use(f)` of another object | Safe | |
| held, initialised on one branch only | Unknown | not initialised (join) |
| held, `_rotate` initialised, destroyer unlinks `_head` | Unknown | not initialised `_head` |
| held, initialised, member-less destroyer | Unknown | names no link member |
| formal handed to the destroyer | Unknown | |
| foreign element bound through `_head`, destroyer `_head` | Safe | |
| foreign element bound through `_rotate`, destroyer `_head` | Unknown | member refusal |
| foreign element, member-less destroyer | Unknown | names no link member |

No fixture is accused. The loader test accepts `resource_unpublished(e,
_head)` beside a release (member recorded, absent from the custody
projection), still loads the member-less form, and rejects a member on a
plain unlink and both malformed arities.

Amended pinned expectations (attributed, not weakened): `element_destroyer.rs`
and `element_release_summary.rs` declare `item_destroy`/`item_close` with
`_head`; the rule 43 fixtures `held-acquisition-destroyed` and
`formal-handed-to-destroyer-caller`, and rule 34's `DESTROYING` row
`q_alloc(); item_destroy(e)`, move Safe → Unknown on rule 46's held refusal
(their rule-43-off expectation was already Unknown, so "off never widens"
still holds).

## 5. Lean

`proof/QCore/ElementLinkPremises.lean`, audited by
`proof/AuditElementLinkPremises.lean` (16 theorems). A link record has two
pointer fields and `mk_list_del` is defined only when both are valid: a fresh
or zeroed record has no defined unlink
(`a_fresh_object_has_no_defined_unlink`,
`a_zeroed_object_has_no_defined_unlink`), so rule 43's unconditional arm
admitted an undefined one
(`the_unchecked_held_arm_admits_an_undefined_unlink`); a self-initialisation
covering both fields of one member makes it unlinkable
(`an_initialised_member_is_unlinkable`), half of one does not
(`a_half_initialisation_is_not_enough`); records survive exactly the
statements whose may-write set excludes their object, and the join
(`retaining_preserves_every_fact`, `the_join_preserves_every_fact`,
`an_initialisation_establishes_its_fact`); the held arm is sound
(`the_held_arm_is_sound`) and only refuses more (`the_check_only_refuses`).
Unlinking the binding member removes the element from the iterated list
(`an_admitted_unlink_removes_the_element`); another member leaves it linked
(`a_mismatched_member_leaves_the_element_linked`). `vectors_hold` equals the
Rust `FOREIGN_VECTORS`, `INIT_VECTORS` and `HELD_VECTORS`. The kill policy's
over-approximation of a statement's writes, the lexical reading of the member
and of the rule 36b write set, P-CRR-MEMBER and the Rust correspondence stay
trust boundary 28's.

## 6. Native measurement and the arbiter

ARBITER-SECTION

## 7. Premise status

| Premise | Status | Where it was load-bearing (base, these preparations) |
|---|---|---|
| P-LERI-LINK-MEMBER | **checked** (rule 46 (2), `QCore.ElementLinkPremises`) | no Safe row: every foreign element a destroyer took on a Safe row is bound through `_head` (system `flb_engine_destroy_tasks`, `flb_storage_input_destroy`, slice `flb_tail_file_remove_all`, corpus Monkey drains). Exercised once: slice `flb_tail_file_purge` (Unknown on other leaves) now refuses its `files_rotated` element |
| P-LERI-HELD-LINKS | **checked**, decided from the body's own initialisation (rule 46 (3)) | the four system rows above, each a formal; the card moves them Safe → Unknown. No route row takes the held arm with a body acquisition on a Safe path |
| P-LERI-CALLER-PUBLISHED | stated | corpus `mk_sched_threads_purge`, `mk_sched_threads_destroy_all` (the only Safe rows applying `ElementsReleased`); both bodies track no object at the call (`sched_thread_cleanup(sched, &sched->threads[_purge])` and a return), so no tracked object could be in the container: the premise holds vacuously on every measured Safe row. Follow-up `LERI-CALLER-PUBLISHED-PREMISE` |
| P-CRR-MEMBER, P-CRR-LINKED | stated (rule 10f) | unchanged |

## 8. Pins and locks

`providers/contracts.py sync` first exited 1 (`stale lock` for the eleven
changed declarations, the expected refusal), wrote both inventory locks
(eleven `link_members` entries) and the libevent `contracts.json` digest.
[pin.py](scripts/pin.py) re-pinned the eleven with the measured binary at
exactly the trees each was pinned at (logs `private/pin-1/`, every exit 0);
`pin --check` at the three trees exits 1 with only the pre-existing
not-in-force set (`flb_config_set_program_name`, `flb_tail_config_destroy`,
the instance and tail destroyers at trees whose bodies differ, and
`mk_vhost_set`, whose tip repository this host lacks); `pin --verify-contracts`
exits 0 (`private/pin-check-1/`); a second `sync` exits 0.

## 9. Gate

GATE-SECTION

## 10. Follow-ups

[integration-amendments.json](integration-amendments.json): retires
`LERI-HELD-AND-LINK-MEMBER-PREMISES` (two premises discharged, the third's
load measured and rehomed) and registers:

* `LERI-CALLER-PUBLISHED-PREMISE`: P-LERI-CALLER-PUBLISHED stays stated; it
  is vacuous on every measured Safe row today, and would become load-bearing
  the first time a caller applying `ElementsReleased` tracks a live object
  (with LERI-CROSS-UNIT-ELEMENTS-RELEASED, likely).
* `LERI-HELD-FORMAL-LINK-OBLIGATION`: the four system rows refuse because a
  formal's links are its caller's; carrying the link-state obligation to the
  callers (a summary precondition the caller discharges, by publication or
  initialisation) would decide them without the premise.
* `LERI-PUBLISHED-ARM-LINK-MEMBER`: rule 34's published arm (a destroyer of
  an object this path published) does not compare the publication's member
  (`mk_list_add(&x->M, …)`) with the destroyer's; not measured.
* `LERI-PLAIN-UNLINK-MEMBER`: a plain foreign unlink `mk_list_del(&e->F)` is
  not compared with the element's `container_of` member (P-CRR-MEMBER's
  remaining part).

The increment paragraph is in the amendments file (the integrator's
`apply_amendments.py` prepends it; writing it into `wave-48-part12.md` here
would make `--check` refuse the file as already applied). No evaluation
profile's baseline changes here (`profiles: []`): the figures are on this
card's preparations.
