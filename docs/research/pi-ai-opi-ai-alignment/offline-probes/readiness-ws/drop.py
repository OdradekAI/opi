import socket,threading,subprocess,pathlib,base64,hashlib,json,os,time
P=pathlib.Path(__file__).parent;exe=r'E:/opi/cargo-targets/windows-direct/debug/opi-readiness-ws-probe.exe';rows=[]
for name,tail in [('partial-frame-header',b'\x81'),('partial-frame-payload',b'\x81\x64abc')]:
 server=socket.socket();server.bind(('127.0.0.1',0));server.listen();wire={}
 def serve():
  c,_=server.accept();c.settimeout(2);data=b''
  while b'\r\n\r\n' not in data:data+=c.recv(4096)
  key=next(line.split(b':',1)[1].strip() for line in data.split(b'\r\n') if line.lower().startswith(b'sec-websocket-key:'));accept=base64.b64encode(hashlib.sha1(key+b'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest());start=time.monotonic();c.sendall(b'HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: '+accept+b'\r\n\r\n'+tail)
  total=b''
  try:
   while True:
    b=c.recv(4096)
    if not b:wire['eof']=True;break
    total+=b
  except ConnectionResetError:wire['reset']=True
  except TimeoutError:wire['harnessTimeout']=True
  wire['secondsAfterWrite']=time.monotonic()-start;wire['receivedHex']=total.hex();c.close()
 t=threading.Thread(target=serve);t.start();env={k:os.environ[k] for k in ['SystemRoot','TEMP','TMP'] if k in os.environ};env['PATH']=str(pathlib.Path(exe).parent);cmd=[exe,f'ws://127.0.0.1:{server.getsockname()[1]}/drop/{name}','owned'];r=subprocess.run(cmd,capture_output=True,text=True,env=env,timeout=4);t.join();server.close();rows.append(dict(case=name,command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr,wire=wire));assert r.returncode==0 and (wire.get('eof') or wire.get('reset')) and not wire.get('harnessTimeout')
(P/'drop-results.json').write_text(json.dumps(dict(scope='read-timeout-then-destroy-whole-socket; never cancel-and-reuse',results=rows),indent=2)+'\n');print(json.dumps(rows,indent=2))
