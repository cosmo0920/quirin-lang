#!/usr/bin/env python3
"""census.py RUN PARITY_JSONL [--json OUT] -- card CG10-BOUND-UNTRACKED-ROOTS:
the population of chain roots the never-bound premise does not cover, and
what the per-path bound-untracked premise (rule 11d) would admit.

Reads the run's resource-diagnostics.json and the parity rows the card's
census records:

* `bound-untracked-root`: one row per settled node and chain root with
  `root_never_bound` false: eligibility, binding class, rooted place, path
  refused, the premise's answer, and whether the chain resolves without and
  with the premise;
* `bound-untracked-closure`: per function, why the lexical closure binds each
  name (class a: a non-Borrowed actual of a contracted callee, with its mode;
  b: an uncontracted callee or a position beyond the declaration; c: an
  assignment whose value holds a call, with the result mode; d: an address;
  e: anything else). A root the lexical closure does not bind at all is bound
  by the Clang AST closure only (rule 10l/10o join): class `ast`.

A `root refusal` is an `unknown_alias` leaf naming R at a (function, line)
whose census row for R does not resolve without the premise. Rows are keyed by
(function, line, root); a key counts as admitted only when every row for it
(W4 re-derivations included) resolves with the premise.

Functions freed alone: non-Safe functions all of whose leaves are root
refusals. `upper`: every such root's premise is `holds` or `path-refused`
(the path refusal may be another root refusal the premise lifts); `strict`:
every one is `holds` in the BEFORE state. Each freed function is attributed to
the class of its roots (the lowest-letter class when a root has several
reasons, `mixed` when roots differ).
"""
import argparse
import collections
import gzip
import json
import pathlib
import re

ALIAS = re.compile(r"unresolved resource identity `([^`]+)`")


def load_run(run):
    path = pathlib.Path(run)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    raw = path.read_bytes()
    if path.suffix == ".gz":
        raw = gzip.decompress(raw)
    data = json.loads(raw)
    functions = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            functions[(unit["file"], function["function"])] = function
    return functions


def classes_of(reasons):
    if not reasons:
        return ["ast"]
    return sorted({reason.split(":", 1)[0] for reason in reasons})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run")
    parser.add_argument("parity")
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    functions = load_run(args.run)
    rows = collections.defaultdict(list)
    closure = collections.defaultdict(lambda: collections.defaultdict(set))
    stores = collections.defaultdict(list)
    with open(args.parity) as stream:
        for line in stream:
            row = json.loads(line)
            position = row.get("position")
            if position == "bound-untracked-root":
                key = (row["function"], row["line"], row["root"])
                rows[key].append(row)
                if row["store"]:
                    stores[key].append(row)
            elif position == "bound-untracked-closure":
                for name, reasons in row["reasons"].items():
                    closure[row["function"]][name].update(reasons)

    def refused_without(key):
        return any(not row["resolves_without"] for row in rows[key])

    def admitted_with(key):
        return all(row["resolves_with"] for row in rows[key])

    def premises(key):
        return {row["premise"] for row in rows[key]}

    population = collections.Counter()
    population_functions = set()
    by_root = collections.Counter()
    by_premise = collections.Counter()
    by_binding = collections.Counter()
    by_class_leaves = collections.Counter()
    by_class_functions = collections.defaultdict(set)
    by_premise_binding = collections.Counter()
    only_roots = []
    freed = {"upper": collections.Counter(), "strict": collections.Counter()}
    freed_rows = []
    admitted_leaves = 0
    admitted_functions = set()
    for (unit, name), function in sorted(functions.items()):
        if function["status"] == "Safe":
            continue
        leaves = function.get("diagnostics", [])
        root_leaves = []
        other = 0
        for leaf in leaves:
            match = ALIAS.fullmatch(leaf.get("description") or "")
            key = match and (name, leaf["line"], match.group(1))
            if match and rows.get(key) and refused_without(key):
                root_leaves.append((leaf, key))
            else:
                other += 1
        for leaf, key in root_leaves:
            root = key[2]
            population["leaves"] += 1
            population_functions.add((unit, name))
            by_root[root] += 1
            for premise in premises(key):
                by_premise[premise] += 1
            for row in rows[key][:1]:
                by_binding[row["binding"]] += 1
            for cls in classes_of(closure[name].get(root)):
                by_class_leaves[cls] += 1
                by_class_functions[cls].add((unit, name))
            first = rows[key][0]
            by_premise_binding[(first["premise"], first["binding"])] += 1
            if admitted_with(key):
                admitted_leaves += 1
                admitted_functions.add((unit, name))
        if not root_leaves or other or function["status"] != "Unknown":
            continue
        only_roots.append({
            "unit": unit,
            "function": name,
            "roots": sorted({key[2] for _, key in root_leaves}),
            "premises": sorted(set().union(*(premises(key) for _, key in root_leaves))),
            "reasons": {key[2]: sorted(closure[name].get(key[2], [])) for _, key in root_leaves},
        })
        answers = set().union(*(premises(key) for _, key in root_leaves))
        classes = set()
        for _, key in root_leaves:
            reasons = closure[name].get(key[2])
            classes.add(classes_of(reasons)[0])
        attributed = classes.pop() if len(classes) == 1 else "mixed"
        upper = answers <= {"holds", "path-refused"}
        strict = answers == {"holds"}
        if upper:
            freed["upper"][attributed] += 1
        if strict:
            freed["strict"][attributed] += 1
        if upper:
            freed_rows.append({
                "unit": unit,
                "function": name,
                "roots": sorted({key[2] for _, key in root_leaves}),
                "class": attributed,
                "premises": sorted(answers),
                "strict": strict,
                "reasons": {key[2]: sorted(closure[name].get(key[2], [])) for _, key in root_leaves},
            })
    store_keys = [key for key in stores if refused_without(key)]
    record = {
        "population": {
            "root_refusal_leaves": population["leaves"],
            "functions": len(population_functions),
            "top_roots": by_root.most_common(12),
            "by_premise": dict(by_premise),
            "by_binding": dict(by_binding),
            "by_class_leaves": dict(by_class_leaves),
            "by_class_functions": {cls: len(names) for cls, names in by_class_functions.items()},
            "by_premise_and_binding_first_row": {
                f"{premise}/{binding}": count
                for (premise, binding), count in by_premise_binding.most_common()
            },
        },
        "functions_whose_only_leaves_are_root_refusals": only_roots,
        "admitted_with_premise_in_before_state": {
            "leaves": admitted_leaves,
            "functions": len(admitted_functions),
        },
        "freed_alone": {level: dict(counts) for level, counts in freed.items()},
        "freed_alone_totals": {level: sum(counts.values()) for level, counts in freed.items()},
        "pointer_store_owner_roots": {
            "refused_without": len(store_keys),
            "admitted_with": sum(1 for key in store_keys if admitted_with(key)),
        },
    }
    print(json.dumps(record, indent=1))
    if args.json:
        record["freed_functions"] = freed_rows
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
