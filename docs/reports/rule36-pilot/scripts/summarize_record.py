#!/usr/bin/env python3
"""RULE36-PILOT: print a native-comparison record's status, declared check,
per-comparison route counts, interactions and contribution ledgers.

Usage: summarize_record.py native-comparison.json
"""
import json
import sys


def main():
    record = json.load(open(sys.argv[1]))
    print("status", record["status"], "failures", record["failures"])
    declared = record.get("declared") or {}
    print("declared", declared.get("result"),
          {route: value["result"] for route, value in (declared.get("routes") or {}).items()})
    for comparison in record["comparisons"]:
        print(comparison["before"], "->", comparison["after"])
        for route, value in comparison["routes"].items():
            print("   ", route, value.get("status"), "moves", value.get("moves"),
                  "content", value.get("content_changes"), "unmatched", value.get("unmatched"),
                  "incomplete", value.get("incomplete_because"))
    print("interactions", json.dumps(record.get("interactions"))[:800])
    contribution = record.get("contribution") or {}
    print("contribution refused:", contribution.get("refused"))
    for ledger in contribution.get("ledgers", []):
        rows = [(r["row"], r["before"], r["after"]) for r in ledger.get("rows_moved", [])]
        print("  ledger", ledger["kind"], ledger["key"], ledger["profile"], "delta", ledger.get("delta"),
              "rows", rows[:12], "traded", len((ledger.get("leaves_traded") or {}).get("rows", [])),
              "unknown", len(ledger.get("rows_unknown") or []))


if __name__ == "__main__":
    main()
