#!/usr/bin/env python3
"""compare_audits.py A/lean.log B/lean.log [OUT.json] -- LEAN-4-35-TOOLCHAIN.

Compares two check_lean_audits.py transcripts audit by audit. Each audit's
captured output is replayed before its `Passed AuditX (i/n) in Ts` line, so the
text between two pass lines belongs to the later audit. Lean 4.35 wraps a long
`#print axioms` list over several lines (continuations start with a space), so
continuation lines are joined before parsing. Reports:

* the audits run, in order (must be identical);
* per audit, the declarations whose axioms are printed, in order (the theorem
  set; must be identical);
* per declaration, the axiom set, and every declaration whose set differs;
* every other output line that differs (AuditAll's summary counts, messages).
"""
import json
import re
import sys
from collections import Counter

PASS = re.compile(r"^Passed (Audit\S+\.lean) \(\d+/\d+\) in [0-9.]+s$")
AXIOMS = re.compile(r"^'(?P<name>[^']+)' depends on axioms: \[(?P<axioms>[^\]]*)\]$")
NONE = re.compile(r"^'(?P<name>[^']+)' does not depend on any axioms$")
STARTING = re.compile(r"^Starting \d+ audits")
ALLOWED = {"propext", "Classical.choice", "Quot.sound"}


def joined(lines):
    out = []
    for line in lines:
        if line.startswith(" ") and out and not PASS.match(line):
            out[-1] += " " + line.strip()
        else:
            out.append(line)
    return out


def parse(path):
    lines = open(path, encoding="utf-8").read().splitlines()
    audits = []
    pending = []
    # The build transcript precedes the first audit; it is not audit output.
    start = next(i for i, line in enumerate(lines) if STARTING.match(line))
    for line in lines[start + 1:]:
        match = PASS.match(line)
        if match:
            decls, other = [], []
            for text in joined(pending):
                axioms = AXIOMS.match(text)
                none = NONE.match(text)
                if axioms:
                    names = [a.strip() for a in axioms.group("axioms").split(",") if a.strip()]
                    decls.append((axioms.group("name"), tuple(sorted(names))))
                elif none:
                    decls.append((none.group("name"), ()))
                else:
                    other.append(text)
            audits.append((match.group(1), decls, other))
            pending = []
        else:
            pending.append(line)
    return audits


def main():
    a, b = parse(sys.argv[1]), parse(sys.argv[2])
    result = {
        "audits": [len(a), len(b)],
        "audit_order_identical": [x[0] for x in a] == [x[0] for x in b],
    }
    decl_names_identical = True
    changed = []
    other_diff = []
    decl_counts = [0, 0]
    for (name_a, decls_a, other_a), (name_b, decls_b, other_b) in zip(a, b):
        decl_counts[0] += len(decls_a)
        decl_counts[1] += len(decls_b)
        if [d[0] for d in decls_a] != [d[0] for d in decls_b]:
            decl_names_identical = False
        for (decl, ax_a), (_, ax_b) in zip(decls_a, decls_b):
            if ax_a != ax_b:
                changed.append({"audit": name_a, "declaration": decl,
                                "before": list(ax_a), "after": list(ax_b)})
        if other_a != other_b:
            other_diff.append({"audit": name_a, "before": other_a, "after": other_b})
    every = [ax for audits in (a, b) for _, decls, _ in audits for _, axs in decls for ax in axs]
    result.update({
        "declarations_printed": decl_counts,
        "declaration_order_identical": decl_names_identical,
        "axioms_outside_allowed": sorted(set(every) - ALLOWED),
        "axiom_set_changes": len(changed),
        "axiom_set_change_shapes": Counter(
            f"{'+'.join(c['before']) or 'none'} -> {'+'.join(c['after']) or 'none'}" for c in changed
        ).most_common(),
        "axiom_set_change_audits": Counter(c["audit"] for c in changed).most_common(),
        "axiom_set_change_namespaces": Counter(
            c["declaration"].rsplit(".", 1)[0] for c in changed
        ).most_common(),
        "other_output_differences": other_diff,
        "changes": changed,
    })
    text = json.dumps(result, indent=1, ensure_ascii=False)
    if len(sys.argv) > 3:
        open(sys.argv[3], "w", encoding="utf-8").write(text + "\n")
    summary = {k: v for k, v in result.items() if k != "changes"}
    print(json.dumps(summary, indent=1, ensure_ascii=False)[:6000])


if __name__ == "__main__":
    main()
