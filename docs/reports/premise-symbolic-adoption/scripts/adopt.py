"""Apply only the frozen original atom/group inventory, preserving every old byte."""
import argparse, hashlib, json, re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
REPORT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--apply',action='store_true');a=p.parse_args()
inv=json.loads((REPORT/'inventory-before.json').read_bytes());results=[];pin_changes=[];writes=[]
for file in inv['files']:
 target=ROOT/file['path'];before=(ROOT/file['before_asset']).read_text();assert hashlib.sha256(before.encode()).hexdigest()==file['sha256'];assert target.read_text()==before
 lines=[];added=[]
 for line in before.splitlines(keepends=True):
  lines.append(line)
  atom=re.fullmatch(r'( *)requires resource_hypothesis\(([a-z][a-z0-9_]*), "(?:[^"\\]|\\.)*"\);\n',line)
  group=re.fullmatch(r'( *)requires resource_premise_(?:all|decomposition)\(([a-z][a-z0-9_]*), ([a-z0-9_, ]+)\);\n',line)
  if atom:
   indent,name=atom.groups();new=f'{indent}requires resource_lifecycle_v1(introduce_{name}, |^+ hypothesis({name}));\n';assert len('introduce_'+name)<=128;lines.append(new);added.append(new)
  if group:
   indent,name,children=group.groups();children=children.split(', ');tree=children[0]
   for child in children[1:]:tree=f'({tree} |-> {child})'
   assert len('decompose_'+name)<=128;new=f'{indent}requires resource_premise_flow_v1(decompose_{name}, {tree});\n';lines.append(new);added.append(new)
 after=''.join(lines);assert ''.join(line for line in lines if line not in set(added))==before
 results.append({'path':file['path'],'before_sha256':file['sha256'],'after_sha256':hashlib.sha256(after.encode()).hexdigest(),'added_lines':len(added),'added_line_sha256':[hashlib.sha256(line.encode()).hexdigest() for line in added],'old_bytes_recovered_exactly':True})
 if a.apply:writes.append((target,after))
corpus=ROOT/'compiler/tests/qcore_contract_corpus.rs';text=corpus.read_text();boundary=text.index('\n];');table=text[:boundary];changes=[]
for file in inv['files']:
 module=file['path'].split('/contracts/',1)[1]
 for d in file['declarations']:
  pattern=r'(\(\s*"'+re.escape(module)+r'"\s*,\s*"'+re.escape(d['declaration'])+r'"\s*,\s*)(\d+)(\s*,?\s*\))';matches=list(re.finditer(pattern,table));assert len(matches)==1,(module,d['declaration'])
  m=matches[0];before=int(m[2]);after=before+len(d['atoms'])+len(d['groups']);changes.append((m.start(2),m.end(2),str(after)));pin_changes.append({'module':module,'declaration':d['declaration'],'before':before,'after':after,'added':after-before})
for start,end,value in sorted(changes,reverse=True):table=table[:start]+value+table[end:]
text=table+text[boundary:];text=text.replace('const EXPECTED_CONTRACTS:', '// PREMISE-SYMBOLIC-ADOPTION: deliberate +1,348 metadata-only clause pins\n// across 169 declarations. Original hypotheses/interpreted clauses are retained.\nconst EXPECTED_CONTRACTS:',1)
assert sum(r['added'] for r in pin_changes)==1348 and sum(r['added_lines'] for r in results)==1348
if a.apply:
 for target,value in writes:target.write_text(value)
 corpus.write_text(text)
(REPORT/'adoption-records.json').write_text(json.dumps({'base':inv['base'],'applied':a.apply,'files':results,'clause_pins':pin_changes,'total_added_clauses':1348,'no_retirements':True},indent=2)+'\n')
print(json.dumps({'files':len(results),'declarations':len(pin_changes),'added_clauses':1348,'applied':a.apply}))
