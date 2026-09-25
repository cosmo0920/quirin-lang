#!/usr/bin/env python3
"""Attribute every leaf that differs between BEFORE and AFTER to this card's rules.

Usage: attribute.py BEFORE.json AFTER.json CAPTURE_DIR OUT.json

Leaves are compared per (unit, function) as multisets of
(line, category, description). Each changed leaf is read against its analysed
(`-E -P`) line from a private capture of the base run (`capture_run.sh` of
W44-CORPUS-CALLEE-SUMMARIES) and assigned to the first class that explains it:

removed
* `decided-statement` -- `indirect memory access` on a statement whose first
                        call is a same-unit function that moved to `Safe`: its
                        summary now validates (for a recursive component, the
                        neutral route's), and the existing W44 argument route
                        admits the actual at its `Borrowed` position;
* `member-release`   -- `indirect memory access` on a statement whose call
                        hands a pointer member (`B->F`, `B[I]`, `*B`) to a
                        callee (rule 1);
* `member-null-test` -- a branch/loop-condition refusal whose controlling
                        expression is a null test of one pointer member
                        (rule 2);
* `decided-callee`   -- `call to uncontracted function F` where F is a
                        function of the same unit that is `Safe` in AFTER and
                        was not in BEFORE (its summary now validates);
* `member-null-test-desugared` -- a branch-condition refusal the W44
                        condition desugaring (gateway constraint 5h) now
                        lowers because one operand (inside `__builtin_expect`,
                        `&&`, `||`) is such a member null test;
* `poison-lifted`    -- `unresolved resource identity X` in a function that
                        also lost a rule-1 or rule-2 leaf: the refusal that
                        poisoned X is gone.
added
* `exchange`         -- `unresolved resource identity B` on a line that lost a
                        rule-1/rule-2 refusal: the base the typed read now
                        obliges does not resolve there;
* `decided-same-line` -- `unresolved resource identity X` on a line whose
                        `call to uncontracted function F` was removed as
                        `decided-callee`: the now-modeled call checks the
                        actual it releases (W44-CORPUS-CALLEE-SUMMARIES'
                        `same-line` class);
* `unmasked`         -- `unresolved resource identity X` elsewhere in a
                        function that lost a rule-1/rule-2 refusal, on a line
                        that names X: the lifted refusal had poisoned every
                        binding, so the identity refusal at a later use of X was
                        not reported separately before.
Anything else is `unattributed`, and the script exits 1.
"""
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "w44-corpus-callee-summaries" / "scripts"))
import definition_sites  # noqa: E402

CALL = re.compile(r"call to uncontracted function `([^`]+)`")
UNRESOLVED = re.compile(r"unresolved resource identity `([^`]+)`")
MEMBER_ARG = re.compile(r"[A-Za-z_]\w*\s*\(.*(\b[A-Za-z_]\w*\s*->\s*[A-Za-z_]\w*|\b[A-Za-z_]\w*\s*\[[^\]]*\]|\*\s*[A-Za-z_]\w*)")
NULL = r"(\(\s*\(\s*void\s*\*\s*\)\s*0\s*\)|NULL|0)"
MEMBER = r"\(*\s*[A-Za-z_]\w*\s*->\s*[A-Za-z_]\w*\s*\)*"
NULL_TEST = re.compile(
    r"\b(if|while)\s*\(\s*\(*\s*!?\s*\(*\s*(" + MEMBER + r"(\s*(==|!=)\s*" + NULL + r")?|" + NULL + r"\s*(==|!=)\s*" + MEMBER + r")\s*\)*\s*\)"
)
BARE_MEMBER = r"[A-Za-z_]\w*\s*->\s*[A-Za-z_]\w*"
OPERAND = re.compile(
    r"(__builtin_expect\s*\(|&&|\|\|)"
    r"|!\s*\(*\s*" + BARE_MEMBER + r"\s*\)*\s*(\)|&&|\|\|)"
)
OPERAND_TEST = re.compile(r"!\s*\(*\s*" + BARE_MEMBER + r"|" + BARE_MEMBER + r"\s*(==|!=)\s*" + NULL
                          + r"|(&&|\|\|)\s*\(*\s*" + BARE_MEMBER + r"\s*\)*\s*(\)|&&|\|\|)")
