#!/usr/bin/env python3
"""Verify every configured artifact field outside the retained row changes."""
from pathlib import Path
import copy
import gzip
import importlib.util
import json

REPORT = Path(__file__).resolve().parents[1]
REPO = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location("canonical_diff", REPO / "docs/reports/corpus-repin/diff_artifacts.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def load(directory, stem):
    return json.loads(gzip.decompress((directory / f"{stem}.json.gz").read_bytes()))


def normalize(value, identities=None):
    identities = identities or {}
    if isinstance(value, str):
        value = value.replace("/home/cosmo/Documents/GitHub/qcore-c", "$CHECKOUT").replace(str(REPO), "$CHECKOUT")
        return identities.get(value, value)
    if isinstance(value, list):
        return [normalize(v, identities) for v in value]
    if isinstance(value, dict):
        result = {normalize(k, identities): normalize(v, identities) for k, v in value.items()}
        assert len(result) == len(value), "normalization collision"
        return result
    return value


for route in ("system", "slice"):
    out = REPORT / "artifacts" / route
    before, after = out / "before", out / "after"
    ca, cb = load(before, "contracts"), load(after, "contracts")
    modules_a, modules_b = module.contract_sources({"sources": ca["contracts"]}), module.contract_sources({"sources": cb["contracts"]})
    a, ac = modules_a
    b, bc = modules_b
    assert not ac and not bc and a.keys() == b.keys()
    assert all(a[k]["text"] == b[k]["text"] for k in a)
    normalized_a, normalized_b = normalize(ca), normalize(cb)
    deps_a, deps_b = normalized_a.pop("contract_dependencies"), normalized_b.pop("contract_dependencies")
    assert normalized_a == normalized_b, f"{route}: unexplained contract-sidecar change"
    empty_artifacts = []
    for stem in ("chunk-contracts", "source-effects"):
        assert (before / f"{stem}.json.gz").read_bytes() == (after / f"{stem}.json.gz").read_bytes()
        empty_artifacts.append(stem)
    old, new = load(before, "diagnostics"), load(after, "diagnostics")
    assert len(old["units"]) == len(new["units"])
    adjusted = copy.deepcopy(old)
    configurations, review_paths = [], []
    for i, (x, y) in enumerate(zip(old["units"], new["units"])):
        assert (x["library"], x["file"]) == (y["library"], y["file"])
        xc, yc = x["compile_configuration"], y["compile_configuration"]
        identities = {xc["dependencies"]["resource_contract_sources"]: yc["dependencies"]["resource_contract_sources"],
                      xc["inventory_sha256"]: yc["inventory_sha256"]}
        assert normalize(xc, identities) == normalize(yc)
        if xc != yc:
            configurations.append({"unit_index": i, "file": x["file"], "identities": identities})
        assert len(x["functions"]) == len(y["functions"])
        for f, g in zip(x["functions"], y["functions"]):
            assert f["function"] == g["function"]
            if f.get("review_holds") != g.get("review_holds"):
                assert normalize(f["review_holds"]) == normalize(g["review_holds"])
                review_paths.append({"unit_index": i, "file": x["file"], "function": f["function"]})
        for key in ("compile_configuration", "functions", "reason", "diagnostic_counts"):
            if key in y:
                adjusted["units"][i][key] = y[key]
    adjusted["diagnostic_counts"] = new["diagnostic_counts"]
    assert normalize(adjusted) == normalize(new), f"{route}: unexplained non-row report field"
    result = {
        "meaning": "Comparison-only normalization; every raw artifact and full duplicate-preserving changed row remains archived",
        "units": len(old["units"]), "configuration_rows": sum(len(u["functions"]) for u in new["units"]),
        "configuration_identity_changes": configurations, "held_contract_module_path_only": review_paths,
        "all_other_nonfunction_source_contract_scope_fields_unchanged": True,
        "normalized_module_texts_identical": len(a),
        "raw_contract_keys": {"before": sorted(ca["contracts"]), "after": sorted(cb["contracts"])},
        "contract_dependencies_before": deps_a, "contract_dependencies_after": deps_b,
        "all_other_contract_sidecar_fields_unchanged_after_checkout_normalization": True,
        "byte_identical_artifacts": empty_artifacts, "source_artifact": "not emitted by current route; absence recorded on both sides",
    }
    (out / "configured-review.json").write_text(json.dumps(result, indent=2) + "\n")
    print(route, len(old["units"]), len(a), "module texts identical; all other non-row fields accounted for")
