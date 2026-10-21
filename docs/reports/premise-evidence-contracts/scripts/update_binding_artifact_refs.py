#!/usr/bin/env python3
"""Refresh only explicit model/audit reference hashes after a recorded repair."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[4];REPORT=Path(__file__).resolve().parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()
p=REPORT/'binding-manifest.json';old=sha(p.read_bytes());m=json.loads(p.read_text())
for binding in m['atoms'].values():
 for field in ['model','audit']:binding[field]['sha256']=sha((ROOT/binding[field]['path']).read_bytes())
wire=json.dumps(m,ensure_ascii=False,separators=(',',':')).encode();p.write_bytes(wire);new=sha(wire)
(REPORT/'binding-manifest-readable.json').write_text(json.dumps(m,indent=2)+'\n')
source=ROOT/'providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv';text=source.read_text();assert old in text;source.write_text(text.replace(old,new))
a=REPORT/'binding-address.json';d=json.loads(a.read_text());d.update(manifest_sha256=new,model_sha256=next(iter(m['atoms'].values()))['model']['sha256'],audit_sha256=next(iter(m['atoms'].values()))['audit']['sha256']);a.write_text(json.dumps(d,indent=2)+'\n')
print(json.dumps({'old_manifest_sha256':old,'new_manifest_sha256':new,'graph_sha256_unchanged':m['graph_sha256'],'source_binding':'open'}))
