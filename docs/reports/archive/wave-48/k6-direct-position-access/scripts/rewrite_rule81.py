#!/usr/bin/env python3
"""Card K6: restate rule 81's conditions over the K6 block (one-off edit helper).

usage: rewrite_rule81.py REPO_ROOT
"""
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
path = root / "docs/agents/rules/81.md"
text = path.read_text()
start = text.index("    is no value of the program. Stated as formulas over the occurrence `#a`")
end = text.index("    Both readings only remove a binding or a refusal, and only where the")
new = """    is no value of the program. Every condition and refusal is a formula of
    the C04.18.9 typed rendering over the K6 block (`k6_`-prefixed terms,
    sorts `k6_amp`, `k6_member`, `k6_place`), checked by the premise-formula
    validator `guarded_proposition` (`address_arrow_place::FORMULAS`,
    `crates/qcore-frontend/tests/rule81_formulas.rs`); prose glosses each.
    (1) **`may_bind`.** `k6_exempt_is_the_shape` and
    `k6_the_shape_is_exempt`:
    `forall (#a: k6_amp) { (#k6_exempt(#a) => ((((#k6_opens_group(#a) && #k6_name_arrow(#a)) && #k6_selection_chain(#a)) && #k6_group_arrow(#a)) && !#k6_outer_address(#a))) }`
    and its converse (the `&` directly follows `(`, the name is followed by
    `->`, the tokens up to the group's `)` are member selections and
    balanced subscripts, `->` follows the group, and reading back from the
    group through `(` meets no `&`).
    `k6_exempt_binds_as_the_plain_spelling`:
    `forall (#a: k6_amp) { (#k6_exempt(#a) => (!#k6_binds(#a) && !#k6_plain_binds(#a))) }`;
    `k6_at_least_the_plain_reader`:
    `forall (#a: k6_amp) { (#k6_plain_binds(#a) => #k6_binds(#a)) }`;
    `k6_only_removes_bindings`:
    `forall (#a: k6_amp) { (#k6_binds(#a) => #k6_old_binds(#a)) }`.
    Refusals: `k6_refuse_outer_address`
    `forall (#a: k6_amp) { (#k6_outer_address(#a) => #k6_binds(#a)) }`
    (`&((&p->q)->f)` is `&p->q.f`, which the plain arm binds),
    `k6_refuse_whole_address`
    `forall (#a: k6_amp) { (!#k6_name_arrow(#a) => #k6_binds(#a)) }`,
    `k6_refuse_operator_in_group`
    `forall (#a: k6_amp) { (!#k6_selection_chain(#a) => #k6_binds(#a)) }`,
    `k6_refuse_undereferenced_group`
    `forall (#a: k6_amp) { (!#k6_group_arrow(#a) => #k6_binds(#a)) }` and
    `k6_refuse_no_group`
    `forall (#a: k6_amp) { (!#k6_opens_group(#a) => #k6_binds(#a)) }`.
    (2) **Places.** `k6_place_is_the_plain_place`:
    `forall (#e: k6_member) { (#k6_over_member_address(#e) => (#k6_place(#e) == #k6_plain_place(#e) && #k6_root(#k6_place(#e)) == #k6_root(#k6_plain_place(#e)))) }`
    (the same spelling, root and indirection as `X.f`);
    `k6_place_only_adds`:
    `forall (#e: k6_member) { (!#k6_old_place(#e) == #k6_refused => #k6_place(#e) == #k6_old_place(#e)) }`;
    refusal `k6_refuse_whole_object_address`:
    `forall (#e: k6_member) { ((#k6_over_address(#e) && !#k6_over_member_address(#e)) => #k6_place(#e) == #k6_refused) }`
    (`(&s)->f`); a `.` over an address keeps the refusal too.
"""
text = text[:start] + new + text[end:]
text = text.replace(
    "    (`new_place_is_c_place`, `the_rule_only_adds`,",
    "    (one definition per K6 term, `k6Terms`, and one theorem per formula,\n"
    "    `formula_*`; `new_place_sound`, `the_rule_only_adds`,")
text = text.replace(
    "    `a_lifted_refusal_is_an_arrow_over_an_address`, `the_rule_only_removes`,",
    "    `a_whole_object_address_is_refused`,\n"
    "    `a_lifted_refusal_is_an_arrow_over_an_address`, `the_rule_only_removes`,")
path.write_text(text)
print("ok")
