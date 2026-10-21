#!/usr/bin/env python3
"""Exact no-pipe commands/exits/logs for this bounded card, including failures."""
import argparse,datetime,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
# Full standing/provider/release stages are run only under the shared advisory
# lock after the exact root merged-base prediction has been committed.
p=argparse.ArgumentParser();p.add_argument('stage',choices=['focused','examples','release','standing','provider','python']);p.add_argument('out',type=Path);a=p.parse_args();out=a.out.resolve();out.mkdir(parents=True,exist_ok=False)
primary=Path('/home/cosmo/Documents/GitHub/qcore-c')
steps={
'focused':[(ROOT,['cargo','test','-p','qcore-contract','--test','plugin_premise_decomposition','--no-fail-fast']),(ROOT,['cargo','test','-p','qcore-audit','--test','plugin_null_hypotheses','--no-fail-fast']),(ROOT/'proof',['lake','build','QCore.PluginPremiseDecomposition']),(ROOT/'proof',['lake','env','lean','AuditPluginPremiseDecomposition.lean'])],
'release':[(ROOT,['cargo','build','--release','-p','qcore-audit','--bin','quirin-audit'])],
'examples':[(ROOT,['cargo','build','-p','qcore-audit','--examples'])],
'standing':[(ROOT,c) for c in [['git','submodule','update','--init','--recursive'],['clang','--version'],['python3','scripts/run_test_binaries.py','--jobs','4'],['cargo','fmt','--all','--','--check'],['cargo','clippy','--all-targets','--','-D','warnings'],['cargo','run','-p','qcore-compiler','--example','reevaluation_probe'],['cargo','run','-p','qcore-compiler','--example','reevaluation_followup'],['python3','scripts/check_lean_audits.py','--jobs','4']]],
'provider':[(ROOT,c) for c in [['cargo','test','--manifest-path','providers/Cargo.toml','--workspace','--locked','--offline','--no-fail-fast'],['cargo','fmt','--manifest-path','providers/Cargo.toml','--all','--','--check'],['cargo','clippy','--manifest-path','providers/Cargo.toml','--workspace','--all-targets','--locked','--offline','--','-D','warnings'],['python3','-m','unittest','discover','-s','providers/tests'],['python3','providers/contracts.py','check'],['python3','providers/package.py','build','--output',str(out/'packages')]]]+[(ROOT,['python3','providers/package.py','verify',str(out/'packages'/name)]) for name in ['qcore.chunkio','qcore.libevent','qcore.librdkafka','qcore.cmetrics','qcore.ctraces']],
'python':[(ROOT,['python3','-m','unittest','discover','-s','scripts/tests'])]}
env=dict(os.environ);env.update(CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0')
receipts=[]
for i,(cwd,command) in enumerate(steps[a.stage]):
 start=datetime.datetime.now(datetime.timezone.utc).isoformat();log=out/f'{i:02}.log'
 with log.open('wb') as stream:code=subprocess.run(command,cwd=cwd,env=env,stdout=stream,stderr=subprocess.STDOUT).returncode
 row=dict(command=command,cwd=str(cwd),environment={k:env.get(k) for k in ['CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL','QCORE_AUDIT_BUILD_COMMIT','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTUP_TOOLCHAIN','ELAN_TOOLCHAIN','QCORE_CLANG']},start=start,end=datetime.datetime.now(datetime.timezone.utc).isoformat(),exit=code,log=str(log),head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip())
 receipts.append(row);(out/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n');(out/f'{i:02}.exit').write_text(str(code)+'\n');print(json.dumps(row),flush=True)

raise SystemExit(0 if all(row['exit']==0 for row in receipts) else 1)
