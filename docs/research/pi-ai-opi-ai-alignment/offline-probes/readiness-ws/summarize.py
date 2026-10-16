import pathlib,json,hashlib,tomllib,zlib,os
p=pathlib.Path(__file__).resolve().parent;data=json.loads((p/'results.json').read_text());rows=data['results'];s=[]
for r in rows:
 frames=bytes.fromhex(r['wire'].get('clientFramesHex','')); items=[]
 while len(frames)>=2:
  a,b=frames[:2];n=b&127;off=2
  if n==126:
   if len(frames)<4:break
   n=int.from_bytes(frames[2:4],'big');off=4
  if n==127:
   if len(frames)<10:break
   n=int.from_bytes(frames[2:10],'big');off=10
  mask=frames[off:off+4] if b&128 else b'';off+=len(mask)
  if len(frames)<off+n:break
  payload=frames[off:off+n];payload=bytes(v^mask[i%4] for i,v in enumerate(payload)) if mask else payload;frames=frames[off+n:]
  text=None;error=None
  if a&15==1:
   try:text=(zlib.decompressobj(-8 if r['case']['id']=='window8' else -15).decompress(payload+b'\0\0\xff\xff') if a&64 else payload).decode()
   except Exception as e:error=repr(e)
  items.append(dict(opcode=a&15,rsv1=bool(a&64),masked=bool(b&128),bytes=n,text=text,error=error))
 r['wire']['decodedClientFrames']=items
 s.append(dict(runtime=r['runtime'],case=r['case']['id'],exit=r['exit'],opened=(r['result'] or {}).get('opened'),messages=len((r['result']or{}).get('messages',[])),error=(r['result']or{}).get('error'),outboundValid=any(i.get('text')==(p/'outbound.txt').read_text() for i in items),outboundCompressed=any(i['rsv1'] and i['opcode']==1 for i in items)))
assert len(rows)==80
for r in rows:
 if r['runtime']=='ratchet-owned-patched' and r['case']['id'] in ['plain','pmd','fragment-ping','context-takeover','mixed-compression-fragment','no-context','window8','window9','window15','headers125','header66000','connection-tokens']:
  assert r['exit']==0 and r['result']['opened'] and not r['result']['error'],r
  assert len(r['result']['messages'])==(2 if r['case'].get('multi') else 1)
  assert all(m==x['text'] for m,x in zip(r['result']['messages'],r['wire']['payloads']))
# The second context-takeover payload actually requires the prior dictionary.
context=next(r for r in rows if r['runtime']=='ratchet-owned-patched' and r['case']['id']=='context-takeover');payload=bytes.fromhex(context['wire']['payloads'][1]['compressedHex']);requires=None
try:zlib.decompressobj(-15).decompress(payload+b'\0\0\xff\xff');requires=False
except zlib.error:requires=True
assert requires
(p/'results.json').write_text(json.dumps(data,indent=2)+'\n');(p/'summary.json').write_text(json.dumps(dict(observations=len(rows),scope=data['scope'],secondContextPayloadRequiresDictionary=requires,rows=s),indent=2)+'\n')
lock=tomllib.loads((p/'rust/Cargo.lock').read_text());cache=pathlib.Path(r'C:/Users/Luiz/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f');dep=[]
for row in lock['package']:
 if row.get('source','').startswith('registry+'):
  archive=cache/f"{row['name']}-{row['version']}.crate";sha=hashlib.sha256(archive.read_bytes()).hexdigest();assert sha==row['checksum'];dep.append(dict(name=row['name'],version=row['version'],checksum=sha,bytes=archive.stat().st_size,official=f"https://static.crates.io/crates/{row['name']}/{row['name']}-{row['version']}.crate"))
(p/'dependency-pins.json').write_text(json.dumps(dict(registryPackages=len(dep),allCachedArchiveChecksumsVerified=True,packages=dep),indent=2)+'\n')
print(json.dumps(dict(observations=len(rows),registryPackages=len(dep),ownedValid12=True,contextActuallyRequiresDictionary=True),indent=2))
print('\n'.join(f"{r['runtime']} {r['case']} messages={r['messages']} exit={r['exit']} compressedOut={r['outboundCompressed']} outValid={r['outboundValid']}" for r in s if r['case'] in ['fragment-ping','context-takeover','window9','header66000','headers125']))
