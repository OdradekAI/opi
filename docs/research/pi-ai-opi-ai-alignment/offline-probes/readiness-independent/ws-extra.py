import socket,threading,subprocess,pathlib,base64,hashlib,json,os
P=pathlib.Path(__file__).parent
source=pathlib.Path('E:/opi/cargo-targets/windows-direct/debug/opi-readiness-ws-probe.exe')
exe=P/'ws-extra-client.exe'
exe.write_bytes(source.read_bytes())
rows=[]
for name,tail in [('length-high-bit',b'\x81\x7f\x80\x00\x00\x00\x00\x00\x00\x00'),('length-u64-overflow',b'\x81\x7f'+b'\xff'*8),('non-minimal-126',b'\x81\x7e\x00\x01x'+b'\x88\x02\x03\xe8')]:
 server=socket.socket();server.bind(('127.0.0.1',0));server.listen();wire={}
 def serve():
  c,_=server.accept();c.settimeout(2);data=b''
  while b'\r\n\r\n' not in data:data+=c.recv(4096)
  key=next(line.split(b':',1)[1].strip() for line in data.split(b'\r\n') if line.lower().startswith(b'sec-websocket-key:'))
  accept=base64.b64encode(hashlib.sha1(key+b'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest())
  c.sendall(b'HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: '+accept+b'\r\n\r\n'+tail)
  try:
   while c.recv(65536):pass
  except (ConnectionError,TimeoutError):pass
  c.close()
 t=threading.Thread(target=serve);t.start()
 env={k:os.environ[k] for k in ['SystemRoot','TEMP','TMP'] if k in os.environ};env['PATH']=str(exe.parent)
 cmd=[str(exe),f'ws://127.0.0.1:{server.getsockname()[1]}/extra/{name}','owned']
 r=subprocess.run(cmd,capture_output=True,env=env,timeout=4);t.join();server.close()
 rows.append(dict(case=name,frame_hex=tail.hex(),command=cmd,exit=r.returncode,stdout=r.stdout.decode(errors='replace'),stderr=r.stderr.decode(errors='replace')))
(P/'ws-extra-results.json').write_text(json.dumps(dict(executable_sha256=hashlib.sha256(exe.read_bytes()).hexdigest(),results=rows),indent=2)+'\n')
print(json.dumps(rows,indent=2))
