"""Offline research harness. Does not edit Pi/Opi sources or install packages.

Supply existing exact runtime, Pi source, cached TypeBox package, Cargo executable
and shared target. Outputs child exit codes/raw streams; a failed build is not a
validation result. Containment timeout is an experiment control, not an SDK cap.
"""
import argparse,hashlib,json,os,shutil,subprocess,sys,time
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('--node',required=True);p.add_argument('--pi',required=True)
p.add_argument('--typebox',required=True);p.add_argument('--cargo',default='cargo')
p.add_argument('--target',required=True);p.add_argument('--output',default='results')
p.add_argument('--scratch',help='Copy research assets into this isolated source directory; target must remain the existing shared Cargo cache')
args=p.parse_args()
assets=Path(__file__).resolve().parent
root=Path(args.scratch).resolve() if args.scratch else assets
if root!=assets:
    root.mkdir(parents=True,exist_ok=True)
    for name in ['loader.mjs','pi-runner.mjs','cases.jsonl','source-pins.json','rust/Cargo.lock','rust/src/main.rs']:
        source=assets/name;dest=root/name;dest.parent.mkdir(parents=True,exist_ok=True)
        if dest.exists() and dest.read_bytes()!=source.read_bytes():sys.exit('Scratch contains different input: '+name)
        if not dest.exists():shutil.copyfile(source,dest)
    source=assets/'rust/Cargo.toml.template'
    if not source.exists():source=assets/'rust/Cargo.toml'
    dest=root/'rust/Cargo.toml'
    if dest.exists() and dest.read_bytes()!=source.read_bytes():sys.exit('Scratch contains different Cargo.toml')
    if not dest.exists():shutil.copyfile(source,dest)
out=root/args.output;out.mkdir(exist_ok=True)
names=['SYSTEMROOT','WINDIR','COMSPEC','PATHEXT','PATH','USERPROFILE','HOMEDRIVE','HOMEPATH','APPDATA','LOCALAPPDATA','TEMP','TMP','CARGO_HOME','RUSTUP_HOME','INCLUDE','LIB','LIBPATH','VCINSTALLDIR','VCTOOLSINSTALLDIR','VSINSTALLDIR','WINDOWSSDKDIR','WINDOWSSDKVERSION','VSCMD_ARG_TGT_ARCH','VSCMD_ARG_HOST_ARCH','ProgramFiles','ProgramFiles(x86)','ProgramW6432','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS']
base={k:v for k,v in os.environ.items() if k.upper() in {n.upper() for n in names}}
base.update({'CARGO_NET_OFFLINE':'true','CARGO_TARGET_DIR':str(Path(args.target).resolve()),'CARGO_INCREMENTAL':'1','PROBE_PI_ROOT':str(Path(args.pi).resolve()),'PROBE_TYPEBOX_ROOT':str(Path(args.typebox).resolve())})
records=[]
pins=json.loads((root/'source-pins.json').read_text(encoding='utf-8'))
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def tree(path,exclude=()):
    path=Path(path);h=hashlib.sha256();n=0
    for f in sorted((f for f in path.rglob('*') if f.is_file() and f.name not in exclude),key=lambda f:f.relative_to(path).as_posix()):
        h.update(f.relative_to(path).as_posix().encode()+b'\0'+bytes.fromhex(sha(f))+b'\n');n+=1
    return {'algorithm':'sha256(sorted relative UTF-8 path + NUL + raw SHA256(file) + LF)','files':n,'sha256':h.hexdigest()}
def check_pins():
    if sha(args.node)!=pins['node']['sha256']:sys.exit('Node executable pin mismatch')
    if sha(Path(args.pi)/'packages/ai/src/utils/validation.ts')!=pins['pi']['validation_ts_sha256']:sys.exit('Pi source pin mismatch')
    if sha(Path(args.pi)/'packages/ai/package.json')!=pins['pi']['package_json_sha256']:sys.exit('Pi package pin mismatch')
    if tree(args.typebox)!=pins['typebox']['package_tree']:sys.exit('TypeBox extracted tree pin mismatch')
    if sha(root/'rust/Cargo.lock')!=pins['cargo_lock_sha256']:sys.exit('Cargo lock pin mismatch')
    if sha(root/'cases.jsonl')!=pins['fixture_sha256']:sys.exit('Fixture pin mismatch')
check_pins()
def run(name,cmd,stdin=None,timeout=60):
    started=time.time()
    try:
        r=subprocess.run(cmd,cwd=root,env=base,input=stdin,capture_output=True,timeout=timeout)
        stdout,stderr,code,status=r.stdout,r.stderr,r.returncode,'exited'
    except subprocess.TimeoutExpired as e:
        stdout,stderr,code,status=e.stdout or b'',e.stderr or b'',None,'containment-timeout'
    (out/(name+'.stdout')).write_bytes(stdout)
    (out/(name+'.stderr')).write_bytes(stderr)
    rec=dict(name=name,command=cmd,exit=code,status=status,elapsed_seconds=round(time.time()-started,3),containment_seconds=timeout,stdout_sha256=hashlib.sha256(stdout).hexdigest(),stderr_sha256=hashlib.sha256(stderr).hexdigest())
    records.append(rec)
    (out/'commands.json').write_text(json.dumps(records,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:rec[k] for k in ['name','exit','status','elapsed_seconds']}),flush=True)
    return code,stdout

