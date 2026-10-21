#!/usr/bin/env python3
"""After all six runs, resolve production bindings and attempt bounded held-source replays."""
import hashlib,json,os,subprocess
from pathlib import Path
repo=Path(__file__).resolve().parents[4];private=repo/'private/libc-reading-site-evidence'
records=json.loads((private/'measurement-commands.json').read_text());assert len(records)==6 and all(r['exit']==0 for r in records)
helper=private/'check-production-bindings-2';deps=repo/'target/release/deps';source=repo/'docs/reports/libc-reading-site-evidence/scripts/check_production_bindings.rs'
command=['rustc','--edition=2024',str(source),'-L','dependency='+str(deps)]
libraries={}
for name in ['qcore_audit','qcore_c_gateway','qcore_contract','qcore_schemas','serde_json']:
 found=list(deps.glob('lib'+name+'-*.rlib'));assert len(found)==1
 libraries[name]={'path':str(found[0]),'sha256':hashlib.sha256(found[0].read_bytes()).hexdigest()};command+=['--extern',name+'='+str(found[0])]
command+=['-o',str(helper)]
with (private/'binding-helper-build-2.log').open('wb') as log:code=subprocess.run(command,cwd=repo,stdout=log,stderr=subprocess.STDOUT).returncode
(private/'binding-helper-build-2.json').write_text(json.dumps(dict(command=command,exit=code,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),libraries=libraries),indent=2)+'\n')
if code:raise SystemExit(code)
env=dict(os.environ,QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
checks=[]
for route in ['system','slice','corpus']:
 directory=private/'runs-1'/route/'after';inv=json.loads((directory/'invocation.json').read_text())
 # The actual retained invocation spells its checkout in argv, independently
 # of a receipt. Pinned mode's first source argument follows its job count.
 argv=inv['command'];checkout=argv[argv.index('--checkout')+1] if route!='corpus' else argv[argv.index('--jobs')+2]
 command=[str(helper),str(private/'base-inputs'),checkout,str(repo),str(directory/'resource-diagnostics.json'),str(private/('production-'+route+'-bindings.json'))]
 print(route,'production binding/source controls',flush=True)
 with (private/('production-'+route+'-bindings.log')).open('wb') as log:code=subprocess.run(command,cwd=repo,env=env,stdout=log,stderr=subprocess.STDOUT).returncode
 checks.append(dict(route=route,command=command,exit=code,environment={'QCORE_CLANG':env['QCORE_CLANG']}));(private/'production-check-commands.json').write_text(json.dumps(checks,indent=2)+'\n')
 if code:raise SystemExit(code)

comparison_commands=[]
for route in ['system','slice','corpus']:
    out=private/('comparison-'+route+'-1')
    if not out.exists():
        command=['python3',str(repo/'docs/reports/libc-reading-site-evidence/scripts/compare.py'),route,str(private/'runs-1'/route/'before'),str(private/'runs-1'/route/'after'),str(out)]
        with (private/('comparison-'+route+'-1.log')).open('wb') as log:code=subprocess.run(command,cwd=repo,stdout=log,stderr=subprocess.STDOUT).returncode
        comparison_commands.append(dict(command=command,exit=code))
    else:
        # The early strict slice comparison is reused only after verifying all
        # complete immutable artifact byte identities against both run paths.
        for side in ['before','after']:
            manifest=json.loads((out/side/'manifest.json').read_text())
            for name,identity in manifest.items():assert hashlib.sha256((private/'runs-1'/route/side/name).read_bytes()).hexdigest()==identity['sha256']
    comparison=json.loads((out/'comparison.json').read_text())
    if not comparison['all_pass']:
        command=['python3',str(repo/'docs/reports/libc-reading-site-evidence/scripts/attribute_time_identity.py'),route,str(out)]
        with (private/('comparison-'+route+'-time.log')).open('wb') as log:code=subprocess.run(command,cwd=repo,stdout=log,stderr=subprocess.STDOUT).returncode
        comparison_commands.append(dict(command=command,exit=code))
        (private/'comparison-commands.json').write_text(json.dumps(comparison_commands,indent=2)+'\n')
        if code:raise SystemExit(code)
(private/'comparison-commands.json').write_text(json.dumps(comparison_commands,indent=2)+'\n')
