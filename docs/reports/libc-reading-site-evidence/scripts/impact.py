#!/usr/bin/env python3
"""Evidence-only exact-context consultation impact; never rewrites a verdict."""
import argparse, hashlib, json
from pathlib import Path

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('diagnostics',type=Path);p.add_argument('out',type=Path);a=p.parse_args()
    raw=a.diagnostics.read_bytes();d=json.loads(raw)
    rows=[dict(unit=u['file'],function=r['function'],original_status=r['status'],context=r['hypothesis_context'],dependencies=r['contract_dependencies'],receipts=r.get('reading_site_evidence',[])) for u in d['units'] for r in u['functions']]
    for index,row in enumerate(rows): row['record_id']=index
    observations=[(r,s) for r in rows for s in r['receipts']]
    controls=[]
    for owner,receipt in observations:
        declaration=receipt['observation']['declaration'];atoms=receipt['atom_sha256']
        for atom in atoms:
            potential=[r for r in rows if atom in r['dependencies']['hypothesis_dependencies'].get(declaration,[])]
            potential_ids={r['record_id'] for r in potential}
            controls.append(dict(receipt=receipt['id'],proposition=receipt['proposition_ids'][atom],premise='nested_call_admission',declaration=declaration,atom=atom,
                control='hypothetical independently established guarded counterexample; NOT a stale/refused receipt and NOT an observed C violation',
                selected_context=receipt['context'],affected_exact_context=[r['record_id'] for r in potential if r['context']==receipt['context']],dependent_other_contexts_unchanged=[r['record_id'] for r in potential if r['context']!=receipt['context']],unrelated=[r['record_id'] for r in rows if r['record_id'] not in potential_ids],
                automatic_verdict_mutations=0,universal_disposition='open'))
    a.out.write_text(json.dumps(dict(schema='libc-reading-impact.v1',diagnostics_sha256=hashlib.sha256(raw).hexdigest(),observations=len(observations),record_inventory=rows,controls=controls,context_rule='all nine coordinates equal; no cross-property, context, custody-to-null or runtime transport',rejection_rule='source/graph/context/manifest mismatch rejects evidence; does not refute an opaque universal proposition',authority='evidence-only impact simulation; existing actual consultation/summary closure only'),indent=2)+'\n')
if __name__=='__main__':main()
