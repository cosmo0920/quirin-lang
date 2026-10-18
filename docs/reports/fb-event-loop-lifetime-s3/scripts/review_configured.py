#!/usr/bin/env python3
"""Check every configured non-function field and unchanged custody row."""
import copy
import difflib
import gzip
import json
from pathlib import Path
import sys

route = sys.argv[1]
assert route in ("system", "slice")
out = Path(__file__).resolve().parents[1] / "artifacts" / route


def load(side, stem):
    return json.loads(gzip.decompress((out / side / f"{stem}.json.gz").read_bytes()))


def norm(value, identities=None):
    if isinstance(value, str):
        for root in ("/tmp/qcore-lexical-typed-precedence", "/tmp/qcore-fb-lifetime-s3",
                     "/home/cosmo/Documents/GitHub/qcore-c"):
            value = value.replace(root, "$CHECKOUT")
        return (identities or {}).get(value, value)
    if isinstance(value, list):
        return [norm(item, identities) for item in value]
    if isinstance(value, dict):
        result = {norm(key, identities): norm(item, identities) for key, item in value.items()}
        assert len(result) == len(value)
        return result
    return value


old, new = load("before", "diagnostics"), load("after", "diagnostics")
assert len(old["units"]) == len(new["units"])
adjusted = copy.deepcopy(old)
identities = []
body_evidence = []
for i, (a, b) in enumerate(zip(old["units"], new["units"])):
    assert (a["library"], a["file"]) == (b["library"], b["file"])
    ac, bc = a["compile_configuration"], b["compile_configuration"]
    mapping = {ac["dependencies"]["resource_contract_sources"]: bc["dependencies"]["resource_contract_sources"],
               ac["inventory_sha256"]: bc["inventory_sha256"]}
    assert norm(ac, mapping) == norm(bc), "unexplained configuration field"
    assert norm(a["functions"]) == norm(b["functions"]), "configured custody row changed beyond checkout paths"
    identities.append({"unit_index": i, "file": a["file"], "digest_mapping": mapping})
    adjusted["units"][i]["compile_configuration"] = bc
    if a.get("contract_body_evidence") != b.get("contract_body_evidence"):
        expected = copy.deepcopy(a["contract_body_evidence"])
        host = "flb_input_set_collector_time"
        clause = "quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl"
        assert clause not in expected["clauses"][host]
        expected["clauses"][host][clause] = []
        assert expected == b["contract_body_evidence"], "unexplained body-evidence change"
        body_evidence.append({"unit_index": i, "file": a["file"], "host": host,
                              "clause": clause, "evidence": [],
                              "meaning": "new protocol clause is explicitly uncovered; ordinary W4 evidence does not validate it"})
        adjusted["units"][i]["contract_body_evidence"] = b["contract_body_evidence"]
assert norm(adjusted) == norm(new), "unexplained non-function source/scope/diagnostic change"
ca, cb = norm(load("before", "contracts")), norm(load("after", "contracts"))
assert ca.keys() == cb.keys()
assert ca["contracts"].keys() == cb["contracts"].keys()
texts = {key: list(difflib.unified_diff(ca["contracts"][key].splitlines(), cb["contracts"][key].splitlines(),
                                     fromfile="before", tofile="after"))
         for key in ca["contracts"] if ca["contracts"][key] != cb["contracts"][key]}
assert len(texts) == 1 and next(iter(texts)).endswith("fluent_bit/input_collector_registration.qv")
changed = [key for key in ca if ca[key] != cb[key]]
assert set(changed) <= {"callback_members", "contract_dependencies", "contract_revision", "contracts"}
for stem in ("chunk-contracts", "source-effects"):
    assert (out / "before" / f"{stem}.json.gz").read_bytes() == (out / "after" / f"{stem}.json.gz").read_bytes()
result = {"meaning": "Raw differences remain archived. All configured custody rows agree after checkout-path normalization; no changed compile/source/scope field is discarded.",
          "route": route, "units": len(old["units"]),
          "configuration_rows": sum(len(item["functions"]) for item in old["units"]),
          "configuration_identity_changes": identities,
          "new_uncovered_protocol_clauses": body_evidence,
          "changed_contract_fields": changed, "changed_module_texts": texts,
          "all_other_fields_unchanged": True,
          "byte_identical_artifacts": ["chunk-contracts", "source-effects"],
          "source_artifact": "not emitted on either configured route"}
(out / "configured-review.json").write_text(json.dumps(result, indent=2) + "\n")
print(route, result["configuration_rows"], "custody rows unchanged; all other fields accounted")
