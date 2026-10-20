#!/usr/bin/env python3
"""Bind fresh preprocessing receipts to the current route's compile entry."""
import hashlib,json,shlex
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIMARY=Path('/home/cosmo/Documents/GitHub/qcore-c')
for route,db in [('system',PRIMARY/'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json'),('slice',PRIMARY/'private/w47-tail/attempt-1/prep/fluent-bit-clang-production/compile_commands.json'),('corpus',PRIMARY/'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json')]:
 raw=db.read_bytes();entries=[e for e in json.loads(raw) if e['file'].endswith('/lib/monkey/mk_bin/monkey.c')];assert len(entries)==1
 entry=entries[0];words=entry.get('arguments') or shlex.split(entry['command']);command=[words[0]];skip=False
 for word in words[1:]:
  if skip:skip=False;continue
  if word=='-c':continue
  if word=='-o':skip=True;continue
  command.append(word)
 command+=['-E','-P']
 receipt=json.loads((ROOT/'private/premise-decomposition-cycle1/time-source-1'/(route+'.json')).read_text())
 assert command==receipt['command'] and entry['directory']==receipt['cwd']
 assert receipt['exit']==0 and hashlib.sha256((ROOT/'private/premise-decomposition-cycle1/time-source-1'/(route+'.c')).read_bytes()).hexdigest()==receipt['source_sha256']
 record={'route':route,'compile_database':str(db),'resolved_database':str(db.resolve()),'compile_database_sha256':hashlib.sha256(raw).hexdigest(),'entry':entry,'c_source_sha256':hashlib.sha256(Path(entry['file']).read_bytes()).hexdigest(),'fresh_preprocess_command':receipt,'exact_command_and_cwd_match':True,'method':'Reconstruct current compile entry argv, remove only -c and the -o output pair, append -E -P. Compare exact argv/cwd with the independently fresh local preprocessing receipt. Full source/config summary hash replay separately binds the produced source bytes to actual measured summary identities.','pinned_loader':'pinned_route.rs:98 reads checkout/build/compile_commands.json for these production units; the explicit Monkey library database augments that prepass and contains no mk_bin/monkey.c.'}
 out=ROOT/'docs/reports/premise-decomposition-cycle1/measurements'/route;out.mkdir(exist_ok=True)
 (out/'compile-command-correspondence.json').write_text(json.dumps(record,indent=2)+'\n')
 print(route,'exact command/cwd/source match')
