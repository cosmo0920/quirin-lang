#!/usr/bin/env python3
"""summary.py PREVIOUS.json FIXED.json OUT.json: the old-vs-fixed table of the
condvar/join census (card W47-CONDVAR-JOIN-PUBLICATION): every C4 site in the
bound files, every C5 release under the two pause tests' roots, and the
whole-program tallies. Each row is a census verdict at that path under the
census's derived premises, never a verified race or fix."""
import json
import sys

BOUND_FILES = {"tests/runtime/in_tail.c", "src/flb_input_thread.c", "src/flb_lib.c"}
ROOTS = ("flb_test_inotify_pause_collectors", "flb_test_inotify_threaded_pause_collectors")


def reason(result):
    return result.get("reason") or result.get("refusal") or result.get("reasons")


def read(path):
    data = json.load(open(path))
    sites = [
        {"at": f"{s['file']}:{s['line']}", "function": s["function"].split("::")[-1],
         "kind": s["kind"], "object": s["object"], "verdict": s["result"]["verdict"],
         "why": reason(s["result"])}
        for s in data["sites"] if s["file"] in BOUND_FILES
    ]
    releases = [
        {"root": r["root"].split("::")[-1], "at": f"{r['file']}:{r['line']}",
         "function": r["function"].split("::")[-1], "what": r["what"],
         "verdict": r["result"]["verdict"], "why": reason(r["result"]),
         "live": r["live"], "unresolved": r["unresolved"]}
        for r in data["releases"] if r["root"].split("::")[-1] in ROOTS
    ]
    others = [
        {"at": f"{s['file']}:{s['line']}", "function": s["function"].split("::")[-1],
         "kind": s["kind"], "object": s["object"], "verdict": s["result"]["verdict"],
         "why": reason(s["result"])}
        for s in data["sites"]
        if s["file"] not in BOUND_FILES and s["result"]["verdict"] != "unresolved"
    ]
    return {"files": data["files"], "tally": data["tally"], "signal_once": data["signal_once"],
            "bound_sites": sites, "pause_test_releases": releases, "other_decided_sites": others}


def main():
    previous, fixed, out = sys.argv[1:4]
    json.dump({"previous_90978ac6~1": read(previous), "fixed_90978ac6": read(fixed)},
              open(out, "w"), indent=1)
    print(out)


if __name__ == "__main__":
    main()
