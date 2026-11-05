#!/usr/bin/env python3
"""Draft the walker table's directory-literal entries from a read census.

Card GATE-SCOPE-PRECISE-SELECTION.  Usage (repository root)::

    python3 docs/reports/gate-scope-precise-selection/scripts/draft_walkers.py \
        CENSUS.json FULL_CENSUS.json > draft.json

Starts from the committed table's hand-written entries (``literal: "*"``,
expansions, and any entry whose ``why`` does not begin with ``census:``) and
gives every other directory literal the scanner finds the narrowest
declaration the census supports, widening only on evidence:

1. a site compiled into provider-workspace code (not in the census), or one
   whose units opened the directory itself or a directory under it, or ran
   ``git ls-files`` on it or a parent (a listing: a new file can change what
   they see), keeps the whole directory (``DIR/**``);
2. every other site starts as a base path (``reads: []``);
3. the model is rebuilt and every observed read the plan drops is charged to
   the sites of that unit whose directory holds the file; each such site
   widens to ``DIR/**/*.EXT`` for the extensions it is charged with when its
   source spells every one of them (``".qv"``/``"qv"``), else to ``DIR/**``;
4. repeat until nothing more is dropped.

A drop no directory-literal site can explain is printed to stderr: it needs a
hand-written entry.  The draft is reviewed before it is committed.
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


def build(crates, files, walkers, expansions):
    return scope_mod.Model(crates, files, walkers=walkers, expansions=expansions)


def main() -> int:
    census = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["units"]
    full = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))["units"]
    table = scope_mod.load_table()
    hand = [entry for entry in table["walkers"] if not entry["why"].startswith(PREFIX)]
    crates = scope_mod.load_crates()
    files = scope_mod.tracked_files()
    model = build(crates, files, hand, table["expansions"])
    units_of = {}
    for name, targets in model.targets.items():
        for target in targets:
            units_of.setdefault(name, set()).add(scope_mod.unit_uid(name, target))
    sites = {}
    for site in model.undeclared:
        source = site["source"].rsplit(":", 1)[0]
        sites.setdefault((source, site["literal"]), set()).add(site["crate"])

    def site_units(source, crate_names):
        units = set()
        for crate in crate_names:
            narrowed = model.reader_units(crate, source)
            units |= set(narrowed) if narrowed is not None else units_of.get(crate, set())
        return units

    declared = {}
    for (source, literal), crate_names in sorted(sites.items()):
        units = site_units(source, crate_names)
        providers = any(model.crates[c].workspace == "providers" for c in crate_names)
        listed = sorted(
            {d for uid in units if uid in full for d in full[uid]["dirs"]
             if d == literal or d.startswith(literal + "/")}
            | {spec for uid in units if uid in full for spec in full[uid]["ls_files"]
               if any(arg in (".", literal) or literal.startswith(arg.rstrip("/") + "/")
                      for arg in json.loads(spec) if not arg.startswith("-"))})
        if providers:
            declared[(source, literal)] = ([f"{literal}/**"],
                                           f"{PREFIX}compiled into provider-workspace code, "
                                           "which the root census does not trace: the whole "
                                           "directory")
        elif listed:
            declared[(source, literal)] = ([f"{literal}/**"],
                                           f"{PREFIX}its units list the directory "
                                           f"({len(listed)} directory opens or ls-files "
                                           "pathspecs), so a new file can change what they "
                                           "see: the whole directory")
        else:
            declared[(source, literal)] = ([], f"{PREFIX}a base path: no unit compiling it "
                                               "opens a file under it that other literals, "
                                               "walkers or manifests do not already attribute")
    for _ in range(20):
        walkers = hand + [{"source": s, "literal": l, "reads": r, "why": w}
                          for (s, l), (r, w) in sorted(declared.items())]
        model = build(crates, files, walkers, table["expansions"])
        tracked = set(model.files)
        charged = {}
        unexplained = []
        cache = {}
        for uid, unit in census.items():
            for path in unit["read"]:
                if path not in tracked:
                    continue
                if path not in cache:
                    cache[path] = scope_mod.plan([path], model, lean=False)
                if scope_mod.selects(cache[path], uid):
                    continue
                hit = False
                for (source, literal), crate_names in sites.items():
                    if path.startswith(literal + "/") and uid in site_units(source, crate_names):
                        charged.setdefault((source, literal), set()).add(path)
                        hit = True
                if not hit:
                    unexplained.append(f"{uid} reads {path}")
        if not charged:
            break
        for key, paths in charged.items():
            source, literal = key
            text = (ROOT / source).read_text(encoding="utf-8", errors="replace")
            reads, _ = declared[key]
            extensions = sorted({posixpath.splitext(p)[1].lstrip(".") for p in paths}
                                | {r.rsplit("*.", 1)[1] for r in reads if "*." in r})
            if all(ext and re.search(r'"\.?%s"' % re.escape(ext), text) for ext in extensions) \
                    and f"{literal}/**" not in reads:
                declared[key] = ([f"{literal}/**/*.{ext}" for ext in extensions],
                                 f"{PREFIX}its units open files under it, all "
                                 f"*.{', *.'.join(extensions)}, the extension(s) its source "
                                 "spells")
            else:
                declared[key] = ([f"{literal}/**"],
                                 f"{PREFIX}its units open files under it and its source "
                                 "spells no filter covering them: the whole directory")
    for line in sorted(set(unexplained)):
        print(f"unexplained: {line}", file=sys.stderr)
    entries = hand + [{"source": s, "literal": l, "reads": r, "why": w}
                      for (s, l), (r, w) in sorted(declared.items())]
    print(json.dumps({"schema": scope_mod.WALKERS_SCHEMA, "note": table.get("note"),
                      "walkers": entries, "expansions": table["expansions"]}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
