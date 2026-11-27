#!/usr/bin/env python3
"""Attribute every edge a configured census reading gains or loses to this
card's derivations.

Usage: attribute_edges.py BEFORE_DIR AFTER_DIR [READING ...]

Each census directory (this card's census.py) holds `READING.functions`
(every function and edge, `--list-functions`) and `READING.json`, for the
six readings. An added edge is
attributed when it is a coroutine-entry dispatch of a collector registrant
whose P-PLUGIN-DIR registration the AFTER reading reports as not derived
(its route is kept); anything else is printed as unattributed. Read-only.
"""

import json
import pathlib
import sys

ENTRY = "include/fluent-bit/flb_input.h::input_pre_cb_collect"


def lines(path):
    return set(pathlib.Path(path).read_text().splitlines())


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    names = sys.argv[3:] or ["fixed-configured", "previous-configured", "fixed", "previous",
                            "fixed-configured-control", "previous-configured-control"]
    for name in names:
        old = lines(before / f"{name}.functions")
        new = lines(after / f"{name}.functions")
        report = json.loads((after / f"{name}.json").read_text())
        registrations = report["typed"]["premises"]["P-PLUGIN-DIR registrations"]
        underived = {line.split(": not derived")[0] for line in registrations
                     if ": not derived" in line}
        added = sorted(new - old)
        removed = sorted(old - new)
        attributed, other = [], []
        for edge in added:
            parts = edge.split()
            ok = (len(parts) >= 5 and parts[0] == "edge" and parts[1] == ENTRY
                  and parts[4] == "CollectorRegistry" and parts[3] in underived)
            (attributed if ok else other).append(edge)
        print(f"== {name}: +{len(added)} / -{len(removed)} lines; "
              f"{len(attributed)} added edges are the kept coroutine route of a registrant "
              f"whose registration is not derived ({len(underived)} such registrants)")
        for edge in attributed:
            print(f"  kept route: {edge}")
        for edge in other:
            print(f"  UNATTRIBUTED added: {edge}")
        for edge in removed:
            print(f"  UNATTRIBUTED removed: {edge}")


if __name__ == "__main__":
    main()
