import pathlib,subprocess,os,json,hashlib,sys,shutil
from cryptography.hazmat.primitives import serialization
P=pathlib.Path(__file__).parent;f=P/'transport/fixtures';repo=pathlib.Path(r'D:/Luiz/Odradek/opi');a=repo/'docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-transport-auth'
dll=pathlib.Path(r'D:/Users/Luiz/anaconda3/Library/bin/libcrypto-3-x64.dll')
assert hashlib.sha256(dll.read_bytes()).hexdigest()=='630b0735e0a3ec5824084698a78d2043326bc133a5d579aa0f41cf0b5a92ddb6'
(f/'malformed.pem').write_text('synthetic malformed PEM')
key=serialization.load_pem_private_key((f/'rsa-2048-e65537-pkcs8.pem').read_bytes(),password=None)
(f/'encrypted.pem').write_bytes(key.private_bytes(serialization.Encoding.PEM,serialization.PrivateFormat.PKCS8,serialization.BestAvailableEncryption(b'synthetic-never-provided')))
multi=pathlib.Path(r'C:/Users/Luiz/AppData/Local/Temp/opi-pi130-ready-transport-auth/replay/fixtures/rsa-multiprime.pem')
if multi.exists():shutil.copyfile(multi,f/'rsa-multiprime.pem')
poison=P/'private-ctx-poison.cnf';poison.write_text('openssl_conf = openssl_init\nconfig_diagnostics = 1\n[openssl_init]\nalg_section = algorithm_section\n[algorithm_section]\ndefault_properties = provider=OPI_SYNTHETIC_MISSING_PROVIDER\n')
env={k:os.environ[k] for k in ['SystemRoot','PATH'] if k in os.environ};env.update(TEMP=str(P),TMP=str(P),OPENSSL_CONF=str(poison),OPENSSL_MODULES=str(P/'does-not-exist'))
cmd=[sys.executable,str(a/'openssl-libctx-probe.py'),str(dll),r'C:/Users/Luiz/AppData/Local/Temp/opi-pi130-ready-transport-auth/npm',str(f),r'D:/nvm/v24.13.0/node.exe']
r=subprocess.run(cmd,capture_output=True,env=env,timeout=60)
(P/'crypto-replay.stdout').write_bytes(r.stdout);(P/'crypto-replay.stderr').write_bytes(r.stderr)
record=dict(command=cmd,exit=r.returncode,stdout_sha256=hashlib.sha256(r.stdout).hexdigest(),stderr_sha256=hashlib.sha256(r.stderr).hexdigest(),fixture_scope='10 newly generated RSA; 3 synthetic negative; optional named author synthetic multiprime file')
(P/'crypto-replay-command.json').write_text(json.dumps(record,indent=2)+'\n')
assert r.returncode==0,r.stderr
data=json.loads(r.stdout);print(json.dumps(dict(exit=r.returncode,observations=len(data['observations']),rsa=sum(x['kind']=='rsa-private-libctx' for x in data['observations']))))
