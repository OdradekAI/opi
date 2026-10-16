"""Research-only pinned native dialect/regex mechanisms; no install/network.

See README. Input archive must already be acquired and verified separately.
All child output/exit/containment is preserved. No SDK budgets are defined.
"""
import argparse, hashlib, json, os, shutil, subprocess, sys, tarfile, time
from pathlib import Path

p=argparse.ArgumentParser()
for n in ['node','pi','typebox','target','regress_archive']:p.add_argument('--'+n.replace('_','-'),required=True)
p.add_argument('--scratch',required=True)
p.add_argument('--assets',default=str(Path(__file__).resolve().parent))
p.add_argument('--output',default='results')
p.add_argument('--unicode16-table',help='Explicit generated UCD16 table; never implicit')
args=p.parse_args(); assets=Path(args.assets).resolve(); root=Path(args.scratch).resolve()
root.mkdir(parents=True,exist_ok=True); out=root/args.output;out.mkdir(exist_ok=True)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
pins=json.loads((assets/'pins.json').read_text(encoding='utf-8'))
if sha(args.node)!=pins['node_sha256']:sys.exit('Node pin mismatch')
if sha(Path(args.pi)/'packages/ai/src/utils/validation.ts')!=pins['pi_validation_sha256']:sys.exit('Pi utility pin mismatch')
archive=Path(args.regress_archive).resolve()
if sha(archive)!=pins['regress_archive_sha256']:sys.exit('Regress archive pin mismatch')
for relative,digest in pins['assets'].items():
    if sha(assets/relative)!=digest:sys.exit('Asset pin mismatch: '+relative)
    if assets!=root:
        dest=root/(relative[:-9] if relative.endswith('.template') else relative);dest.parent.mkdir(parents=True,exist_ok=True)
        if dest.exists() and sha(dest)!=digest:sys.exit('Scratch input conflict: '+relative)
        if not dest.exists():shutil.copyfile(assets/relative,dest)
vendor=root/'vendor'
with tarfile.open(archive) as t:
    for member in t.getmembers():
        dest=(vendor/member.name).resolve()
        if not dest.is_relative_to(vendor.resolve()):sys.exit('Archive traversal')
        if member.isfile() and dest.exists() and hashlib.sha256(t.extractfile(member).read()).hexdigest()!=sha(dest):
            if not args.unicode16_table or member.name not in ['regress-0.12.0/src/unicodetables.rs','regress-0.12.0/tests/unicode_property_escapes.rs']:sys.exit('Existing vendor source mismatch')
    t.extractall(vendor,filter='data')
    member_checks=[{'path':m.name,'sha256':hashlib.sha256(t.extractfile(m).read()).hexdigest()} for m in t.getmembers() if m.isfile()]
(out/'regress-members.json').write_text(json.dumps(member_checks,indent=2)+'\n',encoding='utf-8')
if args.unicode16_table:
    if sha(args.unicode16_table)!=pins['unicode16_table_sha256']:sys.exit('Unicode16 table pin mismatch')
    shutil.copyfile(args.unicode16_table,vendor/'regress-0.12.0/src/unicodetables.rs')
# Verify full TypeBox source tree before imports, including source outside direct helpers.
h=hashlib.sha256();files=sorted((f for f in Path(args.typebox).rglob('*') if f.is_file()),key=lambda f:f.relative_to(args.typebox).as_posix())
for f in files:h.update(f.relative_to(args.typebox).as_posix().encode()+b'\0'+bytes.fromhex(sha(f))+b'\n')
if len(files)!=1385 or h.hexdigest()!=pins['typebox_tree_sha256']:sys.exit('TypeBox tree pin mismatch')
allow={'SYSTEMROOT','WINDIR','COMSPEC','PATHEXT','PATH','USERPROFILE','HOMEDRIVE','HOMEPATH','APPDATA','LOCALAPPDATA','TEMP','TMP','CARGO_HOME','RUSTUP_HOME','INCLUDE','LIB','LIBPATH','VCINSTALLDIR','VCTOOLSINSTALLDIR','VSINSTALLDIR','WINDOWSSDKDIR','WINDOWSSDKVERSION','VSCMD_ARG_TGT_ARCH','VSCMD_ARG_HOST_ARCH','PROGRAMFILES','PROGRAMFILES(X86)','PROGRAMW6432','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS'}
env={k:v for k,v in os.environ.items() if k.upper() in allow}
env.update(CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR=str(Path(args.target).resolve()),CARGO_INCREMENTAL='1',PROBE_PI_ROOT=str(Path(args.pi).resolve()),PROBE_TYPEBOX_ROOT=str(Path(args.typebox).resolve()))
commands=[]
def run(name,cmd,data=None,timeout=60):
    start=time.time()
    try:
        r=subprocess.run(cmd,input=data,env=env,cwd=root,capture_output=True,timeout=timeout);stdout,stderr,code,status=r.stdout,r.stderr,r.returncode,'exited'
    except subprocess.TimeoutExpired as e:stdout,stderr,code,status=e.stdout or b'',e.stderr or b'',None,'containment-timeout'
    (out/(name+'.stdout')).write_bytes(stdout);(out/(name+'.stderr')).write_bytes(stderr)
    row=dict(name=name,command=cmd,exit=code,status=status,elapsed_seconds=round(time.time()-start,3),containment_seconds=timeout,stdout_sha256=hashlib.sha256(stdout).hexdigest(),stderr_sha256=hashlib.sha256(stderr).hexdigest())
    commands.append(row);(out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:row[k] for k in ['name','exit','status','elapsed_seconds']}),flush=True)
    if code!=0:sys.exit('Required research command failed: '+name)
    return stdout
