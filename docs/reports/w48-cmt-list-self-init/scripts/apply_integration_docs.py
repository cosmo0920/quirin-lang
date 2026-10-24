from pathlib import Path
import json,re
root=Path.cwd();p=root/'private/w48-cmt-member-storage';r=root/'docs/reports/w48-cmt-list-self-init'
read=lambda f:json.loads(Path(f).read_text());write=lambda f,v:Path(f).write_text(json.dumps(v,indent=2)+'\n')
n=read(r/'native-comparison.json');g=read(r/'gate.json');assert n['unexplained_leaves']==0
old=read(root/'docs/reports/w48-bu-cmt-map-metric-get/integration-amendments.json')
new={'card':'W48-CMT-LIST-SELF-INIT','rule':'36a','measured_source':n['measured_tip'],'prediction_commit':'08b5546809e99124f384c0d91db7220c9c192520','base':n['base']}
new['increment_paragraph']=f"W48-CMT-LIST-SELF-INIT implements checked unit-local calls to existing self-link initializer leaves (gateway rule 36a), using positive complete source/ABI/field facts, actual CG11 leaf validation and a private empty-custody caller frame; no ordinary summary is published. Fresh BASE/planner precedes prediction 08b554680 and semantic implementation. Own paired corpus 4,834 remains 999 Safe / 3,826 Unknown / 9 provisional Unsafe; system 448 remains 99/345/4 and slice 404 remains 57 Safe / 342 Unknown / 4 Pending / 1 provisional Unsafe. All {n['null_dictionaries_compared']:,} NULL dictionaries and runtime projections remain unchanged after exact metadata attribution; {n['raw_differing_leaves_retained']:,} raw leaves are retained, {n['reading_receipts_replayed']} reading receipts replayed and {n['captured_environment_entries_validated']} captured environments validated, with zero unexplained semantic leaves. The 2,200-body planner is byte-identical; getter ceiling 11 is conditional and measured getter closure gain 0. The frozen gate passes 8,612 Rust tests (545 units / 558 summaries, 15 ignored), 369 Lean 4.35.0 audits and 374 Python tests, all steps exit 0; failed and interrupted attempts remain archived. The deepest admitted reader control uses 328 KiB below 1 MiB. Parent CMT-MAP-METRIC-GET-OWNERSHIP-CHAIN, new W48-CMT-TRACKED-INTERIOR-SELF-INIT and W48-EXACT-RETURN-WITNESS-PRECISION remain Open. Card-native figures precede later main integration and are not a combined-main observation. See docs/reports/w48-cmt-list-self-init/README.md."
new['status_paragraph']='W48-CMT-LIST-SELF-INIT (gateway rule 36a, measured 99457f141) adds a private checked call frame for self-link initializer leaves, with matching audited Lean and no ordinary custody summary. Its fresh isolated BASE/TIP retains corpus 999 Safe / 3,826 Unknown / 9 provisional Unsafe, system 99/345/4 and slice 57 Safe / 342 Unknown / 4 Pending / 1 provisional Unsafe; complete resource, NULL and runtime results agree after exact metadata reconstruction. Getter closure gain 0. Gate 8,612 Rust / 369 Lean / 374 Python passes. Tracked/interior effect transport and the full getter chain remain Open; these card-local figures do not replace later main observations.'
new['ledger']=[{'id':'CMT-MAP-METRIC-GET-OWNERSHIP-CHAIN','operation':'append','text':'W48-CMT-LIST-SELF-INIT adds only a checked initializer-leaf call inside a simple unit-local empty-custody caller frame, with no ordinary summary or production getter gain. CG11 literal self-stores were already supported. Tracked/interior caller effect transport remains Open under W48-CMT-TRACKED-INTERIOR-SELF-INIT; all other parent obligations remain Open.'},{'id':'W48-CMT-TRACKED-INTERIOR-SELF-INIT','operation':'add','title':'Transport checked self-initializer effects to tracked or embedded caller objects','text':'Open after W48-CMT-LIST-SELF-INIT. Its private empty-frame call admits no tracked owner, interior/member-address actual or member-place alias and exports no ordinary summary. DISCHARGE: generic checked shape/effect transport for an exact live caller-owned object or embedded member, preserving actual CG11 writes, derived/member alias invalidation, custody and list-membership obligations; matching audited Rust/Lean without placeholders or project axioms, complete source/ABI/full-body/all-exit checks, wrong-owner/layout/alias and callback/export refusal controls, and fresh BASE/planner then committed prediction then implementation with complete resource/NULL/runtime native attribution. Never erase initializer writes into a neutral summary.','source':'docs/reports/w48-cmt-list-self-init/README.md'}]
write(r/'integration-amendments.json',new)
mfile=root/'docs/reviews/r0_3/acceptance_matrix.json';m=read(mfile);rows=m['named_followups'];byid={x['id']:x for x in rows};applied=[]
for proposal in [old,new]:
 for a in proposal['ledger']:
  key=a['id'];text=a['text']
  if a['operation']=='add':
   if key in byid:assert text in byid[key]['why'],('existing tracker differs',key)
   else:
    row={'id':key,'title':a['title'],'why':text,'source':a['source']};rows.append(row);byid[key]=row
  elif a['operation']=='append':
   assert key in byid,key
   if text not in byid[key]['why']:byid[key]['why']+=' '+text
  else:raise AssertionError(a)
  applied.append({'card':proposal['card'],'item':key,'decision':'applied','verify':[{'file':str(mfile.relative_to(root)),'contains':text}]})
