import pathlib,json,hashlib,subprocess,os
P=pathlib.Path(__file__).parent;S=P/'schema-unicode16';R=S/'results'
source=pathlib.Path('E:/opi/cargo-targets/windows-direct/release/opi-pi130-readiness-schema-probe.exe');exe=P/'schema-fixed-client.exe'
expected=json.loads((R/'summary.json').read_text())['native_executable_sha256']
assert hashlib.sha256(source.read_bytes()).hexdigest()==expected
exe.write_bytes(source.read_bytes());assert hashlib.sha256(exe.read_bytes()).hexdigest()==expected
pi={r['id']:r for r in map(json.loads,(R/'pi.jsonl').read_text(encoding='utf-8').splitlines())}
native={r['id']:r for r in map(json.loads,(R/'native.jsonl').read_text(encoding='utf-8').splitlines())}
env={k:os.environ[k] for k in ['SystemRoot','TEMP','TMP'] if k in os.environ};rows=[]
for case in map(json.loads,(S/'cases.jsonl').read_text(encoding='utf-8').splitlines()):
 case['arguments']=pi[case['id']]['input_js'];raw=(json.dumps(case,ensure_ascii=False)+'\n').encode()
 r=subprocess.run([str(exe)],input=raw,capture_output=True,env=env,timeout=30)
 row=dict(case=case['id'],command=[str(exe)],input_sha256=hashlib.sha256(raw).hexdigest(),exit=r.returncode,stdout=r.stdout.decode(),stderr=r.stderr.decode());rows.append(row)
 assert r.returncode==0 and json.loads(r.stdout)==native[case['id']]
raw=(S/'faults.jsonl').read_bytes();r=subprocess.run([str(exe)],input=raw,capture_output=True,env=env,timeout=30)
assert r.returncode==0 and list(map(json.loads,r.stdout.splitlines()))==list(map(json.loads,(R/'faults.jsonl').read_bytes().splitlines()))
rows.append(dict(case='fault-controls',command=[str(exe)],input_sha256=hashlib.sha256(raw).hexdigest(),exit=r.returncode,stdout=r.stdout.decode(),stderr=r.stderr.decode()))
(P/'schema-fixed-replay.json').write_text(json.dumps(dict(executable_sha256=expected,commands=rows,cases=64,fault_controls=11,result='all-identical'),indent=2)+'\n')
print(json.dumps(dict(result='all-identical',native_cases=64,fault_controls=11,commands=len(rows),executable_sha256=expected)))
