import pathlib,json,socket,threading,subprocess,hashlib,base64,zlib,os,time
P=pathlib.Path(__file__).parent
executableHashes={}
NODE=r'D:/nvm/v24.13.0/node.exe'; BUN=r'D:/nvm/node_global/node_modules/bun/bin/bun.exe';RUST=r'E:/opi/cargo-targets/windows-direct/debug/opi-readiness-ws-probe.exe'
assert hashlib.sha256(pathlib.Path(NODE).read_bytes()).hexdigest()=='d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11'
assert hashlib.sha256(pathlib.Path(BUN).read_bytes()).hexdigest()=='0025a8bc31f136745d102212ddb192a1876c721b71deeb43123d18375fcf030c'
def frame(b,op=1,rsv=False,fin=True):
 b=b.encode() if isinstance(b,str) else b;n=len(b);h=bytes([(128 if fin else 0)|(64 if rsv else 0)|op]);h+=bytes([n]) if n<126 else b'\x7e'+n.to_bytes(2,'big') if n<65536 else b'\x7f'+n.to_bytes(8,'big');return h+b
TEXT='message-canary:abcde-'*100
cases=[dict(id='plain'),dict(id='pmd',ext='permessage-deflate'),dict(id='fragment-ping',ext='permessage-deflate',fragment=True),dict(id='context-takeover',ext='permessage-deflate',multi=True),dict(id='mixed-compression-fragment',ext='permessage-deflate',multi=True,mixed=True),dict(id='no-context',ext='permessage-deflate; server_no_context_takeover; client_no_context_takeover',multi=True,reset=True),dict(id='window8',ext='permessage-deflate; server_max_window_bits=8; client_max_window_bits=8',bits=9,multi=True),dict(id='window9',ext='permessage-deflate; server_max_window_bits=9; client_max_window_bits=9',bits=9,multi=True),dict(id='window15',ext='permessage-deflate; server_max_window_bits=15; client_max_window_bits=15',multi=True),dict(id='window16-invalid',ext='permessage-deflate; server_max_window_bits=16'),dict(id='duplicate-param',ext='permessage-deflate; server_max_window_bits=15; server_max_window_bits=15'),dict(id='headers125',headers=122),dict(id='header66000',pad=66000),dict(id='invalid-accept',bad=True),dict(id='unknown-ext',ext='x-unrequested'),dict(id='unknown-subprotocol',sub=True),dict(id='rsv1-no-negotiation',illegal=True),dict(id='control-rsv1',ext='permessage-deflate',bad_control=True),dict(id='continuation-rsv1',ext='permessage-deflate',fragment=True,bad_continuation=True),dict(id='connection-tokens',connection='keep-alive, Upgrade')]
rows=[]
for runtime,exe,extra in [('node',NODE,[]),('bun',BUN,['--no-install']),('ratchet-native-patched',RUST,[]),('ratchet-owned-patched',RUST,['owned'])]:
 for case in cases:
  executableHashes[exe]=hashlib.sha256(pathlib.Path(exe).read_bytes()).hexdigest()
  listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen();port=listener.getsockname()[1];wire={};errors=[]
  def serve():
   try:
    conn,_=listener.accept();conn.settimeout(2);data=b''
    while b'\r\n\r\n' not in data:data+=conn.recv(8192)
    head,tail=data.split(b'\r\n\r\n',1);h={line.split(b':',1)[0].lower().decode():line.split(b':',1)[1].strip().decode() for line in head.split(b'\r\n')[1:]};wire['request']=head.decode();wire['offer']=h.get('sec-websocket-extensions');wire['fakeAuth']=h.get('authorization');wire['canary']=h.get('x-opi-offline-canary');key=h['sec-websocket-key'];accept=base64.b64encode(hashlib.sha1((key+'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
    headers=['HTTP/1.1 101 Switching Protocols','Upgrade: websocket','Connection: '+case.get('connection','Upgrade'),'Sec-WebSocket-Accept: '+('wrong' if case.get('bad') else accept)]
    headers+=['X-Extra-%d: x'%i for i in range(case.get('headers',0))]
    if case.get('pad'):headers+=['X-Padding: '+'x'*case['pad']]
    if case.get('ext'):headers+=['Sec-WebSocket-Extensions: '+case['ext']]
    if case.get('sub'):headers+=['Sec-WebSocket-Protocol: unrequested']
    response=('\r\n'.join(headers)+'\r\n\r\n').encode();compress=bool(case.get('ext','').startswith('permessage-deflate') or case.get('illegal'));encoder=zlib.compressobj(wbits=-case.get('bits',15));payloads=[]
    for i in range(2 if case.get('multi') else 1):
     msg=(TEXT+str(i)).encode();message_compress=compress and not(case.get('mixed') and i==1);payload=encoder.compress(msg)+encoder.flush(zlib.Z_SYNC_FLUSH) if message_compress else msg
     if message_compress:payload=payload[:-4]
     payloads.append(dict(text=msg.decode(),compressedHex=payload.hex()))
     if case.get('fragment') or (case.get('mixed') and i==1):
      p=len(payload)//2;response+=frame(payload[:p],rsv=message_compress,fin=False)+frame('ping-canary',op=9)+frame(payload[p:],op=0,rsv=bool(case.get('bad_continuation')))
     else:response+=frame(payload,rsv=message_compress)
     if case.get('reset'):encoder=zlib.compressobj(wbits=-case.get('bits',15))
    
    if case.get('bad_control'):response+=frame('bad-rsv-ping',op=9,rsv=True)
    response+=frame(b'\x03\xe8',op=8);wire['responseSha256']=hashlib.sha256(response).hexdigest();wire['responseBytes']=len(response);wire['payloads']=payloads;conn.sendall(response)
    inbound=tail
    try:
     while True:
      b=conn.recv(8192)
      if not b:break
      inbound+=b
    except (TimeoutError,ConnectionError):pass
    wire['clientFramesHex']=inbound.hex();conn.close()
   except Exception as e:errors.append(repr(e))
  t=threading.Thread(target=serve,daemon=True);t.start();url=f'ws://127.0.0.1:{port}/readiness/{case["id"]}'
  args=[exe,*extra,str(P/'client.mjs'),url] if runtime in ('node','bun') else [exe,url,*extra]
  env={k:os.environ[k] for k in ['SystemRoot','TEMP','TMP'] if k in os.environ};env['PATH']=str(pathlib.Path(exe).parent)
  started=time.monotonic()
  try:r=subprocess.run(args,capture_output=True,text=True,env=env,timeout=4);stdout=r.stdout;stderr=r.stderr;code=r.returncode
  except subprocess.TimeoutExpired as e:stdout=str(e.stdout);stderr=str(e.stderr);code='harness-timeout'
  t.join(2.1);listener.close()
  try:result=json.loads(stdout.strip())
  except Exception:result=None
  row=dict(runtime=runtime,case=case,command=args,exit=code,stdout=stdout,stderr=stderr,result=result,wire=wire,serverErrors=errors,seconds=time.monotonic()-started);rows.append(row)
  print(runtime,case['id'],code, None if result is None else (result.get('opened'),len(result.get('messages',[])),result.get('error')),flush=True)
(P/'results.json').write_text(json.dumps(dict(scope='loopback-public-codec-not-opi-provider',executableHashes=executableHashes,containment='500ms native read, 1400ms JS, 4s process; experiment only',results=rows),indent=2)+'\n')
assert len(rows)==80
