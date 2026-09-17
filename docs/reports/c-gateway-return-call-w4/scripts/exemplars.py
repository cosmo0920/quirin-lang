#!/usr/bin/env python3
"""Check the exemplar callees the brief named, against the measured buckets."""
import importlib.util
import json
import pathlib

spec = importlib.util.spec_from_file_location(
    "reach", str(pathlib.Path(__file__).with_name("reach.py"))
)
reach = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reach)

before = json.loads(
    (pathlib.Path(__file__).parent / "runs/before/resource-diagnostics.json").read_text()
)
status = {}
for u in before["units"]:
    for f in u.get("functions") or []:
        status[(u["file"], f["function"])] = f["status"]

leaves = reach.leaves("before")
ret = {k: v for k, v in leaves.items() if k[4] == reach.RET}
want = {
    "cJSON_Parse",
    "cJSON_Duplicate",
    "ctr_span_set_trace_id_with_cid",
    "destroy_data_point",
    "cmt_cat",
}
found = {}
for key, origin in ret.items():
    if key[1] in want:
        text = reach.source_line(origin)
        c = reach.callee(text)
        st = status.get((key[0], c)) if c else None
        found.setdefault(key[1], []).append((reach.classify(text), c, st, origin))

for name in sorted(want):
    rows = found.get(name)
    if not rows:
        print(f"{name}: NO return-expression leaf in BEFORE")
        continue
    for shape, c, st, o in rows:
        print(f"{name}: shape={shape} callee={c} callee_status={st} origin={o}")