write(mfile,m)
# Preserve the live history pointer; create the next part before crossing 32 KiB.
index=root/'docs/development/increments/README.md';idx=index.read_text();match=re.search(r'Current record file: \[(wave-48-part(\d+)\.md)\]',idx);assert match
record=index.parent/match[1];paras=[new['increment_paragraph'],old['increment_paragraph']]
assert all(par not in record.read_text() for par in paras),'increment already applied'
if len(record.read_bytes())+sum(len(x.encode())+2 for x in paras)>32768:
 number=int(match[2])+1;assert number==12,'review new ordinal before creating another history part'
 nextname=f'wave-48-part{number}.md';target=index.parent/nextname;assert not target.exists()
 target.write_text(f'# Wave 48, twelfth record\n\n[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)\n\n> Counts and observations describe their recorded checkpoints. Current rules and headline status live in [AGENTS.md](../../../AGENTS.md) and [STATUS.md](../../STATUS.md). The preceding [eleventh record]({record.name}) retains its original stage claims.\n\n')
 idx=idx.replace(f'[{record.name}]({record.name})',f'[{nextname}]({nextname})',1)
 marker='| 48 | ';assert marker in idx;idx=idx.replace(marker,marker+f'[Wave 48, twelfth record]({nextname}) · ',1);index.write_text(idx);record=target
text=record.read_text();notice=text.index('\n\n',text.index('> Counts'))+2;record.write_text(text[:notice]+'\n\n'.join(paras)+'\n\n'+text[notice:])
for proposal in [old,new]:applied.append({'card':proposal['card'],'item':'increment','decision':'applied','note':'Current history part selected by live index; next part opened if needed before 32 KiB.','verify':[{'file':str(record.relative_to(root)),'contains':proposal['increment_paragraph']}]})
status=root/'docs/STATUS.md';text=status.read_text();prefix='# Current status and evidence boundaries\n\n';assert text.startswith(prefix)
addition='The two CMetrics reports below are isolated card measurements at their named tips, before later main integrations. They establish no combined-main native count; integration source validation is recorded separately.\n\n'+new['status_paragraph']+'\n\n'+old['status_paragraph']+'\n\n'
assert new['status_paragraph'] not in text;status.write_text(prefix+addition+text[len(prefix):])
for proposal in [old,new]:applied.append({'card':proposal['card'],'item':'status','decision':'applied','verify':[{'file':'docs/STATUS.md','contains':proposal['status_paragraph']}]})
(r/'integration').mkdir(exist_ok=True);write(r/'integration/reconciliation.json',{'cards':[old['card'],new['card']],'items':applied,'remaining':['Regenerate report index and acceptance matrix; validate combined committed source; publish integration gate; run committed-matrix Python acceptance; merge main and push configured origin.']})
print('Applied ledger/status/increments to',record)
