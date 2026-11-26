#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Cards QUIRIN-AUDIT-CONNECT and CONNECT-ADOPT-DECLARED-DEPENDENCIES: print
the CX block of the C04.18.9 typed
rendering (sorts and terms) as the Rust entries of
`crates/qcore-frontend/src/premise_formula.rs` (`--rust-sorts`,
`--rust-terms`) or as the specification's table (`--markdown`). The table
below is the one authority (it extends the frozen QUIRIN-AUDIT-CONNECT copy
with the reliance rule and linkage_contract_v1 terms); the Rust and Lean
blocks were generated from it
and are bound to it by `crates/qcore-frontend/tests/connect_formulas.rs`.
"""

import sys

SORTS = [
    ("cx_step", "a step of a chain: one comparison of a native-comparison record, or of two measured side directories"),
    ("cx_route", "a route of a measurement (`corpus`, `system`, `slice`, `twelve-home`)"),
    ("cx_row", "a row of a route: a function row or a unit row, by its composition key"),
    ("cx_point", "a side of a chain: the before side of its first step, or the after side of a step"),
    ("cx_field", "an identity field a link compares (tool, revision, analyzer, bundles, build commit, link resolution, rule switch, dependency graph)"),
    ("cx_ident", "an identity value: a digest, a revision, a record, or the unrecorded marker"),
    ("cx_verdict", "a verdict of the four-value semigroup (`Safe < Pending < Unknown < Unsafe`)"),
    ("cx_bundle", "a contract bundle: one contract crate on a load path (a provider or plugin-SDK user crate)"),
    ("cx_fn", "a declared function name"),
    ("cx_dep", "a declared contract dependency: a `linkage_contract_v1` link or a `contract inherit` directive, elaborated (contract spec C15.2)"),
]

P, F, V = "P", "F", "V"
TERMS = [
    # Part A: links
    ("cx_next", P, ["cx_step", "cx_step"], None, "`#cx_next(#s,#t)`: step `#t` directly follows step `#s` in the chain"),
    ("cx_joined", P, ["cx_step", "cx_step"], None, "`#cx_joined(#s,#t)`: the link from `#s` to `#t` is admitted (`chain::link` names no refusal)"),
    ("cx_after_id", F, ["cx_step", "cx_field"], "cx_ident", "`#cx_after_id(#s,#f)`: field `#f` of step `#s`'s after side (for the tool, the step's measurement tool)"),
    ("cx_before_id", F, ["cx_step", "cx_field"], "cx_ident", "`#cx_before_id(#s,#f)`: field `#f` of step `#s`'s before side (for the tool, the step's measurement tool)"),
    ("cx_required", P, ["cx_field"], None, "`#cx_required(#f)`: a link needs `#f` recorded on both sides (tool, revision, analyzer, bundles)"),
    ("cx_unrecorded", V, ["cx_ident"], None, "the identity of a field a record does not carry"),
    ("cx_compares", P, ["cx_step", "cx_route"], None, "`#cx_compares(#s,#r)`: step `#s` compares route `#r`"),
    ("cx_after_inputs", F, ["cx_step", "cx_route"], "cx_ident", "`#cx_after_inputs(#s,#r)`: the inputs record of route `#r` on `#s`'s after side (compile-database digests, route digest, spelling, source date)"),
    ("cx_before_inputs", F, ["cx_step", "cx_route"], "cx_ident", "`#cx_before_inputs(#s,#r)`: the inputs record of route `#r` on `#s`'s before side"),
    ("cx_f_tool", V, ["cx_field"], None, "the measurement tool's identity (name, version, SHA-256)"),
    ("cx_f_revision", V, ["cx_field"], None, "the side's revision"),
    ("cx_f_analyzer", V, ["cx_field"], None, "the side's analyzer binary SHA-256"),
    ("cx_f_bundles", V, ["cx_field"], None, "the side's contract-bundle identities"),
    ("cx_f_build_commit", V, ["cx_field"], None, "the side's build commit"),
    ("cx_f_link_resolution", V, ["cx_field"], None, "whether the side ran under its link-resolution artifact"),
    ("cx_f_rules_off", V, ["cx_field"], None, "the side's rule switch (rules off)"),
    ("cx_f_dependency_graph", V, ["cx_field"], None, "the side's contract-dependency graph with its identities (Part C)"),
    ("cx_before", F, ["cx_step"], "cx_point", "`#cx_before(#s)`: step `#s`'s before side"),
    ("cx_after", F, ["cx_step"], "cx_point", "`#cx_after(#s)`: step `#s`'s after side"),
    # Part A: completeness
    ("cx_member", P, ["cx_step"], None, "`#cx_member(#s)`: `#s` is a step of the segment being reconciled"),
    ("cx_chain_complete", P, ["cx_route"], None, "`#cx_chain_complete(#r)`: the chained record calls route `#r` complete in the segment"),
    ("cx_status_complete", P, ["cx_step", "cx_route"], None, "`#cx_status_complete(#s,#r)`: step `#s`'s entry for `#r` has status `complete`"),
    ("cx_unmatched", P, ["cx_step", "cx_route"], None, "`#cx_unmatched(#s,#r)`: step `#s` lists an unmatched function row of `#r`"),
    ("cx_unmatched_units", P, ["cx_step", "cx_route"], None, "`#cx_unmatched_units(#s,#r)`: step `#s` lists an unmatched unit row of `#r`"),
    ("cx_conflicted", P, ["cx_step", "cx_route"], None, "`#cx_conflicted(#s,#r)`: a side of step `#s` has a composition-conflict row of `#r`"),
    ("cx_disagrees", P, ["cx_step", "cx_route"], None, "`#cx_disagrees(#s,#r)`: a row of `#r` that `#s` lists has a before verdict other than the after verdict of its nearest earlier listing"),
    # Part A: rows
    ("cx_row_route", F, ["cx_row"], "cx_route", "`#cx_row_route(#w)`: the route row `#w` belongs to"),
    ("cx_listed", P, ["cx_step", "cx_row"], None, "`#cx_listed(#s,#w)`: step `#s` lists `#w` (moved, content-changed, unmatched, a unit move or unit field difference)"),
    ("cx_verdict", F, ["cx_point", "cx_row"], "cx_verdict", "`#cx_verdict(#p,#w)`: the verdict of `#w` at side `#p`, as that side's report states it"),
    ("cx_first", V, ["cx_point"], None, "the segment's first side"),
    ("cx_last", V, ["cx_point"], None, "the segment's last side"),
    ("cx_net_before", F, ["cx_row"], "cx_verdict", "`#cx_net_before(#w)`: the first verdict of `#w`'s net move"),
    ("cx_net_after", F, ["cx_row"], "cx_verdict", "`#cx_net_after(#w)`: the last verdict of `#w`'s net move"),
    ("cx_moved", P, ["cx_step", "cx_row"], None, "`#cx_moved(#s,#w)`: the chained record lists `#s` among the steps that moved `#w`"),
    ("cx_reported", P, ["cx_row"], None, "`#cx_reported(#w)`: the chained record lists `#w` with its verdict at every side"),
    ("cx_moves", F, ["cx_row"], "integer", "`#cx_moves(#w)`: how many steps moved `#w`"),
    ("cx_cancelled", P, ["cx_row"], None, "`#cx_cancelled(#w)`: the record's pattern of `#w` is `cancelled`"),
    ("cx_cascade", P, ["cx_row"], None, "`#cx_cascade(#w)`: the record's pattern of `#w` is `cascade`"),
    ("cx_evidence_changed", P, ["cx_step", "cx_row"], None, "`#cx_evidence_changed(#s,#w)`: step `#s` lists a change of `#w`'s leaves, fields or contract dependencies"),
    ("cx_published", F, ["cx_point", "cx_row"], "cx_verdict", "`#cx_published(#p,#w)`: the verdict the chained record states for `#w` at `#p`"),
    # Part A: attribution
    ("cx_safe", V, ["cx_verdict"], None, "`Safe`"),
    ("cx_pending", V, ["cx_verdict"], None, "`Pending`"),
    ("cx_unknown", V, ["cx_verdict"], None, "`Unknown`"),
    ("cx_new_safe", P, ["cx_step", "cx_row"], None, "`#cx_new_safe(#s,#w)`: `#s` moves `#w` to `Safe`"),
    ("cx_declared_move", P, ["cx_step", "cx_row"], None, "`#cx_declared_move(#s,#w)`: a prediction given for `#s` (a `--declared` block or the step record's own declared check) covers `#w`'s move"),
    ("cx_unattributed", P, ["cx_step", "cx_row"], None, "`#cx_unattributed(#s,#w)`: the record lists `#w`'s move in `#s` as unattributed"),
    ("cx_review", P, ["cx_step", "cx_row"], None, "`#cx_review(#s,#w)`: the record flags `#w`'s new `Safe` in `#s` for review (`unattributed_new_safe`), failing the run"),
    ("cx_accepted", P, ["cx_step", "cx_row"], None, "`#cx_accepted(#s,#w)`: the record accepts `#w`'s new `Safe` in `#s`"),
    ("cx_dependency_changed", P, ["cx_step", "cx_row"], None, "`#cx_dependency_changed(#s,#w)`: a dependency entry naming a declaration `#w` rests on changed identity in `#s`"),
    ("cx_dependency_listed", P, ["cx_step", "cx_row"], None, "`#cx_dependency_listed(#s,#w)`: the record lists that change beside `#w`'s move (never as its attribution)"),
    # Part B
    ("cx_declares", P, ["cx_bundle", "cx_fn"], None, "`#cx_declares(#b,#g)`: bundle `#b`'s lock records a reviewed declaration of `#g`"),
    ("cx_text", F, ["cx_bundle", "cx_fn"], "cx_ident", "`#cx_text(#b,#g)`: the SHA-256 of `#b`'s declaration of `#g` (normalised text, C06.2)"),
    ("cx_libc_name", P, ["cx_fn"], None, "`#cx_libc_name(#g)`: `#g` is a libc/POSIX name (the libc crate's name list or a libc declaration)"),
    ("cx_libc_owner", P, ["cx_bundle"], None, "`#cx_libc_owner(#b)`: `#b` is `quirin-provider-libc`"),
    ("cx_refused", P, ["cx_bundle", "cx_fn"], None, "`#cx_refused(#b,#g)`: the composition refuses `#b`'s declaration of `#g`"),
    ("cx_stamps_decide", P, ["cx_fn"], None, "`#cx_stamps_decide(#g)`: the pin stamps of `#g`'s declarations decide one in force (C06.6)"),
    ("cx_rank", F, ["cx_verdict"], "integer", "`#cx_rank(#v)`: the verdict's position in `Safe < Pending < Unknown < Unsafe` (0 to 3)"),
    ("cx_contribution", F, ["cx_bundle", "cx_fn"], "cx_verdict", "`#cx_contribution(#b,#g)`: the verdict bundle `#b` contributes to `#g` (declaration standing, checker, holds)"),
    ("cx_composed", F, ["cx_fn"], "cx_verdict", "`#cx_composed(#g)`: the composed verdict of `#g`"),
    ("cx_host", F, ["cx_fn"], "cx_verdict", "`#cx_host(#g)`: the host analysis's own verdict for `#g`"),
    ("cx_reasons_kept", P, ["cx_bundle", "cx_fn"], None, "`#cx_reasons_kept(#b,#g)`: the composed record keeps every reason `#b` gave for `#g`"),
    ("cx_plugin", P, ["cx_bundle"], None, "`#cx_plugin(#b)`: `#b` binds an out-of-process checker (`quirin-plugin.json`)"),
    ("cx_proposes_decision", P, ["cx_bundle", "cx_fn"], None, "`#cx_proposes_decision(#b,#g)`: `#b`'s checker proposed `Safe` or `Unsafe` for `#g`"),
    ("cx_checked", P, ["cx_bundle", "cx_fn"], None, "`#cx_checked(#b,#g)`: a trusted-core checker accepted the certificate or validated the witness (`certified-safe`, `validated-unsafe`)"),
    ("cx_checker_verdict", F, ["cx_bundle", "cx_fn"], "cx_verdict", "`#cx_checker_verdict(#b,#g)`: the verdict the firewall class of `#b`'s checker answer stands for (rule 10h)"),
    ("cx_recorded_verdict", F, ["cx_bundle", "cx_fn"], "cx_verdict", "`#cx_recorded_verdict(#b,#g)`: the verdict the report's checker record states"),
    # Part C
    ("cx_satisfied", P, ["cx_dep"], None, "`#cx_satisfied(#d)`: dependency `#d` resolves"),
    ("cx_loaded", P, ["cx_dep"], None, "`#cx_loaded(#d)`: `#d`'s target crate is on the load path and its target declaration is loaded"),
    ("cx_in_range", P, ["cx_dep"], None, "`#cx_in_range(#d)`: the target crate's version lies in `#d`'s range (or `#d` states none)"),
    ("cx_digest_matches", P, ["cx_dep"], None, "`#cx_digest_matches(#d)`: the target declaration's SHA-256 is the one recorded for `#d` (an unrecorded digest matches nothing)"),
    ("cx_declared_crate", P, ["cx_dep"], None, "`#cx_declared_crate(#d)`: the dependent's crate is on the load path, whose contract-level graph (not Cargo's) the target crate must be on (card CONNECT-ADOPT-DECLARED-DEPENDENCIES, D1 (a))"),
    ("cx_owner_named", P, ["cx_dep"], None, "`#cx_owner_named(#d)`: a libc/POSIX target is named under `quirin-provider-libc`"),
    ("cx_held", P, ["cx_bundle", "cx_fn"], None, "`#cx_held(#b,#g)`: `#b`'s declaration of `#g` is held (not in force)"),
    ("cx_target_bundle", F, ["cx_dep"], "cx_bundle", "`#cx_target_bundle(#d)`: the crate `#d` names"),
    ("cx_target", F, ["cx_dep"], "cx_fn", "`#cx_target(#d)`: the declaration `#d` names"),
    ("cx_dependent_bundle", F, ["cx_dep"], "cx_bundle", "`#cx_dependent_bundle(#d)`: the crate of the declaration that states `#d`"),
    ("cx_dependent", F, ["cx_dep"], "cx_fn", "`#cx_dependent(#d)`: the declaration that states `#d`"),
    ("cx_on_cycle", P, ["cx_bundle", "cx_fn"], None, "`#cx_on_cycle(#b,#g)`: `#b`'s declaration of `#g` reaches itself through declared dependencies"),
    ("cx_hold_verdict", F, ["cx_bundle", "cx_fn"], "cx_verdict", "`#cx_hold_verdict(#b,#g)`: the verdict a hold of `#b`'s `#g` contributes (`Safe` when not held)"),
    ("cx_human_resolvable", P, ["cx_bundle", "cx_fn"], None, "`#cx_human_resolvable(#b,#g)`: every cause of the hold is resolved by a person's review and re-pin"),
    # Card CONNECT-ADOPT-DECLARED-DEPENDENCIES: the reliance rule and linkage_contract_v1
    ("cx_reads", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_reads(#b,#g,#c,#h)`: a verdict that uses `#b`'s declaration of `#g` reads `#c`'s declaration of `#h` through it (an inherited clause, a body-checked specification's callee, a frame of `#h`'s calls)"),
    ("cx_decides", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_decides(#b,#g,#c,#h)`: that reading can discharge a `Safe` obligation or validate an `Unsafe` violation (not only yield `Unknown` or `Pending`)"),
    ("cx_relies", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_relies(#b,#g,#c,#h)`: `#b`'s `#g` relies on `#c`'s `#h`: a reading that can decide a verdict"),
    ("cx_co_used", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_co_used(#b,#g,#c,#h)`: one function row's verdict uses both declarations"),
    ("cx_link_required", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_link_required(#b,#g,#c,#h)`: the rule requires `#b`'s `#g` to link `#c`'s `#h`"),
    ("cx_check_passes", P, ["cx_bundle"], None, "`#cx_check_passes(#b)`: the linkage check (`providers/contracts.py check`) passes for bundle `#b`"),
    ("cx_links", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_links(#b,#g,#c,#h)`: `#b`'s declaration of `#g` states a `linkage_contract_v1` link to `#c`'s `#h`"),
    ("cx_inherits", P, ["cx_bundle", "cx_fn", "cx_bundle", "cx_fn"], None, "`#cx_inherits(#b,#g,#c,#h)`: `#b`'s declaration of `#g` is `contract inherit` of `#c`'s `#h`, itself the link (D2)"),
    ("cx_marked", P, ["cx_bundle", "cx_fn"], None, "`#cx_marked(#b,#g)`: `#b`'s declaration of `#g` states `requires linkage_contract_v1;`"),
    ("cx_load_refused", P, ["cx_bundle", "cx_fn"], None, "`#cx_load_refused(#b,#g)`: the loader refuses the module of `#b`'s `#g` (total-or-reject)"),
]


def rust_sorts():
    out = ["    // Card QUIRIN-AUDIT-CONNECT (the CX block): the sorts of the chain's,",
           "    // the composition's and the declared dependencies' conditions, prefixed",
           "    // `cx_`."]
    for name, meaning in SORTS:
        out += ["    (", f'        "{name}",', f'        "{meaning}",', "    ),"]
    return "\n".join(out)


def rust_terms():
    out = ["    // ---- Card QUIRIN-AUDIT-CONNECT (user decisions of 2026-11-26, every",
           "    // condition and refusal a formula): the terms of the chained",
           "    // measurement's links, completeness, rows and attribution (Part A), the",
           "    // bundle composition (Part B) and the declared contract dependencies",
           "    // (Part C), each named by a formula of `qcore_schemas::connect::FORMULAS`.",
           "    // Every name is prefixed `cx_`. The Lean model is `QCore.MeasureConnect`",
           "    // (`cxTerms`, one definition per term). ----"]
    for name, kind, arguments, result, meaning in TERMS:
        if kind == P:
            sort = "RenderedSort::Predicate(&[" + ", ".join(f'"{a}"' for a in arguments) + "])"
        elif kind == F:
            sort = ("RenderedSort::Function(&[" + ", ".join(f'"{a}"' for a in arguments)
                    + f'], "{result}")')
        else:
            sort = f'RenderedSort::Value("{arguments[0]}")'
        meaning = meaning.replace('"', '\\"')
        out += ["    RenderedTerm {", f'        name: "{name}",', f"        sort: {sort},",
                f'        meaning: "{meaning}",', '        block: "CX",', "    },"]
    return "\n".join(out)


def markdown():
    rows = ["| Term | Sort | Meaning |", "|---|---|---|"]
    for name, kind, arguments, result, meaning in TERMS:
        sort = {P: f"pred({', '.join(arguments)})", V: arguments[0],
                F: f"({', '.join(arguments)}) → {result}"}[kind]
        rows.append(f"| `#{name}` | `{sort}` | {meaning} |")
    return "\n".join(rows)


if __name__ == "__main__":
    print({"--rust-sorts": rust_sorts, "--rust-terms": rust_terms,
           "--markdown": markdown}[sys.argv[1]]())
