#!/usr/bin/env python3
"""Attribute every package identity change between two `package.py build` outputs.

Usage: package_identity.py BEFORE_DIR AFTER_DIR [REPO_ROOT]

Both directories are `providers/package.py build --output` results taken in the
same checkout with the same `--target-dir` and toolchain, before and after this
card. For each of the five packages the script checks, from the bytes:

* declarations: the installed `contracts.json` and every `contracts/` asset are
  identical, the installed `verifier-plugin.json` differs in `package_digest`
  only, and the executable's embedded manifest-template and contract-inventory
  envelopes are byte-identical (what a host binds is unchanged);
* capability surface: for each shared Rust module, whether the executable's
  symbol table (`nm -C --defined-only`) defines any function of it, before
  (`qcore_provider_<name>::<module>::`) and after
  (`quirin_provider_shared::<module>::`). The sets must be equal, so linking
  the library gave no binary an evaluator it did not already compile;
* source spelling: which moved module's source path appears in the executable
  (panic locations), before (`../../common/<m>.rs`) and
  after (`quirin-provider-shared/src/<m>.rs`), again equal as module sets. Cargo
  spells a workspace member's sources relative to the workspace root, so the
  before spelling is `<crate>/src/../../common/<m>.rs`;
* shared contract data: no shared `.qcore` source text (first 96 bytes of each
  of the 24 modules) is embedded in any executable;
* code layout: the crate-local function symbols after normalising the crate
  path, listed where they differ (cross-crate inlining and codegen-unit
  partitioning change which helpers are separate symbols), and the counts of
  differing non-local symbols with the crate name normalised. These are
  disclosed, not asserted.

Prints one JSON document. Exit 0 only when every asserted check holds.
"""

import hashlib
import json
import pathlib
import re
import subprocess
import sys

PACKAGES = {
    "qcore.chunkio": "qcore_provider_chunkio",
    "qcore.cmetrics": "qcore_provider_cmetrics",
    "qcore.ctraces": "qcore_provider_ctraces",
    "qcore.libevent": "qcore_provider_libevent",
    "qcore.librdkafka": "qcore_provider_librdkafka",
}
ENVELOPES = [
    (b"<<<qcore.provider-template.v1:begin>>>", b"<<<qcore.provider-template.v1:end>>>"),
    (b"<<<qcore.provider-contracts.v1:begin>>>", b"<<<qcore.provider-contracts.v1:end>>>"),
]
MODULES = [
    "runtime", "source_effect", "counter_effect", "teardown_effect", "call_use_effect",
    "cleanup_replay_effect", "composition_effect", "reactor_callbacks",
]
HASH = re.compile(r"::h[0-9a-f]{16}$")


def envelope(data, begin, end):
    if data.count(begin) != 1 or data.count(end) != 1:
        return None
    return data[data.index(begin):data.index(end) + len(end)]


def symbols(path):
    out = subprocess.run(["nm", "-C", "--defined-only", str(path)], check=True,
                         capture_output=True, text=True).stdout
    names = set()
    for line in out.splitlines():
        parts = line.split(" ", 2)
        if len(parts) == 3:
            names.add(HASH.sub("", parts[2]))
    return names


def local(names, crate):
    prefixes = (crate + "::", "quirin_provider_shared::")
    result = set()
    for name in names:
        for prefix in prefixes:
            if name.startswith(prefix):
                result.add("<crate>::" + name[len(prefix):])
    return result


def tree(package):
    return {
        path.relative_to(package).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted((package / "contracts").rglob("*")) if path.is_file()
    }


def shared_contract_heads(repo):
    heads = {}
    for crate in sorted((repo / "providers").glob("quirin-provider-*")):
        for path in sorted(p for p in (crate / "contracts").rglob("*") if p.suffix in (".qcore", ".qv")):
            heads[path.relative_to(repo).as_posix()] = path.read_bytes()[:96]
    return heads


