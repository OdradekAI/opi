"""Independent R-02 check: local frozen files only; no sockets or requests."""
import base64, hashlib, json, os, pathlib, shutil, subprocess, sys, zlib

ROOT=pathlib.Path(__file__).resolve().parent
REPO=pathlib.Path(r'D:/Luiz/Odradek/opi')
ASSETS=REPO/'docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-ws'
OWN=REPO/'docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-independent'

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    scratch=ROOT/'summary-r02'
    if scratch.exists():raise SystemExit('Use a new exclusive scratch; do not delete existing evidence')
    encoded=json.loads((OWN/'ws-results.json.zlib-base64.json').read_text())
    raw=zlib.decompress(base64.b64decode(encoded['payload']))
    assert hashlib.sha256(raw).hexdigest()==encoded['sha256'] and len(raw)==encoded['decoded_bytes']
    historical=[ASSETS/'results.json',ASSETS/'summary.json']
    author=pathlib.Path(r'C:/Users/Luiz/AppData/Local/Temp/opi-readiness-ws')
    historical += [p for p in [author/'results.json',author/'summary.json'] if p.exists()]
    before={str(p):sha(p) for p in historical}
    positive=scratch/'positive';negative=scratch/'negative'
    for root in [positive,negative]:
        root.mkdir(parents=True)
        shutil.copyfile(ASSETS/'summarize.py',root/'summarize.py')
    (positive/'results.json').write_bytes(raw)
    (positive/'rust').mkdir()
    shutil.copyfile(ASSETS/'rust/Cargo.lock',positive/'rust/Cargo.lock')
    shutil.copyfile(ASSETS/'outbound.txt',positive/'outbound.txt')
    env={k:v for k,v in os.environ.items() if k.upper() in ['SYSTEMROOT','PATH','TEMP','TMP']}
    commands=[]
    for name,root,expected in [('positive',positive,0),('missing-local-results',negative,1)]:
        command=[sys.executable,str(root/'summarize.py')]
        process=subprocess.run(command,cwd=REPO,env=env,capture_output=True,timeout=30)
        stdout,stderr=process.stdout,process.stderr
        (scratch/(name+'.stdout')).write_bytes(stdout);(scratch/(name+'.stderr')).write_bytes(stderr)
        commands.append(dict(name=name,command=command,cwd=str(REPO),exit=process.returncode,stdout=stdout.decode(errors='replace'),stderr=stderr.decode(errors='replace'),script_sha256=sha(root/'summarize.py')))
        assert process.returncode==expected
    summary=json.loads((positive/'summary.json').read_text())
    assert summary['observations']==80 and summary['secondContextPayloadRequiresDictionary']
    assert not (REPO/'summary.json').exists()
    assert sorted(p.name for p in negative.iterdir())==['summarize.py']
    assert {str(p):sha(p) for p in historical}==before
    result=dict(scope='LOCAL-FILES-ONLY-NO-SOCKET-NO-REQUEST',observations=80,positive_exit=0,negative_exit=1,historical_files_unchanged=True,historical_sha256=before,input_ws_results_sha256=encoded['sha256'],script_sha256=sha(ASSETS/'summarize.py'),commands=commands)
    (OWN/'ws-r02-local-summary.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['scope','observations','positive_exit','negative_exit','historical_files_unchanged','script_sha256']}))

if __name__=='__main__':main()
