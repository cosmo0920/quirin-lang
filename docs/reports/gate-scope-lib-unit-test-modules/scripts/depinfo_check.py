#!/usr/bin/env python3
"""Hold the cfg(test) classification to rustc's own record of what it read.

Card GATE-SCOPE-LIB-UNIT-TEST-MODULES.  Usage (repository root, after a full
``run_test_binaries.py`` build of this tree)::

    python3 docs/reports/gate-scope-lib-unit-test-modules/scripts/depinfo_check.py \
        BUILD_JSONL [BUILD_JSONL ...] > depinfo.json

``BUILD_JSONL`` is the runner's ``build.stdout.jsonl`` (cargo's
``compiler-artifact`` messages).  For every root-workspace library cargo tests,
rustc's dep-info file (``deps/<stem>.d`` beside the artifact) lists every
source file and every ``include_str!``/``include_bytes!``/``include!`` file it
read, once for the library as dependents link it (``profile.test`` false) and
once for its unit-test executable (true).  The check:

* **sound**: every tracked file the ordinary library reads selects the crate
  as a whole (``plan([path])`` reaches it whole), so no narrowing hides a
  change from the library's dependents;
* **attributed**: every tracked file only the unit-test build reads selects the
  library's unit (``PKG::lib::NAME``), as a whole crate or as that unit;
* counted, not required: how many of those test-only files the scoper narrows
  to the unit rather than to the whole crate.

Exit 1 if a file fails either required check.
"""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import gate_scope as scope_mod  # noqa: E402


def dep_files(path: Path) -> set[str]:
    """Repository-relative tracked-or-not paths a dep-info file names."""
    found = set()
    root = ROOT.resolve()
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.endswith(":") or line.startswith("#"):
            continue
        name = line[:-1].replace("\\ ", " ")
        try:
            relative = Path(name).resolve().relative_to(root).as_posix()
        except ValueError:
            continue  # a dependency's source, the toolchain, OUT_DIR
        found.add(relative)
    return found


def artifacts(paths):
    """{(package, kind+name, test)}: the dep-info file of each library build."""
    found = {}
    for jsonl in paths:
        for line in Path(jsonl).read_text(encoding="utf-8").splitlines():
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if message.get("reason") != "compiler-artifact":
                continue
            target = message["target"]
            if not set(target["kind"]) & {"lib", "rlib", "dylib", "proc-macro"}:
                continue
            package_path = message["package_id"]
            test = bool(message["profile"]["test"])
            filename = Path(message.get("executable") or message["filenames"][0])
            stem = filename.name.split(".", 1)[0]
            if not test and stem.startswith("lib"):
                stem = stem[3:]
            depinfo = filename.parent / f"{stem}.d"
            if not depinfo.exists():
                depinfo = filename.parent.parent / "deps" / f"{stem}.d"
            found[(package_path, "+".join(target["kind"]), target["name"], test)] = depinfo
    return found


def main() -> int:
    model = scope_mod.real_model()
    tracked = set(model.files)
    by_dir = {crate.directory: crate.name for crate in model.crates.values()}
    builds = artifacts(sys.argv[1:])
    libraries: dict[str, dict] = {}
    for (package_id, kind, name, test), depinfo in builds.items():
        # package_id: "path+file:///…/crates/x#0.1.0" or "x 0.1.0 (path+file://…)"
        crate = next((by_dir[d] for d in sorted(by_dir, key=len, reverse=True)
                      if f"/{d}#" in package_id or f"/{d})" in package_id
                      or package_id.split("#")[0].endswith("/" + d)), None)
        if crate is None or model.crates[crate].workspace != "root":
            continue
        entry = libraries.setdefault(crate, {"uid": f"{crate}::{kind}::{name}"})
        entry["test" if test else "plain"] = str(depinfo)
    cache: dict[str, dict] = {}

    def scope(path):
        if path not in cache:
            cache[path] = scope_mod.plan([path], model, lean=False)
        return cache[path]

    report = {"schema": "gate-scope-depinfo/1", "libraries": {}, "unsound": [],
              "unattributed": []}
    for crate, entry in sorted(libraries.items()):
        if "plain" not in entry or "test" not in entry:
            report["libraries"][crate] = {"skipped": "both builds not found", **entry}
            continue
        plain = {p for p in dep_files(Path(entry["plain"])) if p in tracked}
        test = {p for p in dep_files(Path(entry["test"])) if p in tracked}
        only_test = sorted(test - plain)
        narrowed = 0
        for path in sorted(plain):
            if crate not in scope(path)["affected"]:
                report["unsound"].append(f"{crate}: the ordinary library reads {path}, "
                                         "which does not select the crate whole")
        for path in only_test:
            planned = scope(path)
            if not scope_mod.selects(planned, entry["uid"]):
                report["unattributed"].append(f"{entry['uid']} reads {path}, "
                                              "which does not select it")
            elif crate not in planned["affected"]:
                narrowed += 1
        report["libraries"][crate] = {"uid": entry["uid"], "plain_reads": len(plain),
                                      "test_only_reads": len(only_test),
                                      "test_only_narrowed_to_unit": narrowed,
                                      "test_only_kept_crate_wide": len(only_test) - narrowed}
    print(json.dumps(report, indent=1))
    return 1 if report["unsound"] or report["unattributed"] else 0


if __name__ == "__main__":
    sys.exit(main())
