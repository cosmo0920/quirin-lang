"""Retain and explain all after-only file symlinks without inventing before link records."""
from pathlib import Path
import argparse,json,hashlib,os,subprocess
ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--evidence-root',type=Path,default=Path(__file__).parent);ap.add_argument('--after',type=Path);a=ap.parse_args();P=a.evidence_root.resolve();root=P/'inputs-mirror-1';read=lambda f:json.loads(f.read_bytes());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest();before=read(P/'mirror-files-before.json')['files'];afterfile=a.after or P/'inputs-after.json';after=read(afterfile)
regular={str(f.relative_to(root)):sha(f)for f in sorted(root.rglob('*'))if f.is_file()and not f.is_symlink()};assert regular==before,'a regular file changed, appeared or disappeared'
fb=root/'c-gateway-wave-pin/fluent-bit-clang-source';binding=[]
for side in ['base-3','tip-1']:
 inv=read(P/side/'corpus/invocation.json');assert inv['source_identity']==inv['source_identity_after'];assert inv['source_identity']['tracked_delta_sha256']==hashlib.sha256(b'').hexdigest();binding.append(inv['source_identity'])
assert binding[0]==binding[1]
manifest=read(root/'manifest.json');records=[]
for change in after['mirror']['changes']:
 assert change['base']is None and change['after']is not None
 f=root/change['path'];assert f.is_symlink();target=f.resolve();rel=str(target.relative_to(root));assert rel in before and before[rel]==sha(target)==change['after']
 record={'path':change['path'],'link_text_now':os.readlink(f),'resolved_regular_target':rel,'target_sha256_before_and_after':before[rel],'raw_after_only_record':change,'temporal_link_text_claim':False}
 for part,tree in manifest['trees'].items():
  if change['path'].startswith(part+'/'):
   donor=Path(tree['original'])/change['path'][len(part)+1:];assert donor.is_symlink()and os.readlink(donor)==os.readlink(f);record['current_donor_link_text_equal']=True;break
 if f.is_relative_to(fb):
  path=str(f.relative_to(fb));stage=subprocess.check_output(['git','-C',str(fb),'ls-files','--stage','--',path],text=True).strip();parts=stage.split(None,3);assert parts[0]=='120000';blob=subprocess.check_output(['git','-C',str(fb),'show',binding[0]['head']+':'+path]);assert blob==os.readlink(f).encode();record['pinned_git_symlink_blob']=parts[1];record['temporal_link_text_claim']=True;record['basis']='Both successful corpus invocations held the same pinned HEAD with empty tracked binary diff before and after; current link text equals its pinned 120000 blob.'
 else:record['basis']='The original manifest did not inventory symlink texts. Current donor text and before/after target bytes agree; do not retroactively claim a before link-text capture.'
 records.append(record)
alllinks=[{'path':str(f.relative_to(root)),'text':os.readlink(f),'target_exists':f.exists(),'target_is_file':f.is_file()}for f in sorted(root.rglob('*'))if f.is_symlink()]
result={'exit':0,'before_inventory_regular_files':len(before),'after_regular_files':len(regular),'regular_files_exactly_unchanged':True,'raw_after_only_file_symlinks':len(records),'records':records,'current_all_symlink_texts':alllinks,'after_receipt':str(afterfile),'after_receipt_sha256':sha(afterfile),'pinned_corpus_source_binding':binding[0],'meaning':'All original regular-file entries, including Git metadata, unchanged. Thirteen after-only file-valued entries are symlinks absent from the regular-file before inventory. Their target contents are byte-bound before/after. Ten tracked link texts also have the pinned clean-Git binding; three build-library link texts have no original before snapshot. The raw additions are retained, not erased or relabeled as proven temporal link equality.'}
(P/'mirror-symlink-scope-review.json').write_text(json.dumps(result,indent=2)+'\n');print({'regular_unchanged':len(regular),'after_only_symlinks':len(records),'git_bound':sum(r['temporal_link_text_claim']for r in records),'no_before_link_text_snapshot':sum(not r['temporal_link_text_claim']for r in records)})
