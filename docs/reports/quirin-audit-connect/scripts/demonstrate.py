#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card QUIRIN-AUDIT-CONNECT: run `measure chain` over committed
native-comparison records (no re-measurement) and summarise where links join
and where they break.

    python3 docs/reports/quirin-audit-connect/scripts/demonstrate.py PLUGIN OUT

PLUGIN is the built `quirin-audit-measure` executable; it is driven with the
plugin protocol's request on standard input, exactly as `quirin-audit
--plugin-path DIR measure chain ...` drives it. OUT receives one directory
per chain (its `chain.json`) and `summary.json`.
"""

import gzip
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
REPORTS = ROOT / "docs/reports"

CHAINS = {
    # The brief's demonstration: the landing record of 556460e31 followed by
    # the later landings' integration records that might join it.
    "landing-556460e31-then-sp1": [
        "landing-556460e31/native-comparison.json.gz",
        "sp1-symbolic-openssl-premises/integration/native-comparison.json.gz",
    ],
    "landing-556460e31-then-member-effect-component-scope": [
        "landing-556460e31/native-comparison.json.gz",
        "member-effect-component-scope/integration/native-comparison.json.gz",
    ],
    "landing-556460e31-then-landing-777fba16e": [
        "landing-556460e31/native-comparison.json.gz",
        "landing-777fba16e/native-comparison.json.gz",
    ],
    # Chains whose records join (each later base is the earlier tip, same
    # binary, inputs, bundles and measurement tool).
    "out-parameter-three-landings": [
        "out-parameter-body-summary/integration/native-comparison.json.gz",
        "out-parameter-scalar-frame-premise/integration/native-comparison.json.gz",
        "out-slot-call-write-aliasing/integration/native-comparison.json.gz",
    ],
    "sp2-sp3-sp4": [
        "sp2-symbolic-librdkafka-premises/integration/native-comparison.json.gz",
        "sp3-symbolic-fluent-bit-libc-premises/integration/native-comparison.json.gz",
        "sp4-symbolic-libevent-premises/integration/native-comparison.json.gz",
    ],
    "member-effect-component-scope-then-member-release-untyped-base": [
        "member-effect-component-scope/integration/native-comparison.json.gz",
        "member-release-untyped-base/integration/native-comparison.json.gz",
    ],
}


def run(plugin, output, arguments):
    output.mkdir(parents=True, exist_ok=True)
    output = output.resolve()
    request = {
        "protocol": "quirin-audit-plugin", "protocol_version": 1, "subcommand": "measure",
        "arguments": arguments, "invocation_directory": str(ROOT), "output": str(output),
        "report": str(output / "quirin-audit-measure.report.json"), "load_path": None,
        "providers": [], "identity": {"revision": None, "contract_set_sha256": None,
                                      "refuted_table_sha256": None},
        "audit_report": None, "checkpoints": [],
        "quotas": {"wall_time_ms": 3_600_000, "stdout_bytes": 1 << 24, "stderr_bytes": 1 << 24},
    }
    completed = subprocess.run([plugin, *arguments], input=json.dumps(request).encode(),
                               cwd=output, capture_output=True, env={"PATH": os.environ["PATH"]})
    response = json.loads(completed.stdout)
    return completed.returncode, response


def main():
    plugin, out = sys.argv[1], Path(sys.argv[2])
    summary = {}
    for name, records in CHAINS.items():
        arguments = ["chain"]
        for record in records:
            arguments += ["--step", str(REPORTS / record)]
        code, response = run(plugin, out / name, arguments)
        chain = json.loads((out / name / "chain.json").read_text())
        with gzip.open(out / name / "chain.json.gz", "wt") as handle:
            handle.write(json.dumps(chain, indent=1, sort_keys=True) + "\n")
        (out / name / "chain.json").unlink()
        entry = {
            "steps": [{"record": r, "comparison": s["comparison"],
                       "before": s["before"]["revision"], "after": s["after"]["revision"]}
                      for r, s in zip(records, chain["steps"])],
            "status": chain["status"],
            "plugin_exit": code,
            "outcome": response["outcome"],
            "links": [{"between": link["between"], "joined": link["joined"],
                       "refused_because": sorted({r["formula"] for r in link["refused_because"]})}
                      for link in chain["links"]],
            "segments": [],
            "unattributed_moves": len(chain["unattributed_moves"]),
            "unattributed_new_safe": chain["unattributed_new_safe"],
        }
        for segment in chain["segments"]:
            routes = {}
            for route, record in segment["routes"].items():
                rows = record["rows"]
                routes[route] = {
                    "status": record["status"], "counts": record["counts"],
                    "net_status_delta": record["net_status_delta"],
                    "cancelled": [r["row"] for r in rows if r["pattern"] == "cancelled"],
                    "cascade": [r["row"] for r in rows if r["pattern"] == "cascade"],
                    "moved": [{"row": r["row"], "verdicts": r["verdicts"],
                               "attributed_steps": sorted({a["step"] for a in r["attributed"]})}
                              for r in rows if r["pattern"] in ("moved", "cancelled", "cascade")],
                }
            entry["segments"].append({"steps": segment["steps"], "routes": routes,
                                      "bundle_changes": [
                                          {"step": c["step"], "crates": sorted(c["bundles"])}
                                          for c in segment["contract_changes"]]})
        summary[name] = entry
    (out / "summary.json").write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
    for name, entry in summary.items():
        print(name, entry["status"], [(l["between"], l["joined"], l["refused_because"])
                                      for l in entry["links"]])


if __name__ == "__main__":
    main()
