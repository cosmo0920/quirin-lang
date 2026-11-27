#!/usr/bin/env python3
"""Card K6: restate the README's rule 81 formulas over the K6 block (one-off edit helper).

usage: rewrite_readme.py REPO_ROOT
"""
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
path = root / "docs/reports/k6-direct-position-access/README.md"
text = path.read_text()


def between(start, end, new):
    global text
    a = text.index(start)
    b = text.index(end, a)
    text = text[:a] + new + text[b:]


between("1. **The lexical `may_bind` reader**", "**Soundness.**", """1. **The lexical `may_bind` reader** (`typed_field_access::bound_names`,
   rule 5b's premise for a pointer handed out of the body, rule 10k (2)) read
   the `& p` of `( & p -> q ) -> f` as an address of `p`.
2. **The elaborator's place reader** (`elaboration::path_place`) refused a
   path through the `&`; rule 81 reads it through rule 10n's own
   `arrow_through_address` (one shared item).

Each condition and refusal is a formula of the C04.18.9 typed rendering over
the **K6 block** (16 `k6_`-prefixed terms, sorts `k6_amp`, `k6_member`,
`k6_place`; `RENDERED_TERMS`/`RENDERED_VALUE_SORTS` in
`qcore_frontend::premise_formula`, the contract specification's C04.18.9 K6
table, Lean `k6Terms`/`k6Sorts`), checked by `guarded_proposition`
(`crates/qcore-frontend/tests/rule81_formulas.rs`,
`address_arrow_place::tests`); prose is refused:

| Id | Formula |
|---|---|
| `k6_exempt_is_the_shape` | `forall (#a: k6_amp) { (#k6_exempt(#a) => ((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a))) }` |
| `k6_the_shape_is_exempt` | `forall (#a: k6_amp) { (((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a)) => #k6_exempt(#a)) }` |
| `k6_exempt_binds_as_the_plain_spelling` | `forall (#a: k6_amp) { (#k6_exempt(#a) => (!#k6_binds(#a) && !#k6_plain_binds(#a))) }` |
| `k6_at_least_the_plain_reader` | `forall (#a: k6_amp) { (#k6_plain_binds(#a) => #k6_binds(#a)) }` |
| `k6_only_removes_bindings` | `forall (#a: k6_amp) { (#k6_binds(#a) => #k6_old_binds(#a)) }` |
| `k6_place_is_the_plain_place` | `forall (#e: k6_member) { (#k6_over_member_address(#e) => (#k6_place(#e) == #k6_plain_place(#e) && #k6_root(#k6_place(#e)) == #k6_root(#k6_plain_place(#e)))) }` |
| `k6_place_only_adds` | `forall (#e: k6_member) { (!#k6_old_place(#e) == #k6_refused => #k6_place(#e) == #k6_old_place(#e)) }` |
| `k6_refuse_outer_address` | `forall (#a: k6_amp) { (#k6_outer_address(#a) => #k6_binds(#a)) }` |
| `k6_refuse_whole_address` | `forall (#a: k6_amp) { (!#k6_name_arrow(#a) => #k6_binds(#a)) }` |
| `k6_refuse_operator_in_group` | `forall (#a: k6_amp) { (!#k6_selection_chain(#a) => #k6_binds(#a)) }` |
| `k6_refuse_undereferenced_group` | `forall (#a: k6_amp) { (!#k6_group_arrow(#a) => #k6_binds(#a)) }` |
| `k6_refuse_no_group` | `forall (#a: k6_amp) { (!#k6_opens_group(#a) => #k6_binds(#a)) }` |
| `k6_refuse_whole_object_address` | `forall (#e: k6_member) { ((#k6_over_address(#e) && !#k6_over_member_address(#e)) => #k6_place(#e) == #k6_refused) }` |

""")

