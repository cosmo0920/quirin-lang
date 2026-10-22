#!/usr/bin/env python3
"""Collect counts from completed source-gate logs without running checks again."""
import hashlib, json, re, sys
from pathlib import Path
root=Path(__file__).resolve().parents[3]
run=root/'private/contract-cycle4-integration'/sys.argv[1]
read=lambda p:json.loads(p.read_bytes())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
completion=read(run/'completion.json');commands=read(run/'commands.json')
assert completion['wrapper_exit']==0 and completion['source_stable']
assert completion['commands_sha256']==sha(run/'commands.json')
assert len(commands)==22 and all(c['exit']==0 for c in commands)
rust_log=(run/'rust.log').read_text()
summary_path=Path(re.search(r'^summary: (.+)$',rust_log,re.M)[1])
rust=read(summary_path);assert rust['ok'] and not rust['units_failed']
provider_log=(run/'provider-rust.log').read_text()
rows=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored',provider_log)
assert rows and all(row[0]=='ok' and int(row[2])==0 for row in rows)
provider=dict(summaries=len(rows),passed=sum(int(r[1]) for r in rows),failed=0,ignored=sum(int(r[3]) for r in rows))
lean=int(re.search(r'^Passed (\d+) Lean audits$',(run/'lean.log').read_text(),re.M)[1])
python=int(re.search(r'Ran (\d+) tests?',(run/'provider-python.log').read_text())[1])
record=dict(source=completion['source'],command_count=len(commands),all_commands_zero=True,source_stable=True,rust=dict(rust['totals'],units=rust['units_run'],summary=str(summary_path),summary_sha256=sha(summary_path)),lean_audits=lean,provider_rust=provider,provider_python=python,packages={c['name']:c['exit'] for c in commands if c['name'].startswith('package-')},private_logs=str(run),commands_sha256=sha(run/'commands.json'),log_sha256={name:sha(run/(name+'.log')) for name in ['rust','lean','provider-rust','provider-python']},remaining='Native acceptance and authoritative Python against the committed regenerated matrix are separate receipts.')
Path(__file__).with_name('source-gate-summary.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record,indent=2))