def main() -> int:
    before_root, after_root = map(pathlib.Path, sys.argv[1:3])
    repo = pathlib.Path(sys.argv[3]) if len(sys.argv) > 3 else pathlib.Path(__file__).resolve().parents[4]
    heads = shared_contract_heads(repo)
    result = {"before": str(before_root), "after": str(after_root),
              "shared_contract_modules_checked": len(heads), "packages": {}}
    ok = len(heads) == 24
    for name, crate in PACKAGES.items():
        before, after = before_root / name, after_root / name
        old = (before / "provider").read_bytes()
        new = (after / "provider").read_bytes()
        old_manifest = json.loads((before / "verifier-plugin.json").read_text())
        new_manifest = json.loads((after / "verifier-plugin.json").read_text())
        manifest_fields = sorted(
            field for field in set(old_manifest) | set(new_manifest)
            if old_manifest.get(field) != new_manifest.get(field)
        )
        envelopes_equal = all(
            envelope(old, b, e) is not None and envelope(old, b, e) == envelope(new, b, e)
            for b, e in ENVELOPES
        )
        contracts_equal = (before / "contracts.json").read_bytes() == (after / "contracts.json").read_bytes()
        tree_equal = tree(before) == tree(after)
        old_symbols, new_symbols = symbols(before / "provider"), symbols(after / "provider")
        old_local, new_local = local(old_symbols, crate), local(new_symbols, crate)
        modules = {
            "before": sorted(m for m in MODULES if any(s.startswith(f"<crate>::{m}::") for s in old_local)),
            "after": sorted(m for m in MODULES if any(s.startswith(f"<crate>::{m}::") for s in new_local)),
        }
        paths = {
            "before": sorted(m for m in MODULES if f"/../../common/{m}.rs".encode() in old),
            "after": sorted(
                m for m in MODULES if f"\x00quirin-provider-shared/src/{m}.rs".encode() in new
            ),
            "old_spelling_left_after": sorted(
                m for m in MODULES if f"common/{m}.rs".encode() in new
            ),
        }
        embedded_contracts = sorted(path for path, head in heads.items() if head in new)
        other_before = {
            s.replace(crate, "<crate>") for s in old_symbols
            if not s.startswith((crate + "::", "quirin_provider_shared::"))
        }
        other_after = {
            s.replace("quirin_provider_shared", "<crate>").replace(crate, "<crate>") for s in new_symbols
            if not s.startswith((crate + "::", "quirin_provider_shared::"))
        }
        entry = {
            "executable_sha256": {
                "before": hashlib.sha256(old).hexdigest(),
                "after": hashlib.sha256(new).hexdigest(),
            },
            "executable_bytes": {"before": len(old), "after": len(new)},
            "manifest_fields_changed": manifest_fields,
            "embedded_envelopes_identical": envelopes_equal,
            "contracts_json_identical": contracts_equal,
            "contract_tree_identical": tree_equal,
            "shared_modules_with_symbols": modules,
            "moved_module_source_paths": paths,
            "shared_contract_text_embedded": embedded_contracts,
            "crate_local_symbols": {
                "before": len(old_local),
                "after": len(new_local),
                "only_before": sorted(old_local - new_local),
                "only_after": sorted(new_local - old_local),
            },
            "other_symbols": {
                "before": len(other_before),
                "after": len(other_after),
                "only_before": len(other_before - other_after),
                "only_after": len(other_after - other_before),
                "examples": sorted(other_before ^ other_after)[:12],
            },
        }
        package_ok = (
            manifest_fields == ["package_digest"]
            and envelopes_equal and contracts_equal and tree_equal
            and modules["before"] == modules["after"]
            and paths["before"] == paths["after"] and not paths["old_spelling_left_after"]
            and not embedded_contracts
        )
        entry["asserted_checks_hold"] = package_ok
        ok &= package_ok
        result["packages"][name] = entry
    result["all_asserted_checks_hold"] = ok
    print(json.dumps(result, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
