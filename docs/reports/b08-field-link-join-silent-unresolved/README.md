# B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: partial field links

Soundness repair, wave 48 round 6, registered by
[CG04-LOOP-SITE-GENERATIONS](../cg04-loop-site-generations/README.md) §7 and
approved by the user with priority. Base: local `main` = `a235ead6` (the
brief's base).

- **Rule:** [gateway 11c](../../agents/gateway.md#constraint-11c), new;
  [5k](../../agents/gateway.md#constraint-5k) and
  [11a](../../agents/gateway.md#constraint-11a) amended by reference.
- **Lean:** `QCore.FieldLinkJoin`, `proof/AuditFieldLinkJoin.lean`
  (21 theorems).
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen at `0ee4676a`
  (deviation note inside: the repair was written and fixture-tested, not run
  on any corpus input, before it was frozen).

## 1. The defect, reproduced

`AbstractState::join_field_links` (`legacy_c.rs`) recorded every child
linked on one side of a join as an unresolved effect and nothing else; its
comment claimed that made the exit `Unknown`, which nothing did. The escape
cascade (`cascade_field_links_on_escape`) turned an exactly-live child
`ESCAPED` whether or not the link held on every path, and left a mixed-status
child unresolved silently. The new fixtures (`tests/field_link_join.rs`), run
on the base with the repair absent:

| Shape (after the shared prologue) | Base | Now | Concretely |
|---|---|---|---|
| `if (n) { b->item = it; } else { k_release(it); } k_drop(b);` (the ledger's) | **Safe** | Unsafe, the straight-line form's leak | the linking arm leaks `it` (P-B08-SHALLOW) |
| `if (n) { b->item = it; } return b;` | **Safe** | Unknown (escape refused) | the other arm leaks `it` |
| `b->item = it; if (n) { k_release(it); } return b;` | **Safe** | Unknown (escape refused) | the member dangles on one arm |
| a caller of `k_build` (the one-sided escape above) | **Safe**, `k_build`'s W4 summary validated | not Safe, no summary | as above |
| linked on both arms, `k_release(b->item); k_drop(b);` | Safe | Safe | control |
| linked on both arms, `return b;` | Safe | Safe | control |

The ledger's claim reproduced exactly. **Correction to the input:** the
ledger said `cascade_field_links_on_escape` "withholds the same way"; the
worse case is that the cascade *discharged* a one-sided live child (the
second row): nothing is withheld at the exit, so CG04's withheld-obligation
census could not see it. This card's escape census closes that blind spot
(§2).

## 2. The class, audited

Every place in route G that inserts into `unresolved_effect_sites` at base
`a235ead6`, and whether the same transfer reports a path-poisoning
diagnostic (so the path is `unmodeled` and the function at least
`Unknown`/`Pending`):

| Site | File:line | Reports |
|---|---|---|
| `AbstractState::join` (union of both sides) | `legacy_c.rs:10072` | propagation only; each side's insertion reported on its own path |
| **`join_field_links`**, symmetric difference | `legacy_c.rs:10491` | **silent: the defect** |
| **`cascade_field_links_on_escape`**, `Some(_)` arm; its `Some(LIVE)` arm escaped a one-sided child | `legacy_c.rs:14184`, `:14175` | **silent: the defect** |
| `take_element_result` (two arms) | `legacy_c.rs:12465`, `:12470` | yes: `refuse_element` (`UnknownAlias`) |
| out slot publishing (CTWIN-PUBLISH-OUT), static publication (CTWIN-PUBLISH), indirect publication | `legacy_c.rs:12794`, `:12866`, `:13626` | yes: `reject_publication` (`Unsupported`) |
| derived store through an out slot; derived member store | `legacy_c.rs:12908`, `:13724` | yes: `UnknownAlias` |
| pointer step losing the only name | `legacy_c.rs:13258` | yes: `UnknownAlias` |
| `apply_resource_action`: ambiguous or nullable target; ambiguous cascade; published escape | `legacy_c.rs:13994`, `:14005`, `:14027` | yes: `UnknownAlias`/`Unsupported` |
| reallocation input invalid; reallocated result invalid; derived owner; derived result via out slot | `legacy_c.rs:14267`, `:14461`, `:14324`, `:14498` | yes: `UnknownAlias` |
| `derived_custody_refusal`, `refuse_kind_mismatch` | `legacy_c.rs:14642`, `:14691` | yes: `UnknownAlias` (their accusable arms insert nothing) |
| `attach_contained_resource` (not live; ambiguous) | `legacy_c.rs:14907`, `:14919` | yes: `require_live_resource` / `UnknownAlias` |
| sub-object store's refused call temporary | `legacy_sub_object_store.rs:491` | yes: the fallback's refusal |
| container publication `refuse`; owner of linked objects | `legacy_container_publication.rs:599`, `:606`, `:726` | yes: `Unsupported` |
| refused hand-on (`refused_operands::apply`); refused call (`withdraw_refused_call`) | `legacy_refused_operands.rs:481` | yes: only after `Unsupported`; `UnknownAlias` |
| rule 11a rename; shift refusal; `settle_refused` | `legacy_loop_generations.rs:174`, `:260`, `:302` | propagation; `Unsupported`; only when the node's own diagnostics poison the path |

Readers of the set: `report_leaks` (`legacy_c.rs:15216`, skips an unresolved
identity), `site_disposition` (`:10846`, `Mixed` in a summary observation),
`observe_exit` (`:10973`), the settled report passes (`:11177`, `:11956`,
`:12038`), the exactly-live predicates (`:10380`, `:13810`–`:13924`,
`:15045`), `joined_provenance` (`legacy_joined_provenance.rs:571`, and `:599`
which clears the mark only with its `UnknownAlias` kept), `settled_site`
(`legacy_container_publication.rs:626`) and the leak-witness census. Summary
inference (`observations_match_contract`, `legacy_summary.rs:1077`) validates
only a body with no diagnostic at all, so it inherits whatever the flow
reports. W4 summary application is an ordinary contract call. **Silent
insertions found: the two B08 sites only.** No other defect of the class; no
different defect found.

## 3. Exposure (measured before the repair)

Runs at `a235ead6` (BEFORE) and with a behaviour-neutral census build
(`dc296d00`, parity rows only), `--jobs 8`, [measure.sh](scripts/measure.sh):

- CG04's withheld-obligation census
  ([withheld.py](../cg04-loop-site-generations/scripts/withheld.py)): **0**
  rows in corpus, system and slice (the ledger's figure reproduces).
- The escape census ([escape.py](scripts/escape.py), parity
  `position: "field-link-escape"`): corpus **0**, system **0**, slice 8 rows
  in `flb_create` (Unknown): a must-linked, exactly-live child that already
  carries a reported refusal's unresolved effect. **0 Safe functions.**

## 4. The repair (gateway rule 11c)

`field_link_join.rs` (the decision: `join_link`, `escape_step`,
`exit_refuses`, and the tables shared with Lean) and the route-G adapter
`legacy_field_link_join.rs`, with thin hooks in `legacy_c.rs` and
`legacy_loop_generations.rs`.

- **Join.** The union of the links is kept; each `(owner, field, child)`
  that does not hold on both sides is recorded in a new
  `AbstractState::partial_links` (a subset of `field_links`). The join adds
  **no** unresolved effect, so the one-sided child's obligation is what the
  paths' statuses say: after the owner's release (no cascade,
  P-B08-SHALLOW) a possibly-live child is reported at the exit, exactly as in
  the straight-line form. A store makes its link hold again; `prune` after
  each transfer keeps the subset; a generation shift renames partial triples
  with the links.
- **Escape.** Each child reached is decided by `escape_step`: exactly live
  and linked on every path escapes; escaped or untracked is kept; live on
  no path (released, or contained and so its container's) is withdrawn as
  before (nothing this escape owes; a contained child later detached live
  meets the floor); any other
  possibly-live child (one-sided, or of mixed status) is refused
  (`Unsupported`, "field-link escape: …") and keeps its status with an
  unresolved effect.
- **Reads.** `bound_read_resolves` and the chain view decline a partial link.
- **Floor.** At every settled exit, a path no refusal reached (`unmodeled`
  false) that still withholds a possibly-live identity with an unresolved
  effect, not the caller's, reports a refusal ("exit withholds …"). This
  turns any future silent insertion into `Unknown`, and a W4 summary over
  such a body is not validated.

**Choice, stated:** precision where cheap. The ledger's fixture now reports
the leak (Unsafe) rather than only refusing; an escape that cannot tell which
path the live bit comes from refuses (the conservative reading, never an
accusation). The one correct program the escape refuses
(`released-arm-escape`) is registered as
`FIELD-LINK-ESCAPE-PATH-CORRELATION`. Removing the join's mark makes the
exactly-live predicates see a one-sided child as its paths' statuses say;
its link is only consulted by the escape and by member reads, which decline
it. Measured: this moved nothing (§5).

**Shared foundations** (`elaboration.rs`, `typed_place.rs`,
`call_instantiation.rs`): **untouched**. Added to `legacy_c.rs`: the field
`AbstractState::partial_links`; hooks in `join_field_links` (body moved to
the adapter), the field store (`stored`), `bound_read_resolves`,
`ChainView::field_link`, `apply_resource_action` (census and the escape's
report), `cascade_resource_action`/`cascade_field_links_on_escape` (now
return the refused children), the work list (`prune`) and the settled exit
pass (`exit_backstop`); `legacy_loop_generations.rs::rename` calls the
adapter's `rename`. `lib.rs` declares `field_link_join`.

## 5. Measurements

Runs `private/b08fl/{before,census,after}*` of this worktree and
`/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/b08fl-*`,
every exit 0, `--jobs 8`; binaries in
[artifacts/binaries.txt](artifacts/binaries.txt) (BEFORE at `a235ead6`,
census at `dc296d00`, AFTER built at `277707ec`, whose code tree every later
commit keeps).

| Route | BEFORE | census | AFTER |
|---|---|---|---|
| corpus (`66910c10`) | 602 / 10 / 4,222, `2fc3af8c…` (the brief's digest) | `2fc3af8c…` | **602 / 10 / 4,222, `2fc3af8c…`** |
| system (`046accf6`) | 37 / 2 / 195, `60f636af…` | `60f636af…` | **`60f636af…`** |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending, `a4cfa035…` | `a4cfa035…` | **`a4cfa035…`** |

Every report is byte-identical ([artifacts/totals.txt](artifacts/totals.txt),
[compare-after*.json](artifacts/)): **no function moves, no leaf changes, no
Safe → non-Safe move, so there is no correction to name.** On the AFTER, the
withheld census has 0 rows everywhere (the floor never fired) and the escape
census has no `partial` row; `flb_create`'s 8 rows decide `escape`, as on the
base ([census-row-counts.txt](artifacts/census-row-counts.txt)). As with rule
11a, the repair closes the hole for code the widening cards admit
(CG10-BOUND-UNTRACKED-ROOTS merges it and re-takes its BEFORE on it).

## 6. Prediction against outcome

| Frozen | Outcome |
|---|---|
| Corpus: no Safe → non-Safe | **held** |
| Corpus: Unknown leaf changes possible, Unknown → Unsafe possible, no Unknown → Safe | **held**; none of the possible changes occurred (byte-identical) |
| Floor adds 0 leaves | **held** |
| System: no move, no leaf change | **held** |
| Slice: no move; `flb_create` may gain the escape refusal | **held**; it did not (its child is must-linked and exactly live) |

## 7. Tests

- `tests/field_link_join.rs` (7 tests): the ledger's fixture reports the
  straight-line leak (same leak line and acquisition), the controls stay
  Safe, the one-sided and released-arm escapes refuse, the mixed escape
  refuses, the summary is not validated and its caller not Safe, and the
  pinned shapes decide as the Lean model (`SHAPE_VECTORS` bound to
  `shapeVectors`).
- `tests/unit/field_link_join.rs`: the three tables against Lean, and all 64
  escape inputs (a possibly-live child is escaped or refused, escaped only
  exactly live on every path).
- `tests/unit/legacy_field_link_join.rs` (13 tests): each arm of the join,
  store, prune, rename, escape and floor.
- **Flipped deliberately:**
  `loop_site_generations::the_field_link_join_sibling_is_pinned` →
  `the_field_link_join_sibling_stays_fixed`: the branch form is Unsafe with
  the straight-line form's leak; the loop form moves from Unknown (a rule-11a
  refusal fed by the join's mark) to **Unsafe** with a leak, the source's own
  (every stored generation but the last is overwritten in the member, and
  the last leaks with the owner's release). The reasons are in the test.

## 8. Lean

`QCore.FieldLinkJoin` (21 theorems, audited, no placeholders, no project
axioms): the link join (`link_vectors_hold`, `a_one_sided_link_is_partial`);
the escape step (`escape_vectors_hold`,
`escape_only_an_exactly_live_must_child`,
`a_possibly_live_child_is_escaped_or_refused`,
`an_escape_over_a_one_sided_live_child_refuses`); two concrete paths and
their abstraction (`a_decided_escape_leaves_no_live_child`,
`the_live_bit_names_a_concrete_live_path`); the join
(`the_join_adds_no_unresolved`, `a_live_child_at_exit_is_reported`); the
floor (`exit_vectors_hold`,
`an_unresolved_possibly_live_obligation_at_exit_is_never_safe`, over the
four-valued fold); the shapes (`the_model_is_sound`,
`the_model_accuses_only_concrete_leaks`, `the_base_flow_hides_the_leak`,
`the_branch_reports_the_straight_line_leak`, `shape_vectors_hold`, bound to
the gateway's verdicts). Premises stated, not proved: P-B08-SHALLOW; that
every rule that marks an identity unresolved reports a path-poisoning
diagnostic on that path (§2's audit; the floor makes a violation on a
modeled path a refusal); faithful extraction (trust boundary 28).

## 9. Gate

[scripts/gate.sh](scripts/gate.sh), logs `private/b08fl/gate`; exits in
[gate/](gate/), figures in [gate.json](gate.json). GATE-PLACEHOLDER

## 10. Ledger

- **Retired:** `B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED`. Its DISCHARGE: a
  join and an escape cascade that leave an obligation unresolved report a
  refusal (the join now leaves none, the escape refuses, and the floor
  refuses any silent one at a settled exit), with Lean parity
  (`QCore.FieldLinkJoin`), the pinned branch row flipped, and a measured
  re-pin naming every Safe → non-Safe move (none, on corpus, system and
  slice). Every clause holds.
- **Registered:** `FIELD-LINK-ESCAPE-PATH-CORRELATION` (a correct one-sided
  escape is refused; 0 measured functions). Ledger 657 → 657.
