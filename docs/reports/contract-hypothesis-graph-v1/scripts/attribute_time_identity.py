#!/usr/bin/env python3
"""Account for empty Monkey lifetime-summary keys by exact __TIME__ replay.

The fallback descriptor is the unchanged loop_lifetime_source::read_unit
Rust Debug prefix for AnonymousText/Gnu17 with three empty option vectors.
No artifact is rewritten and no semantic field is excluded by this check.
"""
import argparse,datetime,gzip,hashlib,json,re
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('comparison',type=Path);p.add_argument('preprocessed',type=Path);p.add_argument('command',type=Path)
a=p.parse_args();d=json.loads(gzip.decompress((a.comparison/'erased-normalized-differences.json.gz').read_bytes()))
changes=d['contracts'];assert all(not v for k,v in d.items() if k!='contracts')
paths=[['callback_members','lifetime_protocols','evidence','summaries'],['callback_members','prepass','lifetime_protocols','summaries']]
assert len(changes)==4 and all(c['path'][:-1] in paths for c in changes)
targets={c['path'][-1] for c in changes};assert len(targets)==2
assert all((c.get('before'),c.get('after')) in [(None,{}),({},None)] for c in changes)
s=a.preprocessed.read_bytes();m=re.search(rb'(static const char MONKEY_BUILT\[\] = ")([^"]+)(" " " ")([^"]+)(";)',s);assert m
# As read_facts_in does after compiler_options rejects -fPIE, own command -E -P,
# then C11TranslationUnit { source, dialect:Gnu17, ..Default::default() }.
prefix=b'AnonymousText\0Gnu17\0[]\0[]\0[]\0'+s[:m.start(4)];suffix=s[m.end(4):]
found={};windows=[]
for label in ['before','after']:
 inv=json.loads((a.comparison/label/'invocation.json').read_text());begin=datetime.datetime.fromisoformat(inv['started']);end=datetime.datetime.fromisoformat(inv['ended']);windows.append([inv['started'],inv['ended']])
 for sec in range(int(begin.timestamp()),int(end.timestamp())+1):
  t=datetime.datetime.fromtimestamp(sec,datetime.timezone(datetime.timedelta(hours=9))).strftime('%H:%M:%S');h=hashlib.sha256(prefix+t.encode()+suffix).hexdigest()
  if h in targets: found[h]=t
assert set(found)==targets,(found,targets)
record={'meaning':'four retained raw leaf differences are one empty-summary identity remap, duplicated in evidence/prepass; changing only MONKEY_BUILT __TIME__ reproduces both complete source/config SHA256 keys','source':'lib/monkey/mk_bin/monkey.c','preprocessed_bytes':len(s),'preprocessed_sha256':hashlib.sha256(s).hexdigest(),'descriptor_prefix_hex':b'AnonymousText\0Gnu17\0[]\0[]\0[]\0'.hex(),'timezone':'Asia/Tokyo (+09:00)','date_literal':m.group(2).decode(),'matches':found,'measurement_windows_utc':windows,'preprocess':json.loads(a.command.read_text()),'semantic_summary_before':{},'semantic_summary_after':{},'all_other_erased_normalized_fields_equal':True}
(a.comparison/'provenance-attribution.json').write_text(json.dumps(record,indent=2)+'\n');(a.comparison/'time-source.c.gz').write_bytes(gzip.compress(s,mtime=0));print(json.dumps({'matches':found,'all_residuals_attributed':True}))
