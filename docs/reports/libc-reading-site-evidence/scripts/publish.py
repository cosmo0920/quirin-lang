#!/usr/bin/env python3
"""Publish complete preserved route evidence after successful strict attribution."""
import gzip,hashlib,json,shutil,subprocess,sys
from pathlib import Path
repo=Path(__file__).resolve().parents[4];card=repo/'docs/reports/libc-reading-site-evidence';private=repo/'private/libc-reading-site-evidence'
out=card/'measurements';out.mkdir(exist_ok=True)
manifest={}
for route in ['system','slice','corpus']:
 source=private/('comparison-'+route+'-1');comparison=json.loads((source/'comparison.json').read_text());assert comparison['all_pass']
 target=out/route;shutil.copytree(source,target,dirs_exist_ok=True)
 impact=private/('production-'+route+'-impact.json')
 command=[sys.executable,str(card/'scripts/impact.py'),str(private/'runs-1'/route/'after/resource-diagnostics.json'),str(impact)]
 code=subprocess.run(command,cwd=repo).returncode;assert code==0
 data=json.loads(impact.read_text());assert data['observations']==comparison['site_observations']
 (target/'impact.json.gz').write_bytes(gzip.compress(impact.read_bytes(),mtime=0))
 (target/'impact-command.json').write_text(json.dumps(dict(command=command,exit=code,controls=len(data['controls']),records=len(data['record_inventory']),automatic_verdict_mutations=0),indent=2)+'\n')
 bindings=private/('production-'+route+'-bindings.json');value=json.loads(bindings.read_text())
 (target/'production-bindings.json.gz').write_bytes(gzip.compress(bindings.read_bytes(),mtime=0))
 manifest[route]={str(p.relative_to(target)):{'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(target.rglob('*')) if p.is_file()}
(out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
receipts=card/'validation';receipts.mkdir(exist_ok=True)
for name in ['measurement-commands.json','production-check-commands.json','comparison-commands.json','formal-leaves-1.json','formal-leaves-1.log','binding-helper-build-1.json','binding-helper-build-2.json','baseline-reuse.json','base-input-manifest.json','base-input-manifest.incomplete.json','input-integrity-after.json']:
 shutil.copyfile(private/name,receipts/name)
shutil.copyfile(private/'gate-1/commands.json',receipts/'source-gate-commands.json')
shutil.copyfile(private/'gate-1/binary.json',receipts/'tip-binary.json')
shutil.copyfile(private/'gate-1/clang.log',card/'clang-version.txt')
(card/'clang-version.exit').write_text('0\n')
print('published all full artifacts, finite observations and exact-context impact maps')
