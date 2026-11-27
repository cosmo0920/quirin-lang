#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: build the polarity/arm census instrumentation.

Writes an instrumented copy of `legacy_c.rs` that, after the custody fixed
point has settled, emits one JSON line per `BranchOnNull` **edge** in the
analysed function, recording

  * the edge's structural role  — `exit` (the loop header's leaving edge),
    `body` (the loop header's other edge) or `branch` (an `if`);
  * the header's polarity        — `true_is_null`, so the two inverted
    spellings `while (!flag)` / `while (flag == 0)` are `true`;
  * the arm `refine_pointer_null` takes on the **settled** state — one of
    `untracked`, `unknown_or_multi`, `empty_targets`, `known_non_null`,
    `nullable_single`; and
  * the refinement result for that edge's own null assumption.

It is read after the fixed point for the same reason
`require_live_borrowed_actuals` is: the arm is reached on intermediate iterates
of a header whose fixed point later widens the deciding binding, so a census
taken inside the work list would depend on the work-list schedule (constraint
6). The census changes **no verdict**: it only reads, and the published
BEFORE/AFTER artifacts are produced by binaries built without it.

Usage: python3 instrument_census.py <pristine legacy_c.rs> <out.rs>
"""
import pathlib
import sys

ANCHOR = "    require_live_borrowed_actuals(&cfg, &incoming, &mut diagnostics);"

CALL = ANCHOR + "\n    scalar_route_census(&cfg, &incoming);"

CENSUS = r'''
// ---------------------------------------------------------------------------
// TEMPORARY measurement instrumentation — card SCALAR-ROUTE-EXIT-EDGE.
// Reads only. Gated on QCORE_SRE_CENSUS, which names the output file.
// Removed before the card publishes; the archived artifacts are produced by
// binaries built without it.
// ---------------------------------------------------------------------------
fn scalar_route_census(cfg: &LegacyCfg, incoming: &[Option<AbstractState>]) {
    use std::io::Write as _;
    let Ok(path) = std::env::var("QCORE_SRE_CENSUS") else {
        return;
    };
    let mut lines = String::new();
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let LegacyInstruction::BranchOnNull {
            source,
            true_is_null,
        } = &node.instruction
        else {
            continue;
        };
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let arm = match state.pointers.get(source) {
            None => "untracked",
            Some(pointer) if pointer.unknown || pointer.targets.len() > 1 => "unknown_or_multi",
            Some(pointer) if pointer.targets.is_empty() => "empty_targets",
            Some(pointer) => {
                let site = *pointer.targets.iter().next().expect("one target");
                if !pointer.may_be_null && !state.nullable_sites.contains(&site) {
                    "known_non_null"
                } else {
                    "nullable_single"
                }
            }
        };
        for (edge, _) in node.successors.iter().enumerate() {
            let role = match node.loop_exit_edge {
                Some(exit) if exit == edge => "exit",
                Some(_) => "body",
                None => "branch",
            };
            let assume_null = if edge == 0 { *true_is_null } else { !*true_is_null };
            let mut probe = state.clone();
            let result = match refine_pointer_null(&mut probe, source, assume_null) {
                PointerRefinement::Refined => "refined",
                PointerRefinement::Infeasible => "infeasible",
                PointerRefinement::Unknown => "unknown",
            };
            lines.push_str(&format!(
                "{{\"function\":{:?},\"line\":{},\"node\":{},\"edge\":{},\"role\":{:?},\
\"true_is_null\":{},\"assume_null\":{},\"arm\":{:?},\"result\":{:?},\
\"loop_header\":{}}}\n",
                cfg.function,
                node.line,
                node_id,
                edge,
                role,
                true_is_null,
                assume_null,
                arm,
                result,
                node.loop_exit_edge.is_some(),
            ));
        }
    }
    if lines.is_empty() {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(lines.as_bytes());
    }
}
'''


def main():
    if len(sys.argv) != 3:
        print("usage: instrument_census.py <pristine.rs> <out.rs>", file=sys.stderr)
        return 2
    source = pathlib.Path(sys.argv[1]).read_text()
    if source.count(ANCHOR) != 1:
        print(f"anchor not unique ({source.count(ANCHOR)})", file=sys.stderr)
        return 1
    patched = source.replace(ANCHOR, CALL) + CENSUS
    pathlib.Path(sys.argv[2]).write_text(patched)
    print(f"instrumented -> {sys.argv[2]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
