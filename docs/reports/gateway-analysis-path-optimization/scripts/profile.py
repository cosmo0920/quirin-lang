#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: summarise gdb_sampler.py samples.

Usage: profile.py SAMPLES.jsonl [OUT.json] [--top N]

A *busy* thread-sample is a thread whose stack holds a frame of this
project's crates (`qcore_`, `quirin_`); idle runtime threads and the
coordinating thread waiting for its workers (`parallel::run_ordered`) are not
counted. Each busy thread-sample is classified by the innermost project frame
family:

* contract-environment: inside `qcore_contract` / `qcore_frontend` (the
  contract loader's readers of the environment's declared module texts, which
  lex them); the nearest gateway frame is reported as the asker;
* clang-wait: the thread is inside a Clang transport (a frame of
  `typed_c_ast`, `legacy_preparation`, `legacy_enumeration`'s preprocessing,
  `clang::`, or `std::process`) -- waiting on, writing to or reading a Clang
  child process;
* json-parse: inside serde_json deserialisation (the Clang AST parse);
* analysis: everything else.

Inclusive shares: the fraction of busy thread-samples whose stack contains a
frame of the function (each function counted once per stack). Self shares:
the innermost project frame. Shares of busy thread-samples approximate shares
of worker wall time (the sampler stops every thread at once).
"""
import collections
import json
import re
import sys

PROJECT = re.compile(r"^(<)?(qcore_|quirin_)")
CLANG = re.compile(r"(typed_c_ast::|legacy_preparation::|legacy_enumeration::preprocess|clang::|std::process::|wait_with_output|read_to_end)")
JSON = re.compile(r"serde_json::|serde::de::")
GENERIC = re.compile(r"<[^<>]*>")


def short(name):
    # strip generic arguments and the hash suffix, keep the path
    previous = None
    while previous != name:
        previous, name = name, GENERIC.sub("", name)
    return re.sub(r"::h[0-9a-f]{16}$", "", name)


COORDINATOR = ("qcore_audit::parallel::run_ordered", "quirin_audit::main")
CONTRACT = re.compile(r"^qcore_contract::|^qcore_frontend::")
GATEWAY = re.compile(r"^qcore_c_gateway::")


def main():
    top = 40
    if "--top" in sys.argv:
        top = int(sys.argv[sys.argv.index("--top") + 1])
    inclusive, selfs, classes = collections.Counter(), collections.Counter(), collections.Counter()
    callers = collections.Counter()
    busy = samples = coordinator = 0
    for line in open(sys.argv[1]):
        sample = json.loads(line)
        samples += 1
        for stack in sample["threads"]:
            project = [short(frame) for frame in stack if PROJECT.search(frame)]
            if not project:
                continue
            # the coordinating thread waiting for its workers is not work
            if project[0].startswith(COORDINATOR):
                coordinator += 1
                continue
            busy += 1
            # the contract-environment readers (qcore_contract / qcore_frontend)
            # and the gateway frame that asked them
            if CONTRACT.search(project[0]):
                classes["contract-environment"] += 1
                asking = next((f for f in project if GATEWAY.search(f)), "?")
                callers[asking] += 1
                for name in set(project):
                    inclusive[name] += 1
                selfs[project[0]] += 1
                continue
            for name in set(project):
                inclusive[name] += 1
            selfs[project[0]] += 1
            # classify by the innermost frames (raw, below the first project frame)
            innermost = stack[: stack.index(next(f for f in stack if PROJECT.search(f))) + 1]
            text = " ".join(innermost + project[:3])
            if CLANG.search(text):
                classes["clang-wait"] += 1
            elif JSON.search(" ".join(stack[:12])):
                classes["json-parse"] += 1
            else:
                classes["analysis"] += 1
    result = {
        "samples": samples,
        "busy_thread_samples": busy,
        "coordinator_thread_samples": coordinator,
        "contract_environment_askers": [[n, c, round(100 * c / busy, 1)] for n, c in callers.most_common(25)],
        "classes": {k: [v, round(100 * v / busy, 1)] for k, v in classes.most_common()},
        "inclusive_top": [[n, c, round(100 * c / busy, 1)] for n, c in inclusive.most_common(top)],
        "self_top": [[n, c, round(100 * c / busy, 1)] for n, c in selfs.most_common(top)],
    }
    text = json.dumps(result, indent=1)
    if len(sys.argv) > 2 and not sys.argv[2].startswith("--"):
        open(sys.argv[2], "w").write(text + "\n")
    print(text[:12000])


if __name__ == "__main__":
    main()
