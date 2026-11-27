#!/usr/bin/env python3
"""Summarize one run directory of W47-FB-DOUBLE-STOP-OBSERVATION.

Usage: summarize.py RUN_DIR LIBFLUENT_BIT_SO VARIANT OUT_JSON  (prints the aggregate)

For each SCENARIO-NN.log / .exit pair it reads the harness's `HARNESS` lines
and, for the debug variant, the interposer's `JOINLOG` lines. Frames are
`object+offset` return addresses; those in libfluent-bit.so are resolved with
`addr2line -f -e LIB (offset-1)` against the library the run used. The engine
thread is the thread created with do_start on the creating stack (the spawn at
src/flb_lib.c:969). Per run it reports: the exit (and signal), each flb_start /
flb_stop return, every join-enter of the engine identifier with its call site
and the value the join returned (absent when the process died inside it),
joins of identifier 0, whether any later create reused the engine identifier
(which would make "the same thread" ambiguous), and the TSan report headers.
The aggregate per scenario counts outcomes and join-site sequences.
"""

import collections
import hashlib
import json
import pathlib
import re
import subprocess
import sys

JOINLOG = re.compile(r"^JOINLOG (\S+) seq=(\d+) self=(\S+) thread=(\S+)(?: ret=(-?\d+))? frames=(.*)$")
HARNESS = re.compile(r"^HARNESS (.*)$")


def resolver(lib):
    cache = {}

    def resolve(offset):
        if offset not in cache:
            out = subprocess.run(
                ["addr2line", "-f", "-e", lib, hex(offset - 1)],
                capture_output=True, text=True, check=True,
            ).stdout.split("\n")
            func = out[0]
            where = re.sub(r"^.*/attempt-1/src-[^/]+/", "", out[1])
            cache[offset] = f"{func}@{where}"
        return cache[offset]

    return resolve


def frames_of(text, resolve):
    result = []
    for frame in filter(None, text.split(";")):
        if frame.startswith("libfluent-bit.so+"):
            result.append(resolve(int(frame.split("+", 1)[1], 16)))
        else:
            result.append(frame.split("+", 1)[0])
    return result


def main():
    run_dir = pathlib.Path(sys.argv[1])
    resolve = resolver(sys.argv[2])
    variant = sys.argv[3]
    runs = []
    for log in sorted(run_dir.glob("*-[0-9][0-9].log")):
        scenario, number = log.stem.rsplit("-", 1)
        exit_code = int((run_dir / f"{log.stem}.exit").read_text().strip())
        text = log.read_text(errors="replace")
        record = {
            "scenario": scenario,
            "run": number,
            "exit": exit_code,
            "signal": exit_code - 128 if exit_code > 128 and exit_code != 124 else None,
            "watchdog_fired": exit_code in (124, 137),
            "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
            "harness": [],
        }
        creates, joins = [], []
        for line in text.splitlines():
            m = HARNESS.match(line)
            if m:
                record["harness"].append(m.group(1))
                continue
            m = JOINLOG.match(line)
            if m:
                kind, seq, _self, thread, ret, frames = m.groups()
                event = {"seq": int(seq), "thread": int(thread, 16),
                         "ret": None if ret is None else int(ret),
                         "frames": frames_of(frames, resolve)}
                (creates if kind == "create" else joins).append((kind, event))
        if variant == "debug":
            engine = [e for _, e in creates if any(f.startswith("do_start@") for f in e["frames"])]
            record["engine_creates"] = len(engine)
            tid = engine[0]["thread"] if engine else None
            record["engine_thread"] = hex(tid) if tid is not None else None
            if tid is not None:
                record["engine_identifier_reused_by_later_create"] = any(
                    e["thread"] == tid and e["seq"] > engine[0]["seq"] for _, e in creates)
            enters = [e for k, e in joins if k == "join-enter"]
            returns = {e["seq"]: e for k, e in joins if k == "join-return"}
            sites = []
            for enter in enters:
                if enter["thread"] != tid and enter["thread"] != 0:
                    continue
                ret = None
                later = [r for s, r in returns.items() if s > enter["seq"] and r["thread"] == enter["thread"]
                         and r["frames"] == enter["frames"]]
                if later:
                    ret = min(later, key=lambda r: r["seq"])["ret"]
                sites.append({
                    "thread": "engine" if enter["thread"] == tid and tid is not None else "0 (never written)",
                    "site": enter["frames"][0],
                    "returned": ret,
                })
            record["engine_or_zero_joins"] = sites
            record["other_joins"] = len([e for e in enters if e["thread"] not in (tid, 0)])
        else:
            record["tsan_reports"] = sorted(set(re.findall(r"WARNING: ThreadSanitizer: ([^\n(]+)", text)))
            record["tsan_summaries"] = re.findall(r"SUMMARY: ThreadSanitizer: ([^\n]+)", text)
            record["tsan_check_failed"] = [
                re.sub(r" \(tid=\d+\)", "", m) for m in re.findall(r"ThreadSanitizer: CHECK failed: ([^\n]+)", text)]
            record["tsan_check_site"] = re.findall(r"#\d+ (flb_\w+ \S+?):\d+ \(libfluent-bit", text.split("CHECK failed", 1)[1])[:1] \
                if "CHECK failed" in text else []
            record["tsan_check_site"] = [re.sub(r"^.*/attempt-1/src-[^/]+/", "", x.split(" ", 1)[1]) for x in record["tsan_check_site"]]
        runs.append(record)
    aggregate = collections.defaultdict(lambda: collections.Counter())
    for record in runs:
        key = record["scenario"]
        outcome = f"exit={record['exit']}"
        aggregate[key]["runs"] += 1
        aggregate[key][outcome] += 1
        if variant == "debug":
            seq = " -> ".join(f"{j['thread']}:{j['site']}:ret={j['returned']}" for j in record["engine_or_zero_joins"])
            aggregate[key]["joins " + seq] += 1
            aggregate[key][f"engine_creates={record['engine_creates']}"] += 1
            if record.get("engine_identifier_reused_by_later_create"):
                aggregate[key]["engine identifier reused"] += 1
        else:
            for report in record["tsan_reports"]:
                aggregate[key]["tsan " + report.strip()] += 1
            for check, site in zip(record["tsan_check_failed"], record["tsan_check_site"] or ["?"]):
                aggregate[key]["tsan CHECK failed " + check + " at " + site] += 1
            for summary_line in record["tsan_summaries"]:
                aggregate[key]["tsan summary " + re.sub(r"^.*/attempt-1/src-[^/]+/", "", summary_line)] += 1
        for line in record["harness"]:
            if line.startswith(("flb_start", "flb_stop")):
                aggregate[key][line] += 1
    summary = {"run_dir": str(run_dir), "variant": variant,
               "aggregate": {k: dict(v) for k, v in sorted(aggregate.items())},
               "runs": runs}
    pathlib.Path(sys.argv[4]).write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(summary["aggregate"], indent=1))


if __name__ == "__main__":
    main()
