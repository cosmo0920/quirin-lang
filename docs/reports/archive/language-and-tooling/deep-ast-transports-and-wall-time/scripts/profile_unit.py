#!/usr/bin/env python3
"""profile_unit.py SAMPLES.jsonl UNITS.jsonl SUFFIX [OUT.json] [--top N] --
DEEP-AST-TRANSPORTS-AND-WALL-TIME (adapted from GATEWAY-ANALYSIS-PATH-OPTIMIZATION's
scripts/profile.py): the wall-clock samples of one unit's own analysis.

A thread-sample belongs to the unit when its thread (kernel id) is the thread
the `unit-timing` interval of a file ending in SUFFIX names, and the sample's
Unix time lies inside that interval; a thread the unit's analysis starts
(the rule-13o reparse on its own stack, a Clang input writer) is counted when
its stack holds no project frame of another unit -- such threads are matched
by time only and reported apart (`helper_thread_samples`).

Each of the unit's thread-samples is classified by the frames it holds, in
this order:

* clang-wait: inside a Clang transport (`typed_c_ast::`, the preprocessing,
  `std::process`, `wait_with_output`, `read_to_end`);
* json-default-parse: serde_json under `serde_json::de::from_slice` /
  `from_trait` (the default-limit parse);
* depth-scan: `deep_unit_ast::depth`;
* json-unlimited-parse: serde_json under `deep_unit_ast::unlimited`;
* json-other: any other deserialisation frame near the top (a reader's
  `Value` indexing is `serde_json::value::`, counted as a reader);
* index-and-readers: the rest, reported by the innermost project frame and
  inclusively.

Inclusive and self shares as profile.py reports them.
"""
import collections
import json
import pathlib
import re
import sys

PROJECT = re.compile(r"^(<)?(qcore_|quirin_)")
GENERIC = re.compile(r"<[^<>]*>")
CLANG = re.compile(r"(typed_c_ast::launch|typed_c_ast::translation_unit_ast_with|typed_c_ast::record_layout|legacy_preparation::|legacy_enumeration::preprocess|std::process::|wait_with_output|read_to_end|clang::)")


def short(name):
    previous = None
    while previous != name:
        previous, name = name, GENERIC.sub("", name)
    return re.sub(r"::h[0-9a-f]{16}$", "", name)


def classify(stack):
    joined = " ".join(stack)
    if "deep_unit_ast::depth" in joined:
        return "depth-scan"
    if "deep_unit_ast::unlimited" in joined and re.search(r"serde", joined):
        return "json-unlimited-parse"
    top = " ".join(stack[:14])
    if re.search(r"serde_json::de::|serde::de::", top):
        if "from_slice" in joined or "from_trait" in joined:
            return "json-default-parse"
        return "json-other"
    if CLANG.search(joined):
        return "clang-wait"
    return "index-and-readers"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 40
    if "--top" in sys.argv:
        args.remove(str(top))
    samples_path, units_path, suffix = args[:3]
    out = args[3] if len(args) > 3 else None
    intervals = [json.loads(line) for line in open(units_path)]
    mine = [r for r in intervals if r["file"].endswith(suffix)]
    others = [r for r in intervals if not r["file"].endswith(suffix)]
    classes, inclusive, selfs = collections.Counter(), collections.Counter(), collections.Counter()
    per_interval = collections.Counter()
    helpers = 0
    count = 0
    for line in open(samples_path):
        sample = json.loads(line)
        now = sample["unix"]
        live = [r for r in mine if r["start"] <= now <= r["end"]]
        if not live:
            continue
        threads = {r["thread"] for r in live}
        busy_other = {r["thread"] for r in others if r["start"] <= now <= r["end"]}
        for thread in sample["threads"]:
            lwp = str(thread["lwp"])
            stack = thread["stack"]
            if lwp in threads:
                pass
            elif lwp in busy_other:
                continue
            else:
                # A helper thread: count it only when it holds a deep-AST
                # reparse frame (the one helper the unit's own analysis starts
                # that does work).
                if not any("deep_unit_ast::unlimited" in f for f in stack):
                    continue
                helpers += 1
            count += 1
            per_interval[next((r["start"] for r in live if r["thread"] == lwp), "helper")] += 1
            classes[classify(stack)] += 1
            project = [short(f) for f in stack if PROJECT.search(f)]
            for name in set(project):
                inclusive[name] += 1
            if project:
                selfs[project[0]] += 1
            else:
                selfs[short(stack[0]) if stack else "?"] += 1
    result = {
        "suffix": suffix,
        "intervals": [[round(r["end"] - r["start"], 1), r["thread"]] for r in mine],
        "thread_samples": count,
        "helper_thread_samples": helpers,
        "classes": {k: [v, round(100 * v / max(count, 1), 1)] for k, v in classes.most_common()},
        "inclusive_top": [[n, c, round(100 * c / max(count, 1), 1)] for n, c in inclusive.most_common(top)],
        "self_top": [[n, c, round(100 * c / max(count, 1), 1)] for n, c in selfs.most_common(top)],
    }
    text = json.dumps(result, indent=1)
    if out:
        pathlib.Path(out).write_text(text + "\n")
    print(text[:20000])


if __name__ == "__main__":
    main()
