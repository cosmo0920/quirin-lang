import hashlib,json,re,time
from pathlib import Path
p=Path('private/audit-ir-retained-products');source=(p/'monkey-preprocessed.txt').read_bytes();descriptor=(p/'monkey-descriptor.bin').read_bytes()
m=re.search(rb'MONKEY_BUILT\[\]\s*=\s*"([^"]+)"\s*" "\s*"(\d\d:\d\d:\d\d)"',source)
assert m, 'timestamp declaration absent'
prefix=hashlib.sha256(descriptor+source[:m.start(2)]);suffix=source[m.end(2):]
targets={'0fa43e89ca95ed1868fe9772ff905e626b45517039ed30047bb258c991b0946f':'before','aa40f55a6a8bab1aaad049bde8a7ccbbcb9555322e605cc6dfb86bdb966edcae':'after'};matches=[]
start=time.monotonic()
for seconds in range(86400):
 stamp=f'{seconds//3600:02}:{seconds//60%60:02}:{seconds%60:02}'
 h=prefix.copy();h.update(stamp.encode());h.update(suffix);digest=h.hexdigest()
 if digest in targets:matches.append({'run':targets[digest],'time_macro':stamp,'identity':digest})
 if len(matches)==2:break
record={'source':'lib/monkey/mk_bin/monkey.c','date_macro':m.group(1).decode(),'matches':matches,'descriptor_sha256':hashlib.sha256(descriptor).hexdigest(),'probe_source_sha256':hashlib.sha256(source).hexdigest(),'probe_preprocessing':(p/'timestamp-identity.log').read_text().strip(),'seconds':round(time.monotonic()-start,3),'explanation':'Reproduced both archived keys from the same current preprocessed bytes and exact native descriptor by changing only the eight __TIME__ bytes in MONKEY_BUILT. Both map to empty summary objects at both changed sidecar locations. No published artifact was rewritten or normalized.'}
(p/'timestamp-identity.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record,indent=2));assert len(matches)==2
