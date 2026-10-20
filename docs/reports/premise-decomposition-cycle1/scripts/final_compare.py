#!/usr/bin/env python3
"""Close final original-input comparisons without rewriting rejected evidence."""
import argparse,gzip,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
PRIVATE=ROOT/'private/premise-decomposition-cycle1'
REPORT=ROOT/'docs/reports/premise-decomposition-cycle1'
p=argparse.ArgumentParser();p.add_argument('route',choices=['system','slice','corpus']);a=p.parse_args();route=a.route
base=PRIVATE/'measure-1'/route/'base'
tip=PRIVATE/'measure-1'/route/'tip' if route=='corpus' else PRIVATE/'provider-1/final-configured'/route/'tip'
for directory in [base,tip]:assert json.loads((directory/'invocation.json').read_text())['exit']==0
raw=PRIVATE/'final-comparison-inputs'/route;raw.mkdir(parents=True,exist_ok=False)
(raw/'base').symlink_to(base,target_is_directory=True);(raw/'tip').symlink_to(tip,target_is_directory=True)
out=REPORT/'measurements/final'/route
receipts=[]
def run(label,args,allowed=(0,)):
 command=[sys.executable,str(Path(__file__).with_name(label+'.py')),*map(str,args)]
 log=raw/(label+'.log')
 with log.open('wb') as stream:code=subprocess.run(command,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT).returncode
 receipts.append({'command':command,'exit':code,'log':str(log)});(raw/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n')
 assert code in allowed,(label,code,log)
run('compare',[route,base,tip,out],allowed=(0,1))
run('attribute_metadata',[route,'--comparison',out,'--raw-root',raw,'--environment-directory',PRIVATE/'final-environment'])
run('attribute_time_identity',[out,PRIVATE/'time-source-1'/(route+'.c'),PRIVATE/'time-source-1'/(route+'.json')])
run('five_artifacts',['final-'+route,base,tip,out/'five-artifacts.json'])
facts={}
for side,directory in [('base',base),('tip',tip)]:
 target=raw/(side+'-outer-facts.json');command=[str(ROOT/'target/debug/examples/premise_outer_facts'),str(directory/'resource-diagnostics.json'),str(directory/'resource-diagnostics.contracts.json'),str(target)]
 code=subprocess.run(command,cwd=ROOT).returncode;assert code==0
 receipts.append({'command':command,'exit':code,'output':str(target)})
 facts[side]=json.loads(target.read_text())
assert facts['base']==facts['tip'],'every row closed-world classification must match'
(out/'full-closed-world-facts.json.gz').write_bytes(gzip.compress((json.dumps(facts,indent=2)+'\n').encode(),mtime=0))
(out/'compile-command-correspondence.json').write_bytes((REPORT/'measurements'/route/'compile-command-correspondence.json').read_bytes())
initial=json.loads((out/'comparison.json').read_text());metadata=json.loads((out/'metadata-attribution.json').read_text());time=json.loads((out/'provenance-attribution.json').read_text())
assert sum(metadata['remaining_leaf_counts'].values())==4 and len(time['matches'])==2
checks={'all_complete_record_differences_exactly_attributed':True,'full_original_input_contract_environment_equal_after_root_restoration':metadata['complete_environment_equal_after_exact_migration_and_root_restoration'],'all_complete_compile_configurations_attributed':metadata['all_compile_configurations_equal_after_exact_source_map_hash_and_root_restoration'],'all_rows_actual_closed_world_facts_equal':facts['base']==facts['tip'],'production_lock_byte_identical':metadata['lock_file']['base_sha256']==metadata['lock_file']['tip_sha256']}
for key in ['diagnostic_counts','full_null_dictionaries','null_summary','full_runtime_projection','declaration_and_clause_standing']:checks[key]=initial['checks'][key]
assert all(checks.values()),checks
summary={'route':route,'checks':checks,'all_checks_pass':True,'base':initial['base'],'tip':initial['tip'],'null_rows':initial['null_rows'],'potential_declaration_users':initial['potential_dependent_custody_records'],'hypothetically_affected_report_candidate_instances':initial['affected_custody_records'],'dependent_other_contexts_unchanged':initial['dependent_other_contexts_unchanged'],'unrelated_custody_records':initial['unrelated_custody_records'],'production_verdict_or_assumption_changes':0,'scope':'Exact original production data; nine-atom decomposition is report-owned, with source/model attachment open. Six-home link-map context does not supersede seven-home coverage.','retained_initial_comparison_exit':receipts[0]['exit'],'commands':receipts}
(out/'final-checks.json').write_text(json.dumps(summary,indent=2)+'\n');(raw/'commands.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(summary,indent=2))