CONDITION = ("branch condition", "loop condition", "for-loop condition", "do-while condition")


def leaves(entry):
    return collections.Counter((d["line"], d["category"], d["description"]) for d in entry.get("diagnostics", []))


def main():
    before_path, after_path, capture, out = sys.argv[1:5]
    before, after = json.load(open(before_path)), json.load(open(after_path))
    texts = []
    for path in sorted(pathlib.Path(capture, "pp").glob("*-p1.i")):
        text = path.read_text(errors="replace")
        texts.append((set(definition_sites.definitions(text)), text))
    after_units = {u["library"] + "/" + u["file"]: u for u in after["units"]}
    changes = []
    for unit in before["units"]:
        key = unit["library"] + "/" + unit["file"]
        other = after_units[key]
        names = {f["function"] for f in unit["functions"]}
        match = sorted((t for t in texts if names <= t[0]), key=lambda t: len(t[0]))
        lines = match[0][1].split("\n") if match else []
        status_before = {f["function"]: f["status"] for f in unit["functions"]}
        status_after = {f["function"]: f["status"] for f in other["functions"]}
        after_fns = {f["function"]: f for f in other["functions"]}
        for entry in unit["functions"]:
            name = entry["function"]
            lb, la = leaves(entry), leaves(after_fns.get(name, {}))
            removed = [leaf for leaf, n in (lb - la).items() for _ in range(n)]
            added = [leaf for leaf, n in (la - lb).items() for _ in range(n)]
            if not removed and not added:
                continue
            text = lambda line: lines[line - 1].strip() if 0 < line <= len(lines) else ""
            rule_lines = set()
            rows = []
            for line, category, description in removed:
                t = text(line)
                cls = "unattributed"
                first_call = re.search(r"\b([A-Za-z_]\w*)\s*\(", t)
                callee = first_call.group(1) if first_call else None
                if (description.startswith("indirect memory access") and callee
                        and status_after.get(callee) == "Safe" and status_before.get(callee) != "Safe"):
                    cls = "decided-statement"
                elif description.startswith("indirect memory access") and MEMBER_ARG.search(t):
                    cls = "member-release"
                elif any(description.endswith(c) for c in CONDITION) and NULL_TEST.search(t):
                    cls = "member-null-test"
                elif (any(description.endswith(c) for c in CONDITION)
                      and OPERAND.search(t) and OPERAND_TEST.search(t)):
                    cls = "member-null-test-desugared"
                if cls != "unattributed":
                    rule_lines.add(line)
                rows.append(["removed", line, category, description, cls, t])
            for row in rows:
                if row[4] != "unattributed":
                    continue
                line, description = row[1], row[3]
                call = CALL.fullmatch(description)
                if call and status_after.get(call.group(1)) == "Safe" and status_before.get(call.group(1)) != "Safe":
                    row[4] = "decided-callee"
                elif UNRESOLVED.fullmatch(description) and rule_lines:
                    row[4] = "poison-lifted"
            decided_lines = {row[1] for row in rows if row[4] == "decided-callee"}
            for line, category, description in added:
                t = text(line)
                cls = "unattributed"
                unresolved = UNRESOLVED.fullmatch(description)
                if unresolved and line in rule_lines:
                    cls = "exchange"
                elif unresolved and line in decided_lines:
                    cls = "decided-same-line"
                elif unresolved and rule_lines and re.search(r"\b" + re.escape(unresolved.group(1)) + r"\b", t):
                    cls = "unmasked"
                rows.append(["added", line, category, description, cls, t])
            for row in rows:
                changes.append({
                    "unit": key, "function": name, "change": row[0], "line": row[1], "category": row[2],
                    "description": row[3], "class": row[4], "text": row[5][:200],
                    "status_before": status_before.get(name), "status_after": status_after.get(name),
                })
    summary = collections.Counter((c["change"], c["class"]) for c in changes)
    result = {"summary": {f"{a}:{b}": n for (a, b), n in sorted(summary.items())}, "changes": changes}
    json.dump(result, open(out, "w"), indent=1)
    print(json.dumps(result["summary"], indent=1))
    bad = [c for c in changes if c["class"] == "unattributed"]
    for c in bad[:40]:
        print("UNATTRIBUTED", c["function"], c["change"], c["line"], c["description"][:60], "|", c["text"][:120])
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
