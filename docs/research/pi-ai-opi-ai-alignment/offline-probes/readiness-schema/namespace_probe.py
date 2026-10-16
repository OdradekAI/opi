import pathlib,tarfile,tomllib,json,hashlib,subprocess,os,argparse,shutil,sys
import toml
p=argparse.ArgumentParser()
for n in ['archives','scratch','target','node','registry_cache']:p.add_argument('--'+n.replace('_','-'),required=True)
p.add_argument('--assets',default=str(pathlib.Path(__file__).resolve().parent));a=p.parse_args()
r=pathlib.Path(a.archives).resolve();assets=pathlib.Path(a.assets).resolve();root=pathlib.Path(a.scratch).resolve();root.mkdir(exist_ok=True)
registry=pathlib.Path(a.registry_cache).resolve();pins=json.loads((assets/'pins.json').read_text(encoding='utf-8'));npins=json.loads((assets/'namespace-inputs.json').read_text(encoding='utf-8'))
if hashlib.sha256(pathlib.Path(a.node).read_bytes()).hexdigest()!=pins['node_sha256']:sys.exit('Node pin mismatch')
if toml.__version__!='0.10.2':sys.exit('Expected author tool toml0.10.2; do not auto-install')
specs=[(p['name'],p['version']) for p in npins]
sourcehash=[]
for name,version in specs:
    ident=name+'-'+version; archive=r/(ident+'.crate')
    if not archive.exists():archive=registry/(ident+'.crate')
    if hashlib.sha256(archive.read_bytes()).hexdigest()!=next(p['archive_sha256'] for p in npins if p['name']==name):sys.exit('Family archive pin mismatch')
    with tarfile.open(archive) as t:t.extractall(root/'family',filter='data')
    sourcehash.append({'name':name,'version':version,'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest()})
family={n:(v,'pi-schema-private-'+n.replace('_','-')) for n,v in specs}
for n,v in specs:
    folder=root/'family'/(n+'-'+v);p=folder/'Cargo.toml';d=tomllib.loads(p.read_text(encoding='utf-8'));d['package']['name']=family[n][1];d.setdefault('lib',{})['name']=n.replace('-','_')
    def change(table):
        for name,value in table.items():
            if name in family:
                v,pkg=family[name]
                if isinstance(value,str):value={'version':value};table[name]=value
                value.update(package=pkg,path='../'+name+'-'+v)
    for key in ['dependencies','dev-dependencies','build-dependencies']:
        if key in d:change(d[key])
    for target in d.get('target',{}).values():
        for key in ['dependencies','dev-dependencies','build-dependencies']:
            if key in target:change(target[key])
    p.write_text(toml.dumps(d),encoding='utf-8')
crate=root/'probe';(crate/'src').mkdir(parents=True,exist_ok=True)
s=(assets/'leaves/src/main.rs').read_text(encoding='utf-8');(crate/'src/main.rs').write_text(s,encoding='utf-8')
manifest='''[package]
name="opi-pi130-schema-namespace-probe"
version="0.0.0"
edition="2024"
publish=false
[dependencies]
url={package="pi-schema-private-url",path="../family/url-2.5.8"}
ryu-js={path="RYU_PATH"}
serde_json="=1.0.149"
writeable={version="=0.6.3",default-features=false,features=["alloc"]}
consumer_url={package="url",version="=2.5.8"}
consumer_idna_adapter={package="idna_adapter",version="=1.2.2",features=["compiled_data"]}
[workspace]
'''
ryu_archive=r/'ryu-js-1.0.2.crate';ryu_pin=json.loads((assets/'leaf-inputs.json').read_text(encoding='utf-8'))['ryu_js']['sha256']
if hashlib.sha256(ryu_archive.read_bytes()).hexdigest()!=ryu_pin:sys.exit('ryu archive pin mismatch')
with tarfile.open(ryu_archive) as t:t.extractall(root/'vendor',filter='data')
(crate/'Cargo.toml').write_text(manifest.replace('RYU_PATH',(root/'vendor/ryu-js-1.0.2').as_posix()),encoding='utf-8')
shutil.copyfile(assets/'namespace/Cargo.lock',crate/'Cargo.lock')
env={k:v for k,v in os.environ.items() if k.upper() in {'SYSTEMROOT','WINDIR','PATH','USERPROFILE','CARGO_HOME','RUSTUP_HOME','TEMP','TMP','LOCALAPPDATA','APPDATA'}};env.update(CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR=str(pathlib.Path(a.target).resolve()),CARGO_INCREMENTAL='1')
out=root/'results';out.mkdir(exist_ok=True);cmds=[]
for name,cmd in [('build',['cargo','build','--release','--offline','--locked','--manifest-path',str(crate/'Cargo.toml')]),('metadata',['cargo','metadata','--offline','--locked','--filter-platform','x86_64-pc-windows-msvc','--format-version','1','--manifest-path',str(crate/'Cargo.toml')])]:
    p=subprocess.run(cmd,env=env,capture_output=True,timeout=300);(out/(name+'.stdout')).write_bytes(p.stdout);(out/(name+'.stderr')).write_bytes(p.stderr);cmds.append({'name':name,'command':cmd,'exit':p.returncode});print(name,p.returncode,flush=True)
    if p.returncode:print(p.stderr.decode('utf-8',errors='replace')[-6000:]);raise SystemExit(1)
cases=json.loads((assets/'leaves/cases.json').read_text(encoding='utf-8'));data=(''.join(json.dumps(c)+'\n' for c in cases)).encode();exe=pathlib.Path(env['CARGO_TARGET_DIR'])/'release/opi-pi130-schema-namespace-probe.exe';private_exe=root/'namespace-probe.exe';shutil.copyfile(exe,private_exe);p=subprocess.run([str(private_exe)],input=data,capture_output=True,env=env,timeout=60);(out/'native.stdout').write_bytes(p.stdout);(out/'native.stderr').write_bytes(p.stderr)
node=subprocess.run([a.node,str(assets/'leaves/node.mjs'),str(assets/'leaves/cases.json')],capture_output=True,env=env,timeout=30);(out/'node.stdout').write_bytes(node.stdout);(out/'node.stderr').write_bytes(node.stderr)
if p.returncode or node.returncode:sys.exit('Node/native child failed')
cmds.extend([{'name':'native','command':[str(private_exe)],'exit':p.returncode},{'name':'node','command':[a.node,str(assets/'leaves/node.mjs'),str(assets/'leaves/cases.json')],'exit':node.returncode}])
expected=[json.loads(s) for s in node.stdout.splitlines()];observed=[json.loads(s) for s in p.stdout.splitlines()]
if len(expected)!=43 or len(observed)!=43:sys.exit('Namespace case count mismatch')
summary={'native_exit':p.returncode,'cases':len(cases),'matched':sum(e==o for e,o in zip(expected,observed)),'mismatches':[{'case':c,'node':e,'native':o} for c,e,o in zip(cases,expected,observed) if e!=o],'source_archives':sourcehash,'contract':'only source manifests/package identity changed; selected runtime algorithm files remain archive identical; consumer normal url/idna_adapter coexist'}
integrity=[]
for n,v in specs:
    ident=n+'-'+v;archive=r/(ident+'.crate')
    if not archive.exists():archive=registry/(ident+'.crate')
    same=0;changed=[]
    with tarfile.open(archive) as t:
        for member in t.getmembers():
            if not member.isfile():continue
            original=t.extractfile(member).read();actual=(root/'family'/member.name).read_bytes()
            if actual==original:same+=1
            else:changed.append(member.name)
    if changed!=[ident+'/Cargo.toml']:sys.exit('Unexpected family source change: '+ident)
    integrity.append({'name':n,'unchanged_archive_members':same,'modified_members':changed})
summary['archive_member_integrity']=integrity
(out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8');(out/'commands.json').write_text(json.dumps(cmds,indent=2)+'\n',encoding='utf-8');print(json.dumps({'cases':summary['cases'],'matched':summary['matched'],'mismatches':summary['mismatches']}))