between("**Lean.** `QCore.AddressArrowPlace`", "**Fixtures**", """**Lean.** `QCore.AddressArrowPlace` (`proof/AuditAddressArrowPlace.lean`, 26
theorems; axioms `propext`, `Quot.sound` only, no project axiom). One
definition per K6 term (`k6_binds` is `newBinds`, `k6_place` is `newPlace`,
`k6_plain_place` reads `(&X)->f` as `X.f`, …; `k6Terms` and `k6Sorts` are the
Rust block, bound by `address_arrow_place::tests::the_k6_block_is_lean_s`) and
one theorem per formula (`formula_k6_*`; `formulaTable` is the Rust table, row
for row). Underneath: every answer of the place reader is C's designation
(`new_place_sound`), it keeps every old answer (`the_rule_only_adds`), reads
`(&X)->f` as `X.f` with the same root for a member or subscript place
(`arrow_through_address_is_dot`, `same_root_as_plain`) and refuses `(&s)->f`
(`a_whole_object_address_is_refused`); the `may_bind` arm only removes
(`the_rule_only_removes`), never binds less than the plain spelling
(`at_least_the_plain_reader`), agrees with it on every dereferenced group
(`agrees_with_the_plain_spelling`), exempts only a member address
(`an_exempted_address_is_a_member_address`), keeps a whole address and an
outer address (`a_whole_address_is_bound`,
`an_outer_address_keeps_the_binding`); `lexical_vectors_hold` replays the
Rust `LEXICAL_VECTORS`.

""")

between("## 4. Refused shapes and their named reasons", "## 5. Shared-file hook points", """## 4. Refused shapes and their named reasons

Rule 81's own refusals are the `k6_refuse_*` formulas of section 3. The rows
below are refused by other rules, each under that rule's own named refusal
(not restated here); this card only classifies the rows freed alone on 11-24:

| Shape | Rows | Refused by | Owner |
|---|---:|---|---|
| interior address returned | 10 | rules 5j, 10k (2), 10n (1): an interior address is a derived result | W45-RETURN-POINTER-RESULT-READS, AG3-RETURN-MEMBER-ADDRESS-DERIVED |
| root handed to a callee before a pointer return | 3 | rule 5b's `may_bind` premise (rule 10k (2)); W48's read-only frame exception not granted | W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT |
| explicit cast of a returned load | 1 | rule 10k (2)'s value forms | K6-RETURN-IDENTITY-CAST |
| switch over a `?:`, typedef base | 1 | the elaborated condition's value forms | K6-SWITCH-CONDITIONAL-TYPEDEF-BASE |
| `_Generic` in a condition; a sibling call beside a deciding operand | 2 | rule 10z (2) and (3) | CG06-VALUE-CONNECTIVE-RESIDUALS |
| pointer difference | 1 | rule 78 | K6-POINTER-DIFFERENCE-SCALAR |
| file-scope record scalar store | 2 | rule 12j's storage boundary | K6-FILE-SCOPE-RECORD-SCALAR-STORE |
| formal copied into a local | 2 | rule 77 | (rule 77, by design) |

""")

text = text.replace(
    "| formula statements | `cargo test -p qcore-frontend --test rule81_formulas` | 0 | 2 passed (`formulas-1.log`) |",
    "| formula statements | `cargo test -p qcore-frontend --test rule81_formulas` | 0 | 3 passed (`formulas-3.log`) |")
text = text.replace(
    "K6-FILE-SCOPE-RECORD-SCALAR-STORE (a user\ndecision on rule 12j's storage boundary), K6-POINTER-DIFFERENCE-SCALAR,\nK6-RULE-FORMULA-VOCABULARY (the closed premise vocabulary has no\ngateway-reading terms, so rule conditions are syntax-checked only).",
    "K6-FILE-SCOPE-RECORD-SCALAR-STORE (a user\ndecision on rule 12j's storage boundary), K6-POINTER-DIFFERENCE-SCALAR.")
path.write_text(text)
print("ok")
