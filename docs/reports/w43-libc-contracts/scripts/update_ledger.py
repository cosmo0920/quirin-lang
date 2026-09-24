#!/usr/bin/env python3
"""Apply W43-LIBC-CONTRACTS's hand-maintained matrix edits, once.

Usage: update_ledger.py

* appends every entry of `followups.json` to `named_followups` (refusing a
  duplicate id);
* retires `LOADER-ABI-RESTRICT-QUALIFIER`, which must be registered, whose
  discharge condition was exactly the measurement this card made, and records
  the retirement beside the report;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-libc-contracts"
RETIRED = "LOADER-ABI-RESTRICT-QUALIFIER"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    if RETIRED not in ids:
        raise SystemExit(f"{RETIRED} is not registered; nothing to retire")
    retired = next(entry for entry in entries if entry["id"] == RETIRED)
    (REPORT / "retired-followups.json").write_text(
        json.dumps(
            [
                {
                    "entry": retired,
                    "discharged_by": "W43-LIBC-CONTRACTS",
                    "ground": "Its discharge condition is one measurement: declare a `__restrict`-qualified "
                    "libc callee and observe whether the declared-ABI reader withdraws it. This card declared "
                    "fourteen (printf, fprintf, snprintf, sprintf, stat, strftime, gettimeofday, getsockopt, "
                    "getpeername, dlsym, cnd_timedwait, pthread_sigmask, regexec, regerror) in a private trial "
                    "arbiter: every one was withdrawn with `abi-unreadable-declaration` at every call site "
                    "(2,577 withdrawn call-site leaves), and a five-unit probe isolates the cause to a qualifier "
                    "written after the `*`. abi-evidence.json. The open repair is registered as "
                    "W43-LIBC-POINTER-QUALIFIER-READER.",
                }
            ],
            indent=2,
        )
        + "\n"
    )
    matrix["named_followups"] = [entry for entry in entries if entry["id"] != RETIRED] + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n")
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
    functions = collections.Counter()
    units = collections.Counter()
    reason_codes = collections.Counter()
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        reason_codes[unit.get("reason_code") or "none"] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": "W43-LIBC-CONTRACTS, 2026-09-23. Declarative arbiter, one audit binary for both stages. "
            "BEFORE reproduces the W43-RETURN-EXPRESSION diagnostics byte for byte. AFTER selects eleven new "
            "libc/ modules (55 trusted libc/POSIX declarations in providers/quirin-provider-libc): 314 leaves "
            "removed, 27 added, all attributed (229 declared-callee, 4 newly-Safe callee summaries, 86 same "
            "line, 22 same function); 22 functions Unknown -> Safe, each relative to the declared premises; "
            "0 unit status moves. Leak 272 preserved. Raw units remain unpromoted; provisional Unsafe is not "
            "an upstream defect claim. Previous row preserved in "
            "docs/reports/w43-libc-contracts/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-libc-contracts/artifacts/",
            "source": "docs/reports/w43-libc-contracts/README.md; preceding baseline "
            "docs/reports/w43-return-expression/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
            "unit_reason_codes": dict(sorted(reason_codes.items(), key=lambda kv: (-kv[1], kv[0]))),
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
