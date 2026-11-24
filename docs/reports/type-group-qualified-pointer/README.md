# TYPE-GROUP-QUALIFIED-POINTER: a qualifier after a pointer `*` keeps a group a type group

Card TYPE-GROUP-QUALIFIED-POINTER (USER DECISION 2026-11-24: "launch the
TYPE-GROUP-QUALIFIED-POINTER card"). Base `7bfa0414b`, the tip of
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST (rule 74r, approved as r72), which
sits on rule 74m (r68) and main `3ca85c7a6`. [PREDICTION.md](PREDICTION.md)
was committed (`fb151beaa`) before the implementation (`8ed7a7a7f`). This adds
gateway rule [74u](../../agents/gateway.md#constraint-74u), amending rule
12h's shared `type_group_open`, and `QCore.TypeGroupQualifiedPointer` with
its audit.

## 1. Census of the token type-group and cast readers

Every token reader in `crates/qcore-c-gateway/src` that decides whether a
parenthesised group is a cast or a type name, found by a search for cast,
type-group and uncast functions and by the rules that name them (12h, 12y,
74f, 74g, 74h, 74m, 74r). "Accepts" means it reads the group as a
cast/type. The Clang-matched readers (`uncast_tokens`, `explicit_casts`,
`classify_node`, `void_view`, `legacy_cast_representation`) take the type
from Clang's AST and have no token grammar, so they are not listed.

| reader | grammar at the base | `* const` etc. after `*` | `const T *` | `T * const *`, `T ** const` | `_Atomic` | gap direction |
|---|---|---|---|---|---|---|
| `cast_semantics::type_group_open` (rule 12h(4), 74m, 74r) | words, then trailing `*`s | no | yes | no | as a leading word only | **admitting** (fixed) |
| `ampersand_runs::may_close_a_cast` (rule 74f, 74g, 74h) | names and `*` anywhere, nested declarator groups | yes | yes | yes | yes, also `_Atomic ( T )` | none |
| `legacy_return_family::is_cast_group` / `cast_target` (rule 74h, family K) | resolved words, then trailing `*`s | no | yes, if resolved | no | leading only | refusing (kept, follow-up) |
| `legacy_return_family::cast_type_word` (rule 74r) | names and `*` after the root | yes | yes | yes | yes | none |
| `loop_owner_holder_opaque::cast_start` | names and `*` anywhere | yes | yes | yes | yes | none |
| `legacy_token_shapes::is_leading_cast` | any balanced group without call, `=`, `,`, brackets | yes | yes | yes | yes | none |
| `legacy_slot_move_borrow::is_type_group` | names and `*` anywhere, first a name | yes | yes | yes | yes | none |
| `legacy_descriptor_custody::called_after_cast` | any group | yes | yes | yes | yes | none |
| `legacy_token_shapes::cast_place` (rule 12y) | names and `*`, last a `*` | no | yes | `T * const *` yes, `T ** const` no | leading only | refusing (follow-up) |
| `legacy_sub_object_store::strip_pointer_cast` | names and `*`, last a `*` | no | yes | as `cast_place` | leading only | refusing (follow-up) |
| `legacy_reference_result::uncast` | names and `*`, last a `*` | no | yes | as `cast_place` | leading only | refusing (follow-up) |
| `legacy_cast_return::cast_of_operand` | names and `*` anywhere; pointer target only when the last token is `*` | read, but classed `Other` | yes | as `cast_place` | yes | refusing (follow-up) |

`_Nonnull`-style annotations: Clang 21 accepts `_Nonnull`, `_Nullable`,
`_Nullable_result` and `_Null_unspecified` after a `*` in C (checked with
`clang -fsyntax-only` on a scratch file, together with `const`, `volatile`,
`restrict`, `__restrict`, `__restrict__`, `__const` and `__volatile__`).
`*(long * _Atomic)p` is rejected by Clang (an atomic pointer cannot be
dereferenced directly), but `( long * _Atomic )` is a valid type name. Every
reader above reads these as plain names, so the table's "after `*`" column
covers them.

**Classification.** Only `type_group_open` admits through its gap: its three
callers read through a group it accepts, and stopping early reads less (rule
74r's base missed, rule 12h(4)'s and rule 74m's counts too small). Every
other gap refuses: `is_cast_group` leaves the `*`/`&` after the group
undecided (rule 74h keeps both readings' refusals) and reads `( T * const ) (`
as a call; `cast_place`, `strip_pointer_cast` and `uncast` return no place or
no null constant, which their callers refuse; `cast_of_operand` classes the
target `Other`, so a place under it is refused or left to the pre-card
routes. None of these has `type_group_open`'s shape (a reader of an
operand's type group that its callers read through), so they are registered
rather than changed.

## 2. Change

* **`cast_semantics.rs`.** `type_group_open` keeps its nearest-`(` search and
  now asks the new `type_group`: one or more words (the specifier-qualifier
  list, read as identifiers exactly as before), then any number of `*`, each
  followed by words of the new `POINTER_QUALIFIERS` only (`const`,
  `volatile`, `restrict`, `_Atomic`, `__const`, `__const__`, `__volatile`,
  `__volatile__`, `__restrict`, `__restrict__`, `_Nonnull`, `_Nullable`,
  `_Nullable_result`, `_Null_unspecified`). This is C11 6.7.7's
  `specifier-qualifier-list pointer_opt`, with 6.7.6.1's `pointer` as
  `* type-qualifier-list_opt pointer_opt`. A word after a `*` that is no
  qualifier (`( a * b )`) still makes no group; restricting the words to the
  qualifier set keeps a product of two names from being read as a type.
  No other source file changes; `legacy_return_family.rs` is untouched.
* **Kept detectors.** No second detector. `is_cast_group` and the four
  refusing readers keep their readings (§1, follow-ups in §8).

## 3. Each consumer's direction

`type_group_open` has two call sites, both in family S of
`legacy_return_family` (`stars_before`, `dereferenced_through_groups`), and
three readings depend on them:

* **Rule 12h(4), a may-bind member or subscript root** (`stars_before` with
  `closing` = `usize::MAX`, cast reading, `through`). Admitted only when the
  count is 0. Reading through more groups can only find more `*`s
  (`count_mono`), so a root admitted at the tip was admitted at the base
  (`member_admission_shrinks`). **Stricter, and reachable**:
  `* ( long * const ) q - > f` (and `q [ 0 ]`) was admitted with
  `bases: ["q"]` at the base although the outer `*` dereferences the value
  `q->f` reads, a second dereference rule 12h(4) refuses for `( long * )`. It
  now refuses (`a_member_root_through_the_group_refuses`). End to end, with
  `q` an unbound formal, `*(long * const)q->f` is `Safe` at both base and tip,
  as `*(long *)q->f` is: an older route decides it before family S.
* **Rule 74m, a starred may-bind root** (count one without and with
  `through`, under both of rule 74h's readings). The through count grows and
  the inner count does not use the detector, so an admission at the tip was
  one at the base (`starred_admission_shrinks`). **Stricter in the model, no
  reachable change**: a `*` right after a qualified group is undecided
  (`is_cast_group` does not resolve it), so the operand reading counts none
  and refuses; a `(` right after it is read as a call. So
  `*(long * const)*q` and `*(long * const)(long *)*q` refused at the base and
  still do. The brief's pin of `*(long * const)*q` as refused holds at both;
  §4 says so rather than claiming a base failure for it.
* **Rule 74r, an untracked root's starred base** (`dereferenced_through_groups`).
  Reading through more groups only makes more bases (`through_mono`,
  `bases_grow`), each an obligation the flow check must discharge, and no
  refusal depends on it. **Stricter, and reachable**: the brief's
  `*(long * const)p`.

Family S's admission composes the three: every may-bind root admitted and
every base's obligation discharged. A more accepting recogniser makes it
admit only what it admitted (`the_fix_only_refuses_more`), and
`qualified_only_refuses_more` instantiates it with the base and tip
recognisers via `old_le_new`. No consumer admits more, so no reading is kept
back for the newly recognised groups, and no `Unknown` moves to `Safe`.
Rule 74h's undecided groups are unaffected: whether a `*` may be unary is
`operator_at`'s answer (`is_cast_group` and `may_close_a_cast`), not
`type_group_open`'s, and the model takes it as an input per reading.

## 4. Pins

* **Unit** (`tests/unit/cast_semantics_type_group.rs`, a new module of
  `cast_semantics`, using rule 74r's name oracle):
  * `rule_74u_type_group_vectors_hold` reads `typeGroupVectors` (22 rows) from
    `proof/QCore/TypeGroupQualifiedPointer.lean` and checks each spelling's
    token classes, the base reading, `type_group` and `type_group_open`.
    `(const char * const)`, `(const char * const *)`, `(char ** const)`,
    `(long * const volatile)`, `(long * _Nonnull)`, `(long * _Atomic)` are
    rows; `( a * b )`, `( * const )` and the empty group are not groups.
  * `a_qualifier_list_after_each_star_is_a_type_group`: every
    `POINTER_QUALIFIERS` entry at six positions, the base's groups, and the
    non-groups (`long ( * )`, `_Atomic ( long ) *`, `long * __attribute__`).
  * `an_untracked_root_through_a_qualified_group_is_a_base` (11 spellings),
    `a_may_bind_root_counts_through_a_qualified_group` (four member and
    subscript roots refused), `a_starred_may_bind_root_keeps_its_refusal`
    (`* ( long * const ) * q` refused, rule 74m/74h) and
    `the_base_readings_are_kept`.
  * **Changed deliberately:** rule 74r's `the_base_readings_are_kept` held
    `* ( long * const ) p` at `Some(vec![])` as this card's registered gap,
    and rule 74r's Lean vector `untrackedVectors` held the same row as
    `[.other], false, false`. Both now expect `p` as a base
    (`[.cast, .star .notOperandEnd], false, true`).
* **End to end** (`tests/return_family_type_group.rs`):
  `a_null_untracked_root_through_a_qualified_cast_is_a_base` (`*(long *
  const)p`, `volatile`, `restrict`, `const volatile`, `_Nonnull`, a
  parenthesised cast: `Unknown`, `unknown_alias:p`; and `**(const long *
  const *)pp`: `Unknown`, `unknown_alias:pp`) and `the_base_readings_are_kept`
  (an unbound formal `Safe`, a tracked root `Unsafe` with `leak`, or with
  `UseAfterRelease` + `unknown_alias:p` after a release, a value cast `Safe`,
  and `*(long * const)*q` / `*(long *)*q` with a formal `q` `Unknown` with
  the return refusal).

**Checked at the base.** With the implementation committed, `type_group`'s
last clause was changed to accept only `*`s after the words, which is exactly
the base's reading, and the new and changed tests were run (logs in the
worktree's `private/tgq/base-fixtures/`). The source was then restored byte
for byte from a saved copy (`cmp` clean, then `touch`).

* `unit.log`: `rule_74u_type_group_vectors_hold` (`tip: long * const`),
  `a_qualifier_list_after_each_star_is_a_type_group`,
  `an_untracked_root_through_a_qualified_group_is_a_base` (`Some([])`),
  `a_may_bind_root_counts_through_a_qualified_group`
  (`* ( long * const ) q - > f` gives `Some(["q"])`), and rule 74r's
  `the_base_readings_are_kept` and `rule_74r_untracked_vectors_hold` fail.
  `a_starred_may_bind_root_keeps_its_refusal` and this card's
  `the_base_readings_are_kept` pass, as §3 says they must; so do the 29
  other `return_family` and `cast_semantics` unit tests.
* `e2e.log`: `a_null_untracked_root_through_a_qualified_cast_is_a_base`
  fails at its first row with `("Safe", [])`; `the_base_readings_are_kept`
  passes.

**Other pins:** none moved. No contract, lock, submodule or CT pin is touched.

## 5. Lean

`QCore.TypeGroupQualifiedPointer` and
`proof/AuditTypeGroupQualifiedPointer.lean`, with no imports, placeholders or
project axioms. It states the recogniser (`accepts`, an automaton over token
classes) and the base's (`oldAccepts`), the grammar (`Pointer`, C11 6.7.6.1's
right-recursive `pointer`, and `TypeName`), and proves `accepts_iff`
(recogniser = grammar), `old_le_new`, and the witnesses
`the_qualified_group_is_new` and `a_product_is_no_group`. The consumers are
abstract walks over steps back from a root (`count` with the `closing` bound,
`through`), each proved monotone in the recogniser, composed in `Admits`
(`the_fix_only_refuses_more`, `qualified_only_refuses_more`), with the
witnesses `old_misses_the_qualified_group`, `a_second_dereference_refuses`
(the count alone; §3 explains why the reader already refuses that spelling)
and `a_member_root_through_the_group_refuses`. Token classes and the steps
back are the Rust reader's output (trust boundary 28), bound by
`typeGroupVectors`.

## 6. Measurement against the prediction

MEASUREMENT-PENDING

## 7. Gate

GATE-PENDING

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) (schema
`quirin-amendments/2`, not applied) retires TYPE-GROUP-QUALIFIED-POINTER and
registers three follow-ups. `status_edits` is empty: STATUS states no fact
this card changes. The id it retires is registered by
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST's amendments (r72), which sit on
r68's, both committed in this tree but not applied. `--check` against this
tree is therefore refused (`no registered follow-up has this id`,
`private/tgq/amend-check-tree.log`). In a `git archive` of this card's head
with r68's and then r72's amendments applied first (`--root
private/tgq/amend-root`, both exit 0), it exits 0
(`private/tgq/amend-check.log`).

**Follow-ups registered:**

* **RETURN-FAMILY-RESOLVED-QUALIFIED-CAST.** `is_cast_group`/`cast_target`
  do not resolve a qualified group. Refusing; resolving it would admit more,
  so it needs its own soundness argument and measurement.
* **TYPE-GROUP-NESTED-DECLARATOR.** `type_group_open` still reads no group
  with parentheses or brackets (`( int ( * ) [ 4 ] )`, a function pointer,
  `_Atomic ( T )`, `__typeof__`, an attribute): the same admitting shape.
* **TOKEN-CAST-READERS-QUALIFIED-POINTER.** `cast_place`,
  `strip_pointer_cast`, `legacy_reference_result::uncast` and
  `cast_of_operand` refuse a qualified pointer cast (precision only).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/cast_semantics.rs`: `type_group_open`'s body
  and doc, the new `POINTER_QUALIFIERS` and `type_group` right after it, and
  one test `mod` line at the end. The parallel card
  RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (rule 74t) calls
  `type_group_open` from `legacy_return_family.rs`; it will now also read
  qualified groups, which only strengthens its new bases, but its own pins of
  a qualified group (if any) would change.
* `crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_deref_cast.rs`
  (the one pin line) and `proof/QCore/ReturnFamilyUntrackedDerefCast.lean`
  (one vector row).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74u.md`, `tests/return_family_type_group.rs`,
`tests/unit/cast_semantics_type_group.rs`,
`proof/QCore/TypeGroupQualifiedPointer.lean`,
`proof/AuditTypeGroupQualifiedPointer.lean`, and this directory.
