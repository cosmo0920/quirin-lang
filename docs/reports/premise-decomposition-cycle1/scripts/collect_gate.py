#!/usr/bin/env python3
"""Publish only completed actual gates, with complete commands and private logs."""
import json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIVATE=ROOT/'private/premise-decomposition-cycle1'
REPORT=ROOT/'docs/reports/premise-decomposition-cycle1'
load=lambda p:json.loads(p.read_text())
standing=load(PRIVATE/'standing-2/commands.json');provider=load(PRIVATE/'provider-1/commands.json');python=load(PRIVATE/'python-first/commands.json')
assert len(standing)==8 and len(provider)==13 and len(python)==1
assert all(row['exit']==0 for rows in [standing,provider,python] for row in rows)
root_log=Path(standing[2]['log']).read_text();matches=re.findall(r'^summary: (.+)$',root_log,re.M);assert len(matches)==1
summary_path=Path(matches[0]);summary_path=summary_path if summary_path.is_absolute() else ROOT/summary_path
summary=load(summary_path);assert summary['ok'] and summary['totals']['failed']==0 and not summary['problems']
lean=re.findall(r'^Passed (\d+) Lean audits$',Path(standing[7]['log']).read_text(),re.M);assert len(lean)==1
rust=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',Path(provider[0]['log']).read_text(),re.M);assert rust and all(status=='ok' and int(failed)==0 for status,passed,failed,ignored in rust)
provider_passed=sum(int(row[1]) for row in rust);provider_ignored=sum(int(row[3]) for row in rust)
def unittest_count(path):
 text=Path(path).read_text();matches=re.findall(r'^Ran (\d+) tests? in ',text,re.M);assert len(matches)==1 and re.search(r'^OK(?: \(.*\))?$',text,re.M);return int(matches[0])
provider_python=unittest_count(provider[3]['log']);python_count=unittest_count(python[0]['log'])
comparisons={route:load(REPORT/'measurements/final'/route/'final-checks.json') for route in ['corpus','system','slice']};assert all(item['all_checks_pass'] for item in comparisons.values())
clang=Path(standing[1]['log']).read_bytes();assert clang; (REPORT/'clang-version.txt').write_bytes(clang);(REPORT/'clang-version.exit').write_text(str(standing[1]['exit'])+'\n')
clang_banner=clang.decode().splitlines()[0]
counts={'workspace':{'units':summary['units_run'],**summary['totals'],'summary':str(summary_path)},'lean_audits':int(lean[0]),'provider_rust':{'passed':provider_passed,'ignored':provider_ignored,'failed':0},'provider_python':provider_python,'root_python_after_committed_derived_matrix':python_count}
(REPORT/'measured-counts.json').write_text(json.dumps(counts,indent=2)+'\n')
(REPORT/'command-receipts.json').write_text(json.dumps({'standing':standing,'provider_and_final_configured_tips':provider,'python_after_committed_derived_matrix':python,'root_runner_summary':str(summary_path),'boundary':'Actual isolated worktree commands, exits and private log paths. Source/model freeze70ecbfd95; later edits own reports/tracking. Final production provider files/count pin byte-identical to exact base. No borrowed gate figure. Final gate-backed matrix confirmation will be recorded separately.'},indent=2)+'\n')
gate={'note':f'PREMISE-DECOMPOSITION-CYCLE1 on exact base2d8f6d342: report-owned nine opaque atoms/three conditional theorems; no production contract, classifier or semantic reader change. Root runner{summary["units_run"]}units/{summary["totals"]["summaries"]}summaries/{summary["totals"]["passed"]}passed/{summary["totals"]["ignored"]}ignored/0failed; {lean[0]}Lean audits. Fresh provider{provider_passed}Rust/{provider_ignored}ignored, {provider_python}Python, all5packages rebuilt+verified. RootPython{python_count}passed after committed derived matrix; final gate-backed confirmation documented separately. Six fresh original-input audits compare equal under same six-home link-map context; scoped corpus966Safe/3859Unknown/9provisionalUnsafe, system99/345/4, slice53/346/4Pending/1provisionalUnsafe. Complete null/runtime dictionaries and every-row closed-world classification equal; roots/environment/inventory/timestamp metadata exactly reconstructed. Historical seven-home967 coverage remains separately dated. Candidate production companion rejected by actual classifier control; all attempts retained. Actual local Clang recording accompanies this file.','workspace_tests':summary['totals']['passed'],'provider_tests':provider_passed,'python_evidence_validator_tests':python_count,'reevaluation_probe_exit':standing[5]['exit'],'reevaluation_followup_exit':standing[6]['exit'],'cargo_fmt':'clean','cargo_clippy':'clean','standing_gate':'complete','lean_audits':int(lean[0]),'clang':'present: '+clang_banner}
(REPORT/'gate.json').write_text(json.dumps(gate,indent=2)+'\n');print(json.dumps(counts,indent=2))
