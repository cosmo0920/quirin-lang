from pathlib import Path
import json, subprocess
p=Path('private/w48-cmt-tracked-interior');r=Path('docs/reports/w48-cmt-tracked-interior-self-init')
read=lambda f:json.loads(Path(f).read_bytes())
root=read(p/'latest-gate.json');provider=read(p/'provider-api-gate/gate.json');source=read(p/'api-source-validation.json')
assert root['exit']==provider['exit']==0
assert root['head']==provider['head']==source['api_validated_source']
assert read(p/'steps/api-source-validation-2.json')['exit']==0
assert read(p/'native-comparison.json')['unexplained_leaves']==0
for script in ['publish_report.py','write_report.py']:
 subprocess.run(['python3',str(p/script)],check=True)
f=r/'CMETRICS_APIS.md';s=f.read_text();pending='Validation is pending until the frozen initializer measurement finishes and the API source gate runs.'
assert pending in s
counts=read(p/'publication-counts.json')
s=s.replace(pending,f"Current-source validation at `{root['head']}` passes {counts['root_rust']['passed']:,} Rust tests, {counts['lean']} Lean audits and {counts['python']} Python tests; providers pass {counts['provider_rust']} Rust and {counts['provider_python']} Python tests, inventory checks and all five package build/verifications. [Full gate](standing-gate.json), [provider gate](provider-gate/gate.json), [source separation](api-source-validation.json). Trusted executable API adoption remains Open.")
f.write_text(s)
subprocess.run(['python3',str(p/'apply_current_docs.py')],check=True)
subprocess.run(['python3','scripts/build_docs_index.py'],check=True)
subprocess.run(['python3','docs/reports/remaining-d02/build_matrix.py','.','--gate',str(r/'gate.json')],check=True)
print('Publication generated. Inspect, commit, run post-committed-matrix Python, then seal source and release owned lock.')
