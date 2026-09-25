#!/usr/bin/env python3
"""Collect the card's measurement directories into committed evidence.

Usage (worktree root): collect.py <measure-dir> <evidence-dir>

Reads the worktree-private measure/<run>/ directories written by
measure_cargo.sh, build_options.sh, clang_share.sh, ab_test.py and
verify_env.py, and writes:
  runs.json            every timed run: command, exit, wall, phases, peak RSS,
                       target/ size, load and free disk before/after, totals
  build-options.json   build-only profile measurements
  per-binary.json      per-binary seconds, base vs each later run
  tests/<run>.json.gz  sorted test lines and summaries per binary (equivalence)
  clang-launches.json  Clang launch counts of the heaviest binary
  verify-env.json      the strace/capture environment comparison, values elided
No environment values are copied: verify-env keeps variable names only.
"""

import gzip
import json
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cargo_log  # noqa: E402
import compare  # noqa: E402

RUNS = {
    "base-1": ("base", "plain cargo test, base profile (opt 0, full debuginfo)"),
    "profile-plain-1": ("option", "plain cargo test, debug = line-tables-only"),
    "runner-j8-a": ("option", "runner --jobs 8, base profile, no weights"),
    "final-1": ("option", "runner --jobs 8 + line-tables-only (before the Clang cache)"),
    "profile-cache-plain-1": ("option", "plain cargo test, line-tables-only + Clang resolution cache"),
    "final-cache-1": ("final", "runner (default --jobs 8) + line-tables-only + Clang cache"),
    "final-cache-2": ("final", "runner (default --jobs 8) + line-tables-only + Clang cache"),
    "final-cache-3": ("final", "runner (default --jobs 8) + line-tables-only + Clang cache"),
    "jobs-4": ("jobs", "final configuration with --jobs 4"),
    "jobs-16": ("jobs", "final configuration with --jobs 16"),
    "attempt-1-relative-logdir": ("failed-attempt", "first runner attempt: relative --log-dir broke the "
                                  "capture runner; the run failed closed (exit 101)"),
}


def read(path: Path) -> str:
    return path.read_text().strip() if path.exists() else ""


def elapsed_seconds(text: str) -> float | None:
    match = re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (\S+)", text)
    if not match:
        return None
    parts = [float(p) for p in match[1].split(":")]
    seconds = 0.0
    for part in parts:
        seconds = seconds * 60 + part
    return round(seconds, 2)


def peak_rss_kib(text: str) -> int | None:
    match = re.search(r"Maximum resident set size \(kbytes\): (\d+)", text)
    return int(match[1]) if match else None


def load_of(text: str) -> str:
    return text.split("load average: ")[-1] if text else ""


def free_disk(text: str) -> str:
    lines = text.splitlines()
    return lines[-1].split()[3] if len(lines) >= 2 else ""


def run_record(directory: Path, kind: str, label: str) -> dict:
    time_text = read(directory / "time.txt")
    record = {
        "run": directory.name,
        "kind": kind,
        "configuration": label,
        "exit": int(read(directory / "exit") or -1),
        "wall_seconds": elapsed_seconds(time_text),
        "peak_rss_kib": peak_rss_kib(time_text),
        "target_bytes": int(read(directory / "target.bytes").split()[0]) if read(directory / "target.bytes") else None,
        "load_before": load_of(read(directory / "uptime.before")),
        "load_after": load_of(read(directory / "uptime.after")),
        "free_disk_before": free_disk(read(directory / "df.before")),
        "start": read(directory / "start"),
    }
    summary = directory / "logs" / "summary.json"
    if summary.exists():
        document = json.loads(summary.read_text())
        record["phases_seconds"] = document["phases_seconds"]
        record["totals"] = document["totals"]
        record["runner_ok"] = document["ok"]
        record["jobs"] = document["jobs"]
        record["problems"] = document["problems"]
        record["units"] = document["units_run"]
        exclusive = [u for u in document["units"] if u["exclusive"]]
        record["exclusive_units"] = {u["uid"]: {"ok": u["ok"], "seconds": u["seconds"]} for u in exclusive}
        intervals = document["intervals"]
        if intervals:
            t0 = min(i["start"] for i in intervals)
            parallel = [i for i in intervals if not i["exclusive"]]
            record["parallel_phase_end"] = round(max(i["end"] for i in parallel) - t0, 1)
            record["exclusive_overlaps"] = [
                [a["uid"], b["uid"]] for a in intervals if a["exclusive"] for b in intervals
                if a is not b and a["start"] < b["end"] and b["start"] < a["end"]
            ]
    else:
        output = directory / "output.log"
        if output.exists():
            records, first, last = cargo_log.split(output)
            merged = cargo_log.summarize(records)
            record["phases_seconds"] = {"build": round(first, 1), "run": round(last - first, 1)}
            total = {k: sum(int(s[k]) for e in merged.values() for s in e["summaries"])
                     for k in compare.COUNT_KEYS}
            total["summaries"] = sum(len(e["summaries"]) for e in merged.values())
            record["totals"] = total
    return record