def required(name,cmd,**kw):
    code,data=run(name,cmd,**kw)
    if code!=0: sys.exit(f'{name} failed; inspect raw output; no validation conclusion from this command')
    return data
if hashlib.sha256(required('rustc-version',['rustc','-Vv'])).hexdigest()!=pins['rustc_version_stdout_sha256']:sys.exit('rustc version pin mismatch')
if hashlib.sha256(required('cargo-version',[args.cargo,'-Vv'])).hexdigest()!=pins['cargo_version_stdout_sha256']:sys.exit('cargo version pin mismatch')
node=[args.node,'--experimental-loader',(root/'loader.mjs').as_uri(),str(root/'pi-runner.mjs'),str(root/'cases.jsonl')]
data=required('pi-normal',node,timeout=30)
pi=[json.loads(line) for line in data.splitlines()]
runtime=next(x for x in pi if x.get('kind')=='runtime')
if runtime['version']!=pins['node']['version']:sys.exit('Node version mismatch')
mapped={row['id']:row for row in pi if row.get('kind')=='case'}
cases=[json.loads(line) for line in (root/'cases.jsonl').read_text(encoding='utf-8').splitlines()]
contained_pi=[]
for c in cases:
    if c.get('containment'):
        data=required('pi-'+c['id'],node+[c['id']],timeout=5)
        selected=[json.loads(line) for line in data.splitlines()]
        selected=[row for row in selected if row.get('kind')=='case']
        if len(selected)!=1 or selected[0]['id']!=c['id']: sys.exit('Pi contained case count mismatch')
        mapped[c['id']]=selected[0];contained_pi.extend(selected)
for c in cases:
    row=mapped.get(c['id'])
    if row:
        c['pi_input_js']=row['input_js']
        if row['public_pi'].get('outcome')=='returned': c['pi_normalized']=row['public_pi']['normalized']
normal=[c for c in cases if not c.get('containment')]
if len(mapped)!=len(cases): sys.exit('Pi ordinary/contained case count mismatch')
native_input=''.join(json.dumps(c,ensure_ascii=False,separators=(',',':'))+'\n' for c in normal).encode('utf-8')
(out/'native-input.jsonl').write_bytes(native_input)
manifest=root/'rust'/'Cargo.toml'
required('cargo-build',[args.cargo,'build','--offline','--locked','--manifest-path',str(manifest)],timeout=300)
required('cargo-metadata',[args.cargo,'metadata','--offline','--locked','--filter-platform','x86_64-pc-windows-msvc','--format-version','1','--manifest-path',str(manifest)],timeout=60)
required('cargo-tree-features',[args.cargo,'tree','--offline','--locked','-e','features','--manifest-path',str(manifest)],timeout=60)
exe=Path(args.target)/'debug'/('opi-pi130-schema-offline-probe.exe' if os.name=='nt' else 'opi-pi130-schema-offline-probe')
data=required('native-normal',[str(exe)],stdin=native_input,timeout=60)
native_rows=[json.loads(line) for line in data.splitlines()]
for c in cases:
    if c.get('containment'):
        c_input=(json.dumps(c,ensure_ascii=False,separators=(',',':'))+'\n').encode()
        (out/('native-input-'+c['id']+'.jsonl')).write_bytes(c_input)
        data=required('native-'+c['id'],[str(exe)],stdin=c_input,timeout=5)
        native_rows.extend(json.loads(line) for line in data.splitlines())
for c in cases:
    expected=3
    if c['id'].startswith('mixed-') or c['id']=='malformed-minlength':expected+=1
    count=sum(row['id']==c['id'] for row in native_rows)
    stages=1
    if c.get('pi_input_js',c['arguments'])!=c['arguments']:stages+=1
    if 'pi_normalized' in c and c['pi_normalized']!=c.get('pi_input_js',c['arguments']):stages+=1
    expected*=stages
    if count!=expected:sys.exit(f'Native case count mismatch: {c["id"]}, {count} != {expected}')
(out/'pi-all.jsonl').write_text(''.join(json.dumps(row,ensure_ascii=False,separators=(',',':'))+'\n' for row in mapped.values()),encoding='utf-8')
(out/'native-all.jsonl').write_text(''.join(json.dumps(row,ensure_ascii=False,separators=(',',':'))+'\n' for row in native_rows),encoding='utf-8')
snapshot={'status':'PASS','environment_names':sorted(base),'fixed_environment':{k:base[k] for k in ['CARGO_NET_OFFLINE','CARGO_INCREMENTAL']},'node':str(Path(args.node).resolve()),'node_sha256':hashlib.sha256(Path(args.node).read_bytes()).hexdigest(),'native_executable':str(exe),'native_executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'pi_root':str(Path(args.pi).resolve()),'typebox_root':str(Path(args.typebox).resolve()),'shared_target':str(Path(args.target).resolve()),'case_count':len(cases),'normal_case_count':len(normal),'contained_case_count':len(contained_pi),'native_record_count':len(native_rows)}
(out/'runtime-inputs.json').write_text(json.dumps(snapshot,indent=2)+'\n',encoding='utf-8')
check_pins()
