from pathlib import Path
import subprocess,json,hashlib
p=Path('private/w48-cmt-tracked-interior');r=Path('docs/reports/w48-cmt-tracked-interior-self-init');read=lambda f:json.loads(Path(f).read_bytes());sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
g=read(p/'latest-gate.json');head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();changed=subprocess.check_output(['git','diff','--name-only',g['head'],head],text=True).splitlines();assert all(x.startswith('docs/') for x in changed),changed
step=read(p/'steps/post-matrix-python-1.json');assert step['exit']==0 and step['head']==head
out={'validated_source':g['head'],'publication_head_before_receipt':head,'production_source_contract_proof_changes_after_gate':[],'documentation_changes_after_gate':changed,'post_committed_matrix_python':step,'matrix_sha256':sha('docs/reviews/r0_3/acceptance_matrix.json'),'gate_reuse_basis':'Only listed documentation and report replay artifacts changed after the full current-source root/provider gates; native source is separately frozen and API runtime/view equality is separately retained.'}
(r/'source-confirmation.json').write_text(json.dumps(out,indent=2)+'\n')
(r/'artifact-inventory.json').write_text(json.dumps({str(f.relative_to(r)):{'sha256':sha(f),'bytes':f.stat().st_size} for f in sorted(r.rglob('*')) if f.is_file() and f.name!='artifact-inventory.json'},indent=2)+'\n')
print(json.dumps(out,indent=2))
