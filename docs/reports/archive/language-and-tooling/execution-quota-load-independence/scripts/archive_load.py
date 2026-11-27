#!/usr/bin/env python3
"""EXECUTION-QUOTA-LOAD-INDEPENDENCE: archive the load campaigns.

Copies every loadbisect.py result under private/eqli/ into artifacts/load/
(one JSON per measurement set, with the captured load PIDs reduced to a count,
log paths made relative to private/eqli/, and each failure's panic line kept),
and writes artifacts/load/SUMMARY.md with summarize.py's table over the sets
in the order the report cites them. Idempotent."""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/eqli"
OUT = HERE.parent / "artifacts/load"

# label, private file, what it measures
SETS = [
    ("pilot-cpu", "pilot.json", "base binary, CPU spinners"),
    ("pilot-peers", "pilot2.json", "diagnostic base, concurrent copies / netns churn / mixed"),
    ("pilot-cpu-heavy", "pilot3.json", "diagnostic base, CPU up to 256 spinners"),
    ("build-beside", "pilot4-build.json", "diagnostic base, a release build running beside it"),
    ("idle", "base-idle60.json", "diagnostic base, nothing added"),
    ("host-stall", "stall1.json", "diagnostic base, test process stopped 0..N ms at a time"),
    ("mixed", "mix1.json", "diagnostic base, stall 50 / mixed / 3,000 sleepers"),
    ("sleepers", "sleepers.json", "diagnostic base, 0 / 1,000 / 3,000 / 5,000 idle same-UID processes"),
    ("q-base-child20", "c1-base-child20.json", "diagnostic base, forked-child stall 20 ms"),
    ("q-base-host10", "c1-base-host10.json", "diagnostic base, host stall 10 ms"),
    ("q-after-child20", "c1-after-child20.json", "fixed, forked-child stall 20 ms"),
    ("q-after-host20", "c1-after-host20.json", "fixed, host stall 20 ms"),
    ("q-after-host50", "c1-after-host50.json", "fixed, host stall 50 ms"),
    ("q-base-close", "c3-base-close20.json", "diagnostic base, close-delay amplifier"),
    ("q-base-close-child", "c3-base-close20-child.json", "diagnostic base, close delay + child stall 5/20/50 ms"),
    ("q-after-close-child", "c3-after-close20-child50.json", "fixed, close delay + child stall 50 ms"),
    ("q-base-clock1us", "c3-base-clock1us.json", "diagnostic base, CLOCK_REALTIME rounded to 1 us"),
    ("q-base-clock1ms", "c3-base-clock1ms.json", "diagnostic base, CLOCK_REALTIME rounded to 1 ms"),
    ("q-after-clock1ms", "c3-after-clock1ms.json", "fixed, CLOCK_REALTIME rounded to 1 ms"),
]
for binary, short in [("confined_process_execution", "cpe"),
                      ("enforced_process_limits", "epl"),
                      ("package_identity", "pi")]:
    for side in ("base", "after"):
        SETS.append((f"{short}-{side}-stall", f"c2-{side}-{binary}-stall.json", f"{side} {binary}, host stall 20 / 50 ms"))
        SETS.append((f"{short}-{side}-fork", f"c2-{side}-{binary}-fork.json", f"{side} {binary}, close delay + child stall 50 ms"))
SETS += [
    ("epl-base-count", "c2-base-enforced-count.json", "base enforced_process_limits, 3,000 sleepers / 64 spinners + 8 fork loops"),
    ("epl-after-count", "c2-after-enforced-count.json", "fixed enforced_process_limits, same"),
    ("cpe-base-child-only", "x-cpe-child.json", "attribution probe: base confined, child stall only"),
    ("cpe-base-close-only", "x-cpe-close.json", "attribution probe: base confined, close delay only"),
]


def failures(log: str) -> list:
    found = []
    lines = log.splitlines()
    for index, line in enumerate(lines):
        if not (line.startswith("---- ") and line.endswith(" stdout ----")):
            continue
        test = line[len("---- "):-len(" stdout ----")].split("::")[-1]
        detail = ""
        for offset in range(index + 1, min(index + 12, len(lines))):
            if "panicked at" in lines[offset]:
                detail = " :: ".join(part.strip() for part in lines[offset:offset + 4]
                                     if not part.startswith("note: run with"))
                break
        found.append({"test": test, "panic": detail})
    return found


def private_relative(path: str) -> str:
    """A path under private/eqli/, absolute or relative to the worktree."""
    candidate = pathlib.Path(path)
    if not candidate.is_absolute():
        candidate = REPO / candidate
    return str(candidate.relative_to(PRIVATE))


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    args = []
    for label, name, _ in SETS:
        data = json.loads((PRIVATE / name).read_text())
        data["binary"] = private_relative(data["binary"])
        data["env"] = [item if not item.startswith("LD_PRELOAD=") else "LD_PRELOAD=shim/amplify.so"
                       for item in data.get("env", [])]
        for level in data["levels"]:
            level["load_pid_count"] = len(level.pop("load_pids", []))
            for run in level["runs"]:
                run["log"] = private_relative(run["log"])
                # Re-read each failure's panic with the three lines after it,
                # so an `assert_eq!` failure keeps its `left:` value (the
                # signature) rather than only its first line.
                if run["failures"]:
                    run["failures"] = failures((PRIVATE / run["log"]).read_text(errors="replace"))
        target = OUT / f"{label}.json"
        target.write_text(json.dumps(data, indent=1) + "\n")
        args.append(f"{label}={target}")
    table = subprocess.run([sys.executable, str(HERE / "summarize.py"), *args],
                           capture_output=True, text=True, check=True).stdout
    legend = "\n".join(f"- `{label}`: {what} (`{name}`)" for label, name, what in SETS)
    (OUT / "SUMMARY.md").write_text(
        "# Load campaigns (generated by scripts/archive_load.py)\n\n" + table + "\n" + legend + "\n")
    print(len(SETS), "sets archived")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
