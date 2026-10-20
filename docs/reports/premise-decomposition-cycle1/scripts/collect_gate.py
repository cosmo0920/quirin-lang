#!/usr/bin/env python3
"""Publish only completed actual gates, with complete commands and private logs."""
import json,re,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIVATE=ROOT/'private/premise-decomposition-cycle1'
REPORT=ROOT/'docs/reports/premise-decomposition-cycle1'
load=lambda p:json.loads(p.read_text())
standing=load(PRIVATE/'standing-2/commands.json');configured=load(PRIVATE/'configured-final/commands.json');python=load(PRIVATE/'python-first/commands.json')
assert len(standing)==8 and len(configured)==2 and len(python)==1
assert all(row['exit']==0 for rows in [standing,configured,python] for row in rows)
root_log=Path(standing[2]['log']).read_text();matches=re.findall(r'^summary: (.+)$',root_log,re.M);assert len(matches)==1
summary_path=Path(matches[0]);summary_path=summary_path if summary_path.is_absolute() else ROOT/summary_path
summary=load(summary_path);assert summary['ok'] and summary['totals']['failed']==0 and not summary['problems']
lean=re.findall(r'^Passed (\d+) Lean audits$',Path(standing[7]['log']).read_text(),re.M);assert len(lean)==1
provider_changes=subprocess.check_output(['git','diff','--name-only','2d8f6d342','--','providers','contract','crates/qcore-schemas'],cwd=ROOT,text=True).strip()
assert not provider_changes, provider_changes
provider_passed=133 # Borrowed exact-base provider boundary only; no fresh provider gate applies.
def unittest_count(path):
 text=Path(path).read_text();matches=re.findall(r'^Ran (\d+) tests? in ',text,re.M);assert len(matches)==1 and re.search(r'^OK(?: \(.*\))?$',text,re.M);return int(matches[0])
python_count=unittest_count(python[0]['log'])
comparisons={route:load(REPORT/'measurements/final'/route/'final-checks.json') for route in ['corpus','system','slice']};assert all(item['all_checks_pass'] for item in comparisons.values())
clang=Path(standing[1]['log']).read_bytes();assert clang; (REPORT/'clang-version.txt').write_bytes(clang);(REPORT/'clang-version.exit').write_text(str(standing[1]['exit'])+'\n')
clang_banner=clang.decode().splitlines()[0]
counts={'workspace':{'units':summary['units_run'],**summary['totals'],'summary':str(summary_path)},'lean_audits':int(lean[0]),'provider_gate':{'applicability':'inapplicable: final provider contracts, SDK and schema byte-identical to exact base','borrowed_base_provider_tests':133,'source':'docs/reports/w48-bottom-up-batch3/integration/gate.json at exact base2d8f6d342','fresh_provider_rust_or_python_or_package_claim':False},'root_python_after_committed_derived_matrix':python_count}
(REPORT/'measured-counts.json').write_text(json.dumps(counts,indent=2)+'\n')
(REPORT/'command-receipts.json').write_text(json.dumps({'standing':standing,'final_configured_tips':configured,'python_after_committed_derived_matrix':python,'root_runner_summary':str(summary_path),'boundary':'Actual isolated worktree commands, exits and private log paths. Final source-census pins bbe9c4c76; model/helpers unchanged since70ecbfd95. Final production provider files/count pin byte-identical to exact base. Provider_tests133 only is carried from exact-base integration evidence because final provider gate is inapplicable; not a fresh measurement. Final gate-backed matrix confirmation will be recorded separately.'},indent=2)+'\n')
gate={'note':f'PREMISE-DECOMPOSITION-CYCLE1 on exact base2d8f6d342: report-owned nine opaque atoms/three conditional theorems; no production contract, classifier or semantic reader change. Root runner{summary["units_run"]}units/{summary["totals"]["summaries"]}summaries/{summary["totals"]["passed"]}passed/{summary["totals"]["ignored"]}ignored/0failed; {lean[0]}Lean audits. Conditional provider gate inapplicable: final provider contracts/SDK/schema unchanged; provider_tests133 carries exact-base integration evidence only, with no fresh provider/package claim. RootPython{python_count}passed after committed derived matrix; final gate-backed confirmation documented separately. Six fresh original-input audits compare equal under same six-home link-map context; scoped corpus966Safe/3859Unknown/9provisionalUnsafe, system99/345/4, slice53/346/4Pending/1provisionalUnsafe. Complete null/runtime dictionaries and every-row closed-world classification equal; roots/environment/inventory/timestamp metadata exactly reconstructed. Historical seven-home967 coverage remains separately dated. Candidate production companion rejected by actual classifier control; all attempts retained. Actual local Clang recording accompanies this file.','workspace_tests':summary['totals']['passed'],'provider_tests':provider_passed,'python_evidence_validator_tests':python_count,'reevaluation_probe_exit':standing[5]['exit'],'reevaluation_followup_exit':standing[6]['exit'],'cargo_fmt':'clean','cargo_clippy':'clean','standing_gate':'complete','lean_audits':int(lean[0]),'clang':'present: '+clang_banner}
(REPORT/'gate.json').write_text(json.dumps(gate,indent=2)+'\n');print(json.dumps(counts,indent=2))