def per_binary(measure: Path) -> dict:
    table: dict[str, dict] = {}
    for name in RUNS:
        directory = measure / name
        summary = directory / "logs" / "summary.json"
        output = directory / "output.log"
        if summary.exists():
            for unit in json.loads(summary.read_text())["units"]:
                key = "doc-tests" if unit["uid"] == "doc-tests" else cargo_log.stem(unit["executable"])
                table.setdefault(key, {})[name] = round(
                    table.get(key, {}).get(name, 0.0) + unit["seconds"], 1)
        elif output.exists():
            records, _, _ = cargo_log.split(output)
            for key, entry in cargo_log.summarize(records).items():
                table.setdefault(key, {})[name] = round(entry["seconds"], 1)
    return dict(sorted(table.items(), key=lambda kv: -kv[1].get("base-1", 0.0)))


def main() -> int:
    measure, evidence = Path(sys.argv[1]), Path(sys.argv[2])
    evidence.mkdir(parents=True, exist_ok=True)
    runs = [run_record(measure / name, kind, label) for name, (kind, label) in RUNS.items()
            if (measure / name).exists()]
    (evidence / "runs.json").write_text(json.dumps(runs, indent=1) + "\n")

    options = []
    for round_dir in sorted(measure.glob("build-opts*")):
        for option in sorted(round_dir.iterdir()):
            time_text = read(option / "time.txt")
            options.append({
                "round": round_dir.name, "option": option.name,
                "exit": int(read(option / "exit") or -1),
                "build_seconds": elapsed_seconds(time_text),
                "peak_rss_kib": peak_rss_kib(time_text),
                "target_bytes": int(read(option / "target.bytes").split()[0]),
                "load_before": load_of(read(option / "uptime.before")),
            })
    (evidence / "build-options.json").write_text(json.dumps(options, indent=1) + "\n")

    (evidence / "per-binary.json").write_text(json.dumps(per_binary(measure), indent=1) + "\n")

    tests_dir = evidence / "tests"
    tests_dir.mkdir(exist_ok=True)
    for name in RUNS:
        directory = measure / name
        source = directory / "logs" / "summary.json"
        if not source.exists():
            source = directory / "output.log"
        if not source.exists() or name == "attempt-1-relative-logdir":
            continue
        side = compare.load(source)
        data = {key: {"summaries": entry["summaries"], "tests": entry["tests"]}
                for key, entry in sorted(side.items())}
        with gzip.open(tests_dir / f"{name}.json.gz", "wt", encoding="utf-8") as handle:
            json.dump(data, handle, indent=0, sort_keys=True)

    launches = {}
    for name in ("clang-share-ucrp", "clang-share-ucrp-cached"):
        log = measure / name / "clang.log"
        if not log.exists():
            continue
        kinds: dict[str, int] = {}
        for line in log.read_text().splitlines():
            args = " ".join(line.split()[2:])
            args = re.sub(r"-ast-dump-filter -Xclang \S+", "-ast-dump-filter -Xclang NAME", args)
            kinds[args] = kinds.get(args, 0) + 1
        launches[name] = {"total": sum(kinds.values()), "by_arguments": kinds,
                          "report": read(measure / name / "report.txt"),
                          "load": read(measure / name / "uptime")}
    (evidence / "clang-launches.json").write_text(json.dumps(launches, indent=1) + "\n")

    verify = {}
    for name in ("verify-env-schemas", "verify-env-compiler"):
        path = measure / name / "verify.json"
        if path.exists():
            document = json.loads(path.read_text())
            verify[name] = {
                "problems": document["problems"],
                "env_vars_compared": document["env_vars_compared"],
                "argv_equal": document["observed"]["argv"] == document["captured"]["argv"],
                "observed_cwd": document["observed"]["cwd"],
                "captured_cwd": document["captured"]["cwd"],
                "cargo_set_variables": sorted(k for k in document["observed"]["env"]
                                              if k.startswith(("CARGO", "LD_LIBRARY_PATH"))),
            }
    (evidence / "verify-env.json").write_text(json.dumps(verify, indent=1) + "\n")
    print(f"wrote {len(runs)} runs, {len(options)} build options")
    return 0


if __name__ == "__main__":
    sys.exit(main())
