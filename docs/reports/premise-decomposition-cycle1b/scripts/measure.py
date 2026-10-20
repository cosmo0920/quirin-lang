#!/usr/bin/env python3
"""Use the frozen three-route measurement harness with an explicit contract root."""
import argparse
import importlib.util
import os
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--contract-root', required=True, type=Path)
    known, rest = parser.parse_known_args()
    target = ROOT / 'docs/reports/contract-hypothesis-graph-v1/scripts/measure.py'
    spec = importlib.util.spec_from_file_location('hypothesis_measurement', target)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.REPO = known.contract_root.resolve()
    os.environ["QCORE_PROFILES"] = str(module.REPO / "contract/profiles")
    sys.argv = [str(target), *rest]
    code = module.main()
    invocation = Path(rest[-1]) / "invocation.json"
    record = json.loads(invocation.read_text())
    record["contract_inputs_root"] = str(module.REPO)
    record["environment"]["QCORE_PROFILES"] = os.environ["QCORE_PROFILES"]
    invocation.write_text(json.dumps(record, indent=2)+"\n")
    return code

if __name__ == '__main__':
    raise SystemExit(main())
