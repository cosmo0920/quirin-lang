#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card PRIORITY0-ATTRIBUTION: classify the evidence changes of every row of
the priority0 chain record by cause (read-only; no re-measurement).

    python3 evidence_classes.py CHAIN.json LANDINGS.json OUT.json

CHAIN.json is `quirin-audit measure chain`'s record over the measurement's
side directories (step 1 prev->common, step 2 common->tip). LANDINGS.json
lists the committed pinned landing records with the first-parent interval
they fall in. Every evidence change of a row in a step is classified as:

* `record:<card>`   the same row changes in a committed pinned landing record
                    (same inputs) whose comparison lies inside the step;
* `text:<commit>`   an added leaf whose text was introduced inside the step's
                    range by the named commit (`git log -S` on crates/);
* `decl:<commit>`   an added declaration whose `fn NAME(` was introduced
                    inside the step's range by the named commit (providers/);
* `atom:<commit>`   an added hypothesis atom introduced inside the range;
* `cascade`         a summary dependency added or withdrawn (follows the
                    callee's own row);
* `note:<card>`     a leaf tag or field the named card's own report gives as
                    its effect (NOTE_MAP);
* otherwise the class stays unattributed and the row is listed.
"""

import collections
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
RANGES = {1: ("556460e31", "2c07390b6"), 2: ("2c07390b6", "9aed1913a")}


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout


CACHE = {}


def present(fragment, revision, paths):
    return subprocess.run(["git", "grep", "-q", "-F", fragment, revision, "--", *paths],
                          cwd=ROOT).returncode == 0


def introduced(fragment, step, paths):
    """The commit that introduced `fragment` inside the step's range: absent
    from `paths` at the range's start, present at its end; the oldest commit
    of the range whose diff changes its number of occurrences, as `hash
    subject-prefix`. A fragment the range did not introduce attributes
    nothing."""
    key = (fragment, step, tuple(paths))
    if key not in CACHE:
        lo, hi = RANGES[step]
        CACHE[key] = None
        if not present(fragment, lo, paths) and present(fragment, hi, paths):
            out = git("log", "--format=%h %s", "-S", fragment, f"{lo}..{hi}", "--", *paths).splitlines()
            if out:
                h, subject = out[-1].split(" ", 1)
                CACHE[key] = f"{h} {subject.split(':')[0][:60]} [landed in {landing_of(h)}]"
    return CACHE[key]


FIRST_PARENT = []


def landing_of(commit):
    """The subject of the first first-parent commit of main that contains
    `commit` (the merge that landed it)."""
    if not FIRST_PARENT:
        FIRST_PARENT.extend(git("log", "--first-parent", "--reverse", "--format=%H %s",
                                "556460e31^..9aed1913a").splitlines())
    lo, hi = 0, len(FIRST_PARENT) - 1
    while lo < hi:
        mid = (lo + hi) // 2
        if subprocess.run(["git", "merge-base", "--is-ancestor", commit, FIRST_PARENT[mid].split()[0]],
                          cwd=ROOT).returncode == 0:
            hi = mid
        else:
            lo = mid + 1
    h, subject = FIRST_PARENT[lo].split(" ", 1)
    return f"{h[:9]} {subject[:70]}"


def fragments(description):
    """Candidate source fragments of a leaf text: its trailing parenthesised
    tag (`(callback-registry-undecided)`), then its backtick-free runs,
    longest first."""
    out = []
    tag = re.search(r"\(([a-z0-9]+(?:-[a-z0-9]+)+)\)\s*$", description)
    if tag:
        out.append(tag.group(1))
    parts = [p.strip(" ,.;:()") for p in re.split(r"`[^`]*`|\(\d+ unit", description)]
    out += sorted((p for p in parts if len(p) >= 12), key=len, reverse=True)
    return out


# A leaf tag or field whose change a landed card's own report names as its
# effect, per step (`note:` causes; weaker than a record, stated as such).
NOTE_MAP = {
    2: [
        ("callback-registry-undecided", "CG10-EMBEDDED-LIST-BORROWED-CONTAINER (r45 note: access leaves become callback-registry-undecided)"),
        ("retention-undecided", "CG10-EMBEDDED-LIST-BORROWED-CONTAINER (r45 note: access leaves become retention-undecided)"),
        ("interior-borrow-ended", "NULLABLE-GUARDED-RELEASE (rule 74c member-address frame)"),
        ("reference-dropped", "NULLABLE-GUARDED-RELEASE (rule 74c member-address frame)"),
        ("reference-unbalanced", "NULLABLE-GUARDED-RELEASE (rule 74c member-address frame)"),
        ("relational-untested", "CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT (rule 72)"),
        ("without an out-parameter contract", "OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY"),
        ("member-store-through-formal", "CG10-MEMBER-STORE-RETAINED-BY / MEMBER-EFFECT-STORES (rules 70, 38k)"),
        ("field:rule_obligations", "MEMBER-EFFECT-STORES (rule obligations of rule 38k)"),
        ("field:reading_site_evidence", "K2-NO-TYPED-ACCESS / K6-DIRECT-POSITION-ACCESS (reading-site declaration-reader digest re-pinned for rules 79/81)"),
    ],
    1: [],
}


def note_cause(step, kind, cat, text):
    probe = f"field:{cat}" if kind == "field" else text
    for tag, card in NOTE_MAP.get(step, []):
        if tag in probe:
            return "note:" + card
    return None


def main():
    chain = json.loads(Path(sys.argv[1]).read_text())
    landings = json.loads(Path(sys.argv[2]).read_text())
    out_path = Path(sys.argv[3])
    record_rows = collections.defaultdict(set)  # (step, route, row) -> cards
    for rec in landings:
        if rec["step"] is None:
            continue
        for route, rows in rec["rows"].items():
            for row in rows:
                record_rows[(rec["step"], route, row)].add(rec["card"])
    result = {"schema": "priority0-evidence-classes/1", "routes": {}}
    for route, r in chain["segments"][0]["routes"].items():
        per_step = {1: collections.Counter(), 2: collections.Counter()}
        cause_counts = {1: collections.Counter(), 2: collections.Counter()}
        unattributed = []
        rows_total = {1: 0, 2: 0}
        for x in r["rows"]:
            if x["kind"] != "function" or x["pattern"] != "evidence-only":
                continue
            for e in x.get("evidence", []):
                step, ev = e["step"], e["evidence"]
                rows_total[step] += 1
                classes = []
                for leaf in ev.get("leaves_added", []):
                    classes.append(("leaf+", leaf["category"], leaf["description"]))
                for leaf in ev.get("leaves_removed", []):
                    classes.append(("leaf-", leaf["category"], leaf["description"]))
                for field, change in (ev.get("contract_dependencies") or {}).items():
                    for k in ("added", "removed"):
                        for name in change.get(k, []):
                            classes.append(("dep-" + k, field, name))
                for field in ev.get("other_fields_changed", []):
                    classes.append(("field", field if isinstance(field, str) else json.dumps(field), ""))
                causes = set()
                cards = record_rows.get((step, route, x["row"]))
                if cards:
                    causes |= {"record:" + c for c in sorted(cards)}
                unmapped = []
                for kind, cat, text in classes:
                    per_step[step][f"{kind} {cat}"] += 1
                    cause = None
                    if kind == "leaf+":
                        for frag in fragments(text):
                            c = introduced(frag, step, [":(glob)crates/*/src/**"])
                            if c:
                                cause = "text:" + c
                                break
                    elif kind == "dep-added" and cat == "declarations":
                        name = json.loads(text) if text.startswith('"') else text
                        c = introduced(f"fn {name}(", step, [":(glob)providers/*/contracts/**", "contract"])
                        cause = c and "decl:" + c
                    elif kind == "dep-added" and cat == "hypothesis_dependencies":
                        try:
                            atoms = json.loads(text.split("=", 1)[1])
                        except (IndexError, ValueError):
                            atoms = []
                        for atom in atoms:
                            if atom.startswith("legacy_atom"):
                                continue
                            c = introduced(atom, step, [":(glob)providers/*/contracts/**", "contract"])
                            if c:
                                cause = "atom:" + c
                                break
                    elif kind.startswith("dep-") and cat in ("summaries", "withdrawn"):
                        cause = "cascade"
                    cause = cause or note_cause(step, kind, cat, text)
                    if cause:
                        causes.add(cause)
                    else:
                        unmapped.append(f"{kind} {cat}")
                for c in causes:
                    cause_counts[step][c] += 1
                if not causes:
                    cause_counts[step]["unattributed"] += 1
                    unattributed.append({"row": x["row"], "step": step, "classes": sorted(set(unmapped))})
        result["routes"][route] = {
            "rows_with_evidence_change": rows_total,
            "classes": {s: dict(per_step[s].most_common()) for s in per_step},
            "causes": {s: dict(cause_counts[s].most_common()) for s in cause_counts},
            "unattributed": unattributed,
        }
        print(route, "rows", rows_total, "unattributed",
              collections.Counter(u["step"] for u in unattributed))
    out_path.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
