#!/usr/bin/env python3
"""Summarize compare_outputs.py results: one line per artifact.

Usage: summarize_identity.py COMPARE.json...
"""
import json
import sys

for path in sys.argv[1:]:
    data = json.load(open(path))
    print(f"== {path}: ok={data['ok']}")
    for name, value in data["results"].items():
        if isinstance(value, dict) and "different" in value:
            print(f"  {name}: different in {value['different']} leaves; first {value['first'][:3]}")
        elif isinstance(value, dict) and "identical_after_masking" in value:
            differing = [line["line"] for line in value.get("differing_lines", [])]
            print(f"  {name}: identical_after_masking={value['identical_after_masking']} "
                  f"masked/differing lines {differing} inserted {value.get('inserted_lines')}")
        else:
            print(f"  {name}: {value}")