if hashlib.sha256(run('rustc-version',['rustc','-Vv'])).hexdigest()!=pins['rustc_vv_sha256']:sys.exit('rustc pin mismatch')
if hashlib.sha256(run('cargo-version',['cargo','-Vv'])).hexdigest()!=pins['cargo_vv_sha256']:sys.exit('Cargo pin mismatch')
manifest=str(root/'rust/Cargo.toml')
run('cargo-build',['cargo','build','--release','--offline','--locked','--manifest-path',manifest],timeout=300)
metadata=run('cargo-metadata',['cargo','metadata','--offline','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc','--manifest-path',manifest])
run('cargo-features',['cargo','tree','--offline','--locked','-e','features','--manifest-path',manifest])
node=[args.node,'--experimental-loader',(root/'loader.mjs').as_uri(),str(root/'pi-runner.mjs'),str(root/'cases.jsonl')]
rows=[json.loads(l) for l in run('pi-normal',node).splitlines()]
cases=[json.loads(l) for l in (root/'cases.jsonl').read_text(encoding='utf-8').splitlines()]
for c in cases:
    if c.get('containment'):rows.extend(json.loads(l) for l in run('pi-'+c['id'],node+[c['id']],timeout=10).splitlines())
mapped={r['id']:r for r in rows if r.get('kind')=='case'}
if len(mapped)!=len(cases):sys.exit('Pi count mismatch')
(out/'pi.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in mapped.values()),encoding='utf-8')
exe=str(Path(args.target)/'release/opi-pi130-readiness-schema-probe.exe')
native=[];comparisons=[]
for c in cases:
    # This harness tests validator mechanisms using JS-parsed inputs, not Rust normalization.
    cc=dict(c);cc['arguments']=mapped[c['id']]['input_js']
    data=run('native-'+c['id'],[exe],data=(json.dumps(cc,ensure_ascii=False)+'\n').encode(),timeout=30)
    result=json.loads(data);native.append(result)
    expected=mapped[c['id']]['direct_typebox']
    if 'check' in expected and result['outcome']=='evaluated':same=result['valid']==expected['check']
    elif 'compile_exception' in expected and result['outcome']=='mechanism-error':same=True
    else:same=False
    comparisons.append({'id':c['id'],'same_observation':same,'pi_check':expected.get('check'),'pi_compile_error':expected.get('compile_exception'),'native':result})
(out/'native.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in native),encoding='utf-8')
fault_input=(root/'faults.jsonl').read_bytes()
faults=[json.loads(l) for l in run('native-fault-controls',[exe],data=fault_input).splitlines()]
fault_ok=all(r['outcome']==('evaluated' if 'unselected' in r['id'] else 'mechanism-error') for r in faults) and all(r.get('valid') is True for r in faults if 'unselected' in r['id'])
(out/'faults.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in faults),encoding='utf-8')
summary={'status':'MECHANISM-RESEARCH-NOT-PRODUCT','profile':'regress0.12.0+UCD16' if args.unicode16_table else 'regress0.12.0-unadapted','unicode16_table_sha256':sha(args.unicode16_table) if args.unicode16_table else None,'cases':len(cases),'matched_observations':sum(c['same_observation'] for c in comparisons),'mismatches':[c for c in comparisons if not c['same_observation']],'fault_controls':len(faults),'fault_controls_expected':fault_ok,'comparisons':comparisons,'native_executable_sha256':sha(exe),'environment_names':sorted(env),'runtime_regex_errors_from_regress':False,'note':'Regress runtime find returns Option, not Result; fault controls are explicitly synthetic Result-propagation tests; TypeBox direct validation on JS parsed inputs, no native normalizer or SDK/public consumer'}
(out/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
if not fault_ok:sys.exit('Fault control expectation failed')
if sha(args.node)!=pins['node_sha256']:sys.exit('Post-execution Node pin mismatch')
if sha(Path(args.pi)/'packages/ai/src/utils/validation.ts')!=pins['pi_validation_sha256']:sys.exit('Post-execution Pi utility pin mismatch')
for relative,digest in pins['assets'].items():
    if sha(root/(relative[:-9] if relative.endswith('.template') else relative))!=digest:sys.exit('Post-execution asset pin mismatch')
print(json.dumps({k:summary[k] for k in ['status','cases','matched_observations','mismatches','fault_controls','fault_controls_expected']}),flush=True)
