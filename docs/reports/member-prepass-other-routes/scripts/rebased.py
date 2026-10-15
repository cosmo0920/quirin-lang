#!/usr/bin/env python3
"""rebased.py -- MEMBER-PREPASS-OTHER-ROUTES: the rebased tip's runs (`a3`)
against the report and null-row digests main `711287771` publishes
(LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's AFTER, docs/reports/librdkafka-plugin-premise),
and against this card's own pre-rebase AFTER (`a2`): totals, digests, null
tallies, wall times, the pre-pass and scope record, and the checkpointed
system run. Writes artifacts/rebased.json and prints it."""
import collections
import hashlib
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
PRIVATE = HERE.parents[3] / "private/mpor"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
MAIN = {"corpus": ("82214412", "5e854d2a"), "system": ("50d2bcaa", "23316f8b"),
        "slice": ("47e5f874", "ed8de3d2")}


def paths(name, route):
    if route == "slice":
        return SLICE / f"mpor-{name}" / "resource-diagnostics.json", PRIVATE / f"{name}-slice"
    if route == "corpus":
        return PRIVATE / name / "resource-diagnostics.json", PRIVATE / name
    return PRIVATE / f"{name}-system" / "resource-diagnostics.json", PRIVATE / f"{name}-system"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    out = {}
    for route in ("corpus", "system", "slice"):
        report, stem = paths("a3", route)
        rows = json.loads(report.read_text())
        totals = dict(collections.Counter(f["status"] for u in rows["units"] for f in u["functions"]))
        null = pathlib.Path(f"{stem}-null.jsonl")
        members = json.loads(report.with_suffix(".contracts.json").read_text())["callback_members"]
        prepass = members["prepass"]
        out[route] = {
            "totals": totals,
            "report_sha256": sha(report),
            "report_matches_main": sha(report).startswith(MAIN[route][0]),
            "null_rows_sha256": sha(null),
            "null_rows_match_main": sha(null).startswith(MAIN[route][1]),
            "null_tallies": json.loads(pathlib.Path(f"{stem}-null.summary.json").read_text())["null"],
            "seconds": pathlib.Path(f"{stem}.seconds").read_text().strip(),
            "prepass": {k: prepass[k] for k in ("units", "read", "rounds", "unread")},
            "scope": {k: v for k, v in prepass["scope"].items() if k != "meaning"},
            "demoted": [members["demoted"], members.get("constructed_states", {}).get("demoted")],
        }
    out["corpus"]["run_json_binary"] = json.loads((PRIVATE / "a3/run.json").read_text())["binary_sha256"]
    whole, ckpt = PRIVATE / "a3-sysw", PRIVATE / "a3-sysc"
    out["system_checkpointed"] = {
        "exits": [(whole / "exit").read_text().strip(), (ckpt / "exit").read_text().strip()],
        "seconds": [(whole / "seconds").read_text().strip(), (ckpt / "seconds").read_text().strip()],
        "report_identical": (whole / "resource-diagnostics.json").read_bytes()
        == (ckpt / "resource-diagnostics.json").read_bytes(),
        "sidecar_identical": (whole / "resource-diagnostics.contracts.json").read_bytes()
        == (ckpt / "resource-diagnostics.contracts.json").read_bytes(),
        "report_sha256": sha(ckpt / "resource-diagnostics.json"),
        "checkpoints": len(list((ckpt / "qo").glob("*.qo"))),
    }
    text = json.dumps(out, indent=1) + "\n"
    (HERE.parent / "artifacts/rebased.json").write_text(text)
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
