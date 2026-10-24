"""Compact complete command receipts; report unsuccessful attempts unchanged."""
from pathlib import Path
import json,re,hashlib,argparse
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);args=parser.parse_args();P=args.evidence_root.resolve()
sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
result={'gates':[],'scope':'Each attempt owns its source, command, counts, exit and private logs. No failed attempt is relabeled successful.'}
for name in ['gate-1','gate-2-unconfined']:
 d=P/name;assert (d/'exit').exists(),name+' is incomplete'
 cmds=json.loads((d/'commands.json').read_text());assert len(cmds)==19
 rust=json.loads((d/'rust-units/summary.json').read_text());provider=(d/'provider-rust.log').read_text()
 summaries=re.findall(r'^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',provider,re.M)
 pycounts={}
 for log in ['python','provider-python']:
  text=(d/(log+'.log')).read_text();m=re.search(r'^Ran (\d+) tests? in ([\d.]+)s',text,re.M);assert m
  pycounts[log]={'tests':int(m[1]),'seconds':float(m[2])}
 lean=(d/'lean.log').read_text()
 result['gates'].append({'name':name,'exit':int((d/'exit').read_text()),'environment':json.loads((d/'environment.json').read_text()),'commands':cmds,'root_rust':{'units_run':rust['units_run'],'units_failed':rust['units_failed'],'totals':rust['totals'],'problems':rust['problems'],'summary_path':str(d/'rust-units/summary.json'),'summary_sha256':sha(d/'rust-units/summary.json')},'provider_rust':{'result_lines':len(summaries),'passed':sum(int(x[0]) for x in summaries),'failed':sum(int(x[1]) for x in summaries),'ignored':sum(int(x[2]) for x in summaries),'count_method':'Sum the top-level cargo test-result lines; unit/integration/doc-test result lines all retained in log.'},'python':pycounts,'lean_log_tail':lean[-1200:],'command_receipt_sha256':sha(d/'commands.json')})
(P/'validation-receipt.json').write_text(json.dumps(result,indent=2)+'\n')
print([(g['name'],g['exit'],g['root_rust']['totals']) for g in result['gates']])
