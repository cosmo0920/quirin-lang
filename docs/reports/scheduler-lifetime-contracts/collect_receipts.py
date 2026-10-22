#!/usr/bin/env python3
"""Collect finite focused receipts after execution, preserving earlier failures."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[3]
CARD=ROOT/'docs/reports/scheduler-lifetime-contracts'
RUNS={
'focused-1':{'rust':'cargo test -p qcore-contract --test scheduler_lifetime --no-fail-fast','lean-build':'lake build QCore.SchedulerLifetime','lean-audit':'lake env lean AuditSchedulerLifetime.lean'},
'focused-2':{'source':'cargo run -p qcore-contract --example scheduler_lifetime_source -- private/scheduler-lifetime-contracts/source','clippy':'cargo clippy -p qcore-contract --all-targets -- -D warnings','lean-build':'lake build QCore.SchedulerLifetime','lean-audit':'lake env lean AuditSchedulerLifetime.lean'},
'focused-3':{'fmt':'cargo fmt --all -- --check','lean-build':'lake build QCore.SchedulerLifetime','lean-audit':'lake env lean AuditSchedulerLifetime.lean'},
'focused-4':{'rust':'cargo test -p qcore-contract --test scheduler_lifetime --no-fail-fast','source':'cargo run -p qcore-contract --example scheduler_lifetime_source -- private/scheduler-lifetime-contracts/source','fmt':'cargo fmt --all -- --check','clippy':'cargo clippy -p qcore-contract --all-targets -- -D warnings','lean-build':'lake build QCore.SchedulerLifetime','lean-audit':'lake env lean AuditSchedulerLifetime.lean'}}
RUNS['focused-5']={key:RUNS['focused-4'][key] for key in ('rust','fmt','clippy')}
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
rows=[]
for run,commands in RUNS.items():
    directory=ROOT/'private/scheduler-lifetime-contracts'/run
    for stage,command in commands.items():
        log=directory/(stage+'.log');status=directory/(stage+'.exit')
        rows.append({'run':run,'command':command,'cwd':str(ROOT/'proof' if stage.startswith('lean-') else ROOT),
        'exit':int(status.read_text()),'private_log':str(log),'log_sha256':digest(log)})
final=ROOT/'private/scheduler-lifetime-contracts/focused-4'
assert all(row['exit']==0 for row in rows if row['run'] in ('focused-4','focused-5'))
assert final.joinpath('source-status.txt').read_text()==''
source_log=final.joinpath('source.log').read_text();source_result=json.loads(next(line for line in source_log.splitlines() if line.startswith('{')))
rust_log=final.joinpath('rust.log').read_text();assert '10 passed; 0 failed' in rust_log
lean_log=final.joinpath('lean-audit.log').read_text();assert lean_log.count("'QCore.SchedulerLifetime.")==12
assert 'sorryAx' not in lean_log and 'warning' not in lean_log
payload={'collection':'post-execution receipt collection; no retroactive execution attestation','measured_source':final.joinpath('source-head.txt').read_text().strip(),
'final_source_status_at_start':'clean','final_rust_passed':10,'final_lean_theorems':12,'source_result':source_result,
'parallel_lock':'/tmp/qcore-c-card-validation-20261022.lock',
'environment':{'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'},
'standing_gate':'delegated to root combined integration; this is focused evidence only',
'provider_gate':'inapplicable: no provider input/interface changed',
'nonzero_attempts':'focused-1 Lean build failed on reserved protected binder and line-broken structure literal; subsequent audit failed on missing object. Corrected without weakening theorem statements. Earlier report-inventory draft tool call also failed its complete-key assertion for four missing operation propositions; shipped producer includes all four.',
'control_addendum':'focused-5 adds params-absent/present × callback-empty/positive controls; implementation artifacts equal focused-4',
'rows':rows}
(CARD/'focused-receipts.json').write_text(json.dumps(payload,indent=2)+'\n')
