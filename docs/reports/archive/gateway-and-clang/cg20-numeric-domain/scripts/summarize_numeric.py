#!/usr/bin/env python3
"""Summarize numeric_obligations reports: one block per function, one line per
site with its obligations (CG20-NUMERIC-DOMAIN)."""

import json
import sys


def main() -> int:
    for path in sys.argv[1:]:
        report = json.load(open(path))
        print(f"# {path}: {report['data_model']} {report['target']['triple']}")
        for function in report["functions"]:
            print(
                f"{function['name']}: {function['verdict']}"
                f" refused={function['refused']} outside={function['outside']}"
            )
            for site in function["sites"]:
                obligations = ", ".join(
                    f"{o['fault']}={o['status']}" for o in site["obligations"]
                )
                print(f"  line {site['line']}: {site['operation']} [{site['c_type']}] {obligations}")
            for event in function["events"]:
                print(f"  event line {event['line']}: {event['kind']} {event['detail']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
