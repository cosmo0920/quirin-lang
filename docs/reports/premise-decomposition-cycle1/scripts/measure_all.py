#!/usr/bin/env python3
import json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIVATE=ROOT/'private/premise-decomposition-cycle1'
BASE='/home/cosmo/.codex/worktrees/w48-bottom-up-batch4/qcore-c/private/w48-bottom-up-batch4/base/quirin-audit'
tip=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
for route in ['system','slice','corpus']:
 for side,binary,commit,inputs in [('base',BASE,'35aaecd3e (analyzer-source equivalent to 2d8f6d34257db14031a2578ce14f941bee25fc19)',PRIVATE/'base-inputs'),('tip',ROOT/'target/release/quirin-audit',tip,ROOT)]:
  command=[sys.executable,str(Path(__file__).with_name('measure.py')),route,str(binary),commit,str(inputs),str(PRIVATE/'measure-1'/route/side)]
  code=subprocess.call(command,cwd=ROOT)
  print(json.dumps(dict(route=route,side=side,exit=code)),flush=True)
  if code:raise SystemExit(code)

for stage in ['standing','provider']:
 code=subprocess.call([sys.executable,str(Path(__file__).with_name('run_steps.py')),stage,str(PRIVATE/(stage+'-1'))],cwd=ROOT)
 print(json.dumps(dict(stage=stage,exit=code)),flush=True)
 if code:raise SystemExit(code)
