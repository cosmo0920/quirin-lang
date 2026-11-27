#!/usr/bin/env python3
"""W48-CALLEE-CLOSURE-R3: the closure through unreported header helpers.

Usage: helper_closure.py DIAGNOSTICS PARITY_JSONL SEED[,SEED...] [--json OUT]

A `static` header definition a unit carries is inside W4's derivation scope
(gateway 4a) but is not a reported row; when its body refuses, its callers
see an uncontracted call. SEED names callees taken as newly contracted (for
example a libc function declared in quirin-provider-libc). From the parity
stream, every unreported function name whose recorded refusals (any
position, old and new route both refusing; an uncontracted call, or rule
10y's `nested call ... has no contract in force`) are all uncontracted calls to a
seed or to a helper already closed is closed too, to a fixpoint. The closed
helpers and the seeds are then served in the reported diagnostics, and the
Unknown functions whose every blocker is served are listed (one-round upper
bound, whatif.py's test). A helper that also refuses for a non-call reason is
never closed. This predicts where to look; the arbiter decides.
"""
import argparse
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402

UNC = re.compile(r"call to uncontracted function `([^`]+)`|nested call `([^`]+)` (?:has no contract in force|inside a modeled call is outside the resource projection)")


def refusals_by_helper(parity, reported):
    refs = collections.defaultdict(set)
    with open(parity) as handle:
        for line in handle:
            row = json.loads(line)
            name = row.get("function")
            if not name or name in reported:
                continue
            refs.setdefault(name, set())
            if row.get("old_admitted") or row.get("new_admitted"):
                continue
            reason = row.get("old_refusal") or row.get("new_refusal") or row.get("refusal")
            if not reason:
                continue
            match = UNC.fullmatch(reason)
            refs[name].add(("call", match.group(1) or match.group(2)) if match else ("leaf", reason))
    return refs


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics")
    parser.add_argument("parity")
    parser.add_argument("seeds")
    parser.add_argument("--json")
    args = parser.parse_args()
    _, functions = closure_r3.analyse(closure_r3.load(args.diagnostics))
    reported = {key[1] for key in functions}
    refs = refusals_by_helper(args.parity, reported)
    served = set(args.seeds.split(","))
    closed_rounds = []
    while True:
        added = sorted(name for name, reasons in refs.items()
                       if name not in served and reasons
                       and all(kind == "call" and callee in served for kind, callee in reasons))
        if not added:
            break
        closed_rounds.append(added)
        served |= set(added)
    safe = {key for key, value in functions.items() if value["status"] == "Safe"}
    freed, touched = [], collections.Counter()
    for key, entry in functions.items():
        if entry["status"] != "Unknown":
            continue
        hit = set(entry["calls"]) & served
        for name in hit:
            touched[name] += 1
        if not hit or entry["others"]:
            continue
        if all(name in served or (key[0], name) in safe for name in entry["calls"]):
            freed.append("::".join(key))
    print("closed helpers by round:")
    for number, names in enumerate(closed_rounds):
        print(f"  {number}: {', '.join(names)}")
    print(f"reported Unknown functions naming a served callee: {sum(1 for _ in touched)} callees, "
          f"{sum(touched.values())} function-callee pairs")
    print(f"freed (upper bound) {len(freed)}")
    for label in sorted(freed):
        print(f"  {label}")
    if args.json:
        pathlib.Path(args.json).write_text(json.dumps(
            {"seeds": sorted(args.seeds.split(",")), "closed_rounds": closed_rounds,
             "touched": dict(touched), "freed": sorted(freed)}, indent=1, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
