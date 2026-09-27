#!/usr/bin/env python3
"""Build and measure a named integration stage once, preserving commands/exits.

Run only in the granted heavy-audit slot, never alongside the standing suite.
Usage: measure.py LABEL [--jobs N]
"""
import argparse
import datetime
import json
import pathlib
import subprocess

p=argparse.ArgumentParser(description=__doc__);p.add_argument('label');p.add_argument('--jobs',type=int,default=8);args=p.parse_args()
if not args.label.replace('-','').isalnum() or args.jobs<1:raise SystemExit('invalid stage/jobs')
root=pathlib.Path(__file__).resolve().parents[4]
out=root/'private/w46-next'/args.label;out.mkdir(parents=True,exist_ok=False)
revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
commands=[('build-corpus',['cargo','build','--release','-p','qcore-compiler','--example','cycle19_fluent_bit_libraries_audit']),('build-system',['cargo','build','--release','-p','qcore-audit','--bin','quirin-audit']),('corpus',['python3','docs/reports/w43-return-expression/scripts/run_audit.py',str(out/'corpus'),revision,'--jobs',str(args.jobs)]),('system',['python3','docs/reports/w46-contract-gap-expansion/scripts/run_system.py',str(out/'system'),'--jobs',str(min(args.jobs,4))])]
record={'revision':revision,'stage':args.label,'steps':[]}
for name,command in commands:
 start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with (out/(name+'.log')).open('wb') as stream:r=subprocess.run(command,cwd=root,stdout=stream,stderr=subprocess.STDOUT)
 (out/(name+'.exit')).write_text(str(r.returncode)+'\n');record['steps'].append(dict(name=name,command=command,exit=r.returncode,start=start,end=datetime.datetime.now(datetime.timezone.utc).isoformat()));(out/'steps.json').write_text(json.dumps(record,indent=2)+'\n');print(name,r.returncode,flush=True)
 if r.returncode:raise SystemExit(r.returncode)
