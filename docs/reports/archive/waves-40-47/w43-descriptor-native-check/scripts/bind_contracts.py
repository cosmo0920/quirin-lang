#!/usr/bin/env python3
"""Phase B: bind every accepted run's records to the landed libc contracts.

Usage: bind_contracts.py RUN_DIR OUT.json

RUN_DIR is a run_scenarios.py output (or an extracted pinned archive): it has
summary.json and runs/<name>/records.jsonl. For every run the replay accepted
(checker_exit 0), runs the `bind_descriptor_contracts` example over the four
`quirin-provider-libc` modules that carry the descriptor, stream and
directory contracts, writes runs/<name>/binding.json, and aggregates:
per bound callee, executions, successes, failures, injected, contradictions
(with run names), caller observations, zero-descriptor results and facts;
per unbound callee, the summed shape profile. Deterministic: sorted keys,
runs in summary order. Exit 1 if any binding was rejected.
"""
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
BINDER = REPO / "target/release/examples/bind_descriptor_contracts"
LIBC = REPO / "providers/quirin-provider-libc/contracts/libc"
MODULES = [LIBC / name for name in ("dirent.qcore", "socket.qcore", "stdio.qcore", "unistd.qcore")]


def add(total, part):
    for key, value in part.items():
        if isinstance(value, bool) or key == "contract":
            continue
        if isinstance(value, int):
            total[key] = total.get(key, 0) + value
        elif isinstance(value, dict):
            add(total.setdefault(key, {}), {str(k): v for k, v in value.items()})


def main() -> int:
    run_dir, out = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2])
    if not BINDER.is_file():
        raise SystemExit(f"build the binder first: {BINDER}")
    summary = json.loads((run_dir / "summary.json").read_text())
    bound, unbound, rejected = {}, {}, []
    contradiction_runs, zero_runs, caller_runs = {}, {}, {}
    accepted = 0
    for run in summary["runs"]:
        if run["checker_exit"] != 0:
            continue
        accepted += 1
        name = run["name"]
        records = run_dir / "runs" / name / "records.jsonl"
        done = subprocess.run([str(BINDER), str(records), *map(str, MODULES)], text=True, capture_output=True)
        if done.returncode != 0:
            rejected.append({"run": name, "error": done.stderr.strip()})
            continue
        (run_dir / "runs" / name / "binding.json").write_text(done.stdout)
        report = json.loads(done.stdout)["report"]
        for callee, binding in report["bound"].items():
            total = bound.setdefault(callee, {"contract": binding["contract"], "runs": 0})
            total["runs"] += 1
            add(total, {k: v for k, v in binding.items() if not k.endswith("examples")})
            if binding["contradictions"]:
                contradiction_runs.setdefault(callee, []).append(
                    {"run": name, "examples": binding["contradiction_examples"]})
            if binding["zero_results"]:
                zero_runs.setdefault(callee, []).append(name)
            for cls in binding["caller_observations"]:
                caller_runs.setdefault(f"{callee}:{cls}", []).append(name)
        for callee, shape in report["unbound"].items():
            total = unbound.setdefault(callee, {"runs": 0})
            total["runs"] += 1
            add(total, shape)
    result = {
        "schema_version": 1,
        "modules": [str(path.relative_to(REPO)) for path in MODULES],
        "runs_bound": accepted - len(rejected),
        "rejected": rejected,
        "contradictions": sum(b.get("contradictions", 0) for b in bound.values()),
        "bound": bound,
        "contradiction_runs": contradiction_runs,
        "zero_descriptor_result_runs": zero_runs,
        "caller_observation_runs": caller_runs,
        "unbound": unbound,
        "static_certificate": False,
    }
    out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(json.dumps({"runs_bound": result["runs_bound"], "rejected": len(rejected),
                      "contradictions": result["contradictions"], "bound_callees": len(bound)}))
    return 1 if rejected else 0


if __name__ == "__main__":
    sys.exit(main())
