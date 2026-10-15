#!/usr/bin/env python3
"""summarize_runs.py -- MEMBER-PREPASS-OTHER-ROUTES: BEFORE (`before2`
corpus, `before` system and slice; binary private/mpor/bin/before built from
the base tree) against AFTER (`a2`) on every route: totals, report digests
and byte identity, null rows and tallies, wall times, run.json binary
digests, the sidecar keys that differ (base-tree paths normalised), the
callback-members section's pre-pass and members, and the checkpointed system
run against its whole twin. Writes artifacts/summary.json and
artifacts/callback-members-{system,slice}.json, and prints the summary."""
import collections
import hashlib
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/mpor"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ARTIFACTS = HERE.parent / "artifacts"
BASE = str(PRIVATE / "base-src")


def report(name, route):
    if route == "slice":
        return SLICE / f"mpor-{name}" / "resource-diagnostics.json"
    if route == "corpus":
        return PRIVATE / name / "resource-diagnostics.json"
    return PRIVATE / f"{name}-system" / "resource-diagnostics.json"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def totals(path):
    data = json.loads(path.read_text())
    return dict(collections.Counter(f["status"] for u in data["units"] for f in u["functions"]))


def text(path):
    return path.read_text().strip() if path.exists() else None


def main():
    out = {}
    names = {"corpus": ("before2", "a2"), "system": ("before", "a2"), "slice": ("before", "a2")}
    for route, (before, after) in names.items():
        paths = [report(before, route), report(after, route)]
        suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
        nulls = [PRIVATE / f"{name}{suffix}-null.jsonl" for name in (before, after)]
        tallies = [json.loads((PRIVATE / f"{name}{suffix}-null.summary.json").read_text())["null"]
                   for name in (before, after)]
        sidecars = [json.loads(p.with_suffix(".contracts.json").read_text().replace(BASE, str(REPO)))
                    for p in paths]
        differing = sorted(k for k in set(sidecars[0]) | set(sidecars[1])
                           if sidecars[0].get(k) != sidecars[1].get(k))
        members = sidecars[1].get("callback_members", {})
        members_before = sidecars[0].get("callback_members", {})
        entry = {
            "before": {"name": before, "totals": totals(paths[0]), "sha256": digest(paths[0]),
                       "seconds": text(PRIVATE / f"{before}{suffix}.seconds")},
            "after": {"name": after, "totals": totals(paths[1]), "sha256": digest(paths[1]),
                      "seconds": text(PRIVATE / f"{after}{suffix}.seconds")},
            "report_identical": paths[0].read_bytes() == paths[1].read_bytes(),
            "null": {"rows_identical": nulls[0].read_bytes() == nulls[1].read_bytes(),
                     "tallies": tallies},
            "sidecar_keys_differing": differing,
            "callback_members_keys_differing": sorted(
                k for k in set(members) | set(members_before)
                if members.get(k) != members_before.get(k)),
            "prepass": {k: members.get("prepass", {}).get(k)
                        for k in ("units", "read", "rounds", "unread")},
            "prepass_tokens": len(members.get("prepass", {}).get("tokens", [])),
            "scope": {k: v for k, v in members.get("prepass", {}).get("scope", {}).items()
                      if k != "meaning"},
            "members_after": {m: (v.get("closed"), len(v.get("candidates", v.get("open", []))))
                              for m, v in members.get("members", {}).items()},
            "members_before": {m: (v.get("closed"), len(v.get("candidates", v.get("open", []))))
                               for m, v in members_before.get("members", {}).items()},
            "demoted": [members.get("demoted"),
                        members.get("constructed_states", {}).get("demoted")],
        }
        if route == "corpus":
            entry["run_json_binary"] = [json.loads((PRIVATE / n / "run.json").read_text())
                                        .get("binary_sha256") for n in (before, after)]
        else:
            ARTIFACTS.joinpath(f"callback-members-{route}.json").write_text(
                json.dumps(members, indent=1) + "\n")
        out[route] = entry
    whole, ckpt = PRIVATE / "a2-sysw", PRIVATE / "a2-sysc"
    out["system_checkpointed"] = {
        "whole_seconds": text(whole / "seconds"), "checkpointed_seconds": text(ckpt / "seconds"),
        "exits": [text(whole / "exit"), text(ckpt / "exit")],
        "report_identical": (whole / "resource-diagnostics.json").read_bytes()
        == (ckpt / "resource-diagnostics.json").read_bytes(),
        "sidecar_identical": (whole / "resource-diagnostics.contracts.json").read_bytes()
        == (ckpt / "resource-diagnostics.contracts.json").read_bytes(),
        "report_sha256": digest(ckpt / "resource-diagnostics.json"),
        "whole_equals_null_run_report": (whole / "resource-diagnostics.json").read_bytes()
        == report("a2", "system").read_bytes(),
        "checkpoints": len(list((ckpt / "qo").glob("*.qo"))),
        "console_lines": [line for line in (ckpt / "console.log").read_text().splitlines()
                          if line.startswith(("member facts", "callback members"))],
    }
    (ARTIFACTS / "summary.json").write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
