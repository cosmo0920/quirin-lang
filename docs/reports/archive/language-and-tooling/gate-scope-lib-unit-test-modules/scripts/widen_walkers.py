#!/usr/bin/env python3
"""Widen the committed walker table until the read census drops nothing.

Card GATE-SCOPE-LIB-UNIT-TEST-MODULES.  Usage (repository root)::

    python3 docs/reports/gate-scope-lib-unit-test-modules/scripts/widen_walkers.py \
        docs/reports/gate-scope-precise-selection/read-census.json > table.json

Narrowing a library's ``#[cfg(test)]`` reads to its unit-test executable takes
away attribution that only reached a unit by accident: a cfg(test) walker of
``contract/profiles`` in qcore-verification's library, read crate-wide, also
claimed every test of the crates depending on it.  Some ``reads: []`` entries
("a base path") were drafted against that accident.  This script starts from
the committed table (not from base paths, so every entry the census still
supports keeps its declaration) and applies the widening rule of
``gate-scope-precise-selection/scripts/draft_walkers.py``, with one
correction: a directory-literal site in ordinary library code runs in the
crate's units *and in every unit of its reverse dependencies*, so a read
observed there is charged to it too.  A charged site widens to
``DIR/**/*.EXT`` when its source spells every extension charged, else to
``DIR/**``; the loop repeats until nothing observed is dropped.  An entry not
drafted from the census (``why`` not starting ``census:``) is never changed.
A drop no site explains is printed to stderr.  The output is reviewed before
it is committed.
"""
import json
import posixpath
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import gate_scope as scope_mod  # noqa: E402

PREFIX = "census: "


def main() -> int:
    census = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["units"]
    table = scope_mod.load_table()
    crates = scope_mod.load_crates()
    files = scope_mod.tracked_files()
    walkers = [dict(entry) for entry in table["walkers"]]
    by_key = {(entry["source"], entry["literal"]): entry for entry in walkers}
    changed: dict[tuple[str, str], list[str]] = {}
    for _ in range(20):
        model = scope_mod.Model(crates, files, walkers=walkers, expansions=table["expansions"])
        units_of: dict[str, set[str]] = {}
        for name, targets in model.targets.items():
            for target in targets:
                units_of.setdefault(name, set()).add(scope_mod.unit_uid(name, target))
        # Every directory-literal reader the model holds, by site, with the
        # units that run it.
        sites: dict[tuple[str, str], set[str]] = {}
        for key, readers in model.walker_sites.items():
            for crate, narrowed in readers:
                if narrowed is not None:
                    units = set(narrowed)
                else:
                    units = set(units_of.get(crate, ()))
                    for dependent in model.reverse_closure([crate]):
                        units |= units_of.get(dependent, set())
                sites.setdefault(key, set()).update(units)
        tracked = set(model.files)
        cache: dict[str, dict] = {}
        charged: dict[tuple[str, str], set[str]] = {}
        unexplained = []
        for uid, unit in census.items():
            for path in unit["read"]:
                if path not in tracked:
                    continue
                if path not in cache:
                    cache[path] = scope_mod.plan([path], model, lean=False)
                if scope_mod.selects(cache[path], uid):
                    continue
                hit = False
                for key, units in sites.items():
                    entry = by_key[key]
                    if path.startswith(key[1] + "/") and uid in units \
                            and entry["why"].startswith(PREFIX):
                        charged.setdefault(key, set()).add(path)
                        hit = True
                if not hit:
                    unexplained.append(f"{uid} reads {path}")
        if not charged:
            break
        for key, paths in charged.items():
            source, literal = key
            entry = by_key[key]
            text = (ROOT / source).read_text(encoding="utf-8", errors="replace")
            reads = entry["reads"]
            extensions = sorted({posixpath.splitext(p)[1].lstrip(".") for p in paths}
                                | {r.rsplit("*.", 1)[1] for r in reads if "*." in r})
            if all(ext and re.search(r'"\.?%s"' % re.escape(ext), text) for ext in extensions) \
                    and f"{literal}/**" not in reads:
                entry["reads"] = [f"{literal}/**/*.{ext}" for ext in extensions]
                entry["why"] = (f"{PREFIX}its units, or its reverse dependencies' units, open "
                                f"files under it, all *.{', *.'.join(extensions)}, the "
                                "extension(s) its source spells")
            else:
                entry["reads"] = [f"{literal}/**"]
                entry["why"] = (f"{PREFIX}its units, or its reverse dependencies' units, open "
                                "files under it and its source spells no filter covering "
                                "them: the whole directory")
            changed[key] = sorted(paths)
    for line in sorted(set(unexplained)):
        print(f"unexplained: {line}", file=sys.stderr)
    for (source, literal), paths in sorted(changed.items()):
        print(f"widened: {source} {literal}: {len(paths)} dropped reads, e.g. {paths[0]}",
              file=sys.stderr)
    print(json.dumps({"schema": scope_mod.WALKERS_SCHEMA, "note": table.get("note"),
                      "walkers": walkers, "expansions": table["expansions"]}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
