"""Pinned pure offline Number::toString/WHATWG URL.canParse leaf comparison."""
import argparse,hashlib,json,os,shutil,subprocess,sys,tarfile,time
from pathlib import Path
p=argparse.ArgumentParser()
for n in ['node','target','archives','scratch']:p.add_argument('--'+n,required=True)
p.add_argument('--profile',choices=['baseline','icu16'],required=True)
p.add_argument('--assets',default=str(Path(__file__).resolve().parent))
a=p.parse_args();assets=Path(a.assets).resolve();root=Path(a.scratch).resolve();root.mkdir(parents=True,exist_ok=True);out=root/'results';out.mkdir(exist_ok=True)
def sha(f):return hashlib.sha256(Path(f).read_bytes()).hexdigest()
pins=json.loads((assets/'pins.json').read_text(encoding='utf-8'));leafpins=json.loads((assets/'leaf-inputs.json').read_text(encoding='utf-8'))
if sha(a.node)!=pins['node_sha256']:sys.exit('Node pin mismatch')
for f,h in leafpins['assets'].items():
    if sha(assets/f)!=h:sys.exit('Leaf input pin mismatch: '+f)
archives=[leafpins['ryu_js']]+(leafpins['icu16_archives'] if a.profile=='icu16' else [])
for spec in archives:
    name=spec['name']+'-'+spec['version'];archive=Path(a.archives)/(name+'.crate')
    if sha(archive)!=spec['sha256']:sys.exit('Leaf archive pin mismatch: '+name)
    with tarfile.open(archive) as t:
        for m in t.getmembers():
            if not (root/'vendor'/m.name).resolve().is_relative_to((root/'vendor').resolve()):sys.exit('Archive traversal')
        t.extractall(root/'vendor',filter='data')
crate=root/'leaves';(crate/'src').mkdir(parents=True,exist_ok=True)
for source,target in [('leaves/src/main.rs','src/main.rs'),('leaves/'+a.profile+'/Cargo.toml.template','Cargo.toml'),('leaves/'+a.profile+'/Cargo.lock','Cargo.lock')]:shutil.copyfile(assets/source,crate/target)
allow={'SYSTEMROOT','WINDIR','COMSPEC','PATHEXT','PATH','USERPROFILE','HOMEDRIVE','HOMEPATH','APPDATA','LOCALAPPDATA','TEMP','TMP','CARGO_HOME','RUSTUP_HOME','INCLUDE','LIB','LIBPATH','VCINSTALLDIR','VCTOOLSINSTALLDIR','VSINSTALLDIR','WINDOWSSDKDIR','WINDOWSSDKVERSION','VSCMD_ARG_TGT_ARCH','VSCMD_ARG_HOST_ARCH','PROGRAMFILES','PROGRAMFILES(X86)','PROGRAMW6432','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS'}
env={k:v for k,v in os.environ.items() if k.upper() in allow};env.update(CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR=str(Path(a.target).resolve()),CARGO_INCREMENTAL='1')
commands=[]
def run(name,cmd,data=None,timeout=60):
    start=time.time();r=subprocess.run(cmd,input=data,cwd=root,env=env,capture_output=True,timeout=timeout)
    (out/(name+'.stdout')).write_bytes(r.stdout);(out/(name+'.stderr')).write_bytes(r.stderr)
    row={'name':name,'command':cmd,'exit':r.returncode,'elapsed_seconds':round(time.time()-start,3),'stdout_sha256':hashlib.sha256(r.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(r.stderr).hexdigest()};commands.append(row);(out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n',encoding='utf-8')
    if r.returncode!=0:sys.exit('Required leaf command failed: '+name)
    return r.stdout
if hashlib.sha256(run('rustc-version',['rustc','-Vv'])).hexdigest()!=pins['rustc_vv_sha256']:sys.exit('rustc pin mismatch')
if hashlib.sha256(run('cargo-version',['cargo','-Vv'])).hexdigest()!=pins['cargo_vv_sha256']:sys.exit('Cargo pin mismatch')
manifest=str(crate/'Cargo.toml')
run('cargo-build',['cargo','build','--release','--offline','--locked','--manifest-path',manifest],timeout=300)
run('cargo-metadata',['cargo','metadata','--offline','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc','--manifest-path',manifest])
run('cargo-features',['cargo','tree','--offline','--locked','-e','features','--manifest-path',manifest])
exe=Path(a.target)/('release/opi-pi130-schema-leaf16-probe.exe' if a.profile=='icu16' else 'release/opi-pi130-schema-leaf-probe.exe')
private_exe=root/'leaf-probe.exe';shutil.copyfile(exe,private_exe)
cases=json.loads((assets/'leaves/cases.json').read_text(encoding='utf-8'))
expected=[json.loads(l) for l in run('node',[a.node,str(assets/'leaves/node.mjs'),str(assets/'leaves/cases.json')]).splitlines()]
observed=[json.loads(l) for l in run('native',[str(private_exe)],data=(''.join(json.dumps(c)+'\n' for c in cases)).encode()).splitlines()]
if len(expected)!=43 or len(observed)!=43:sys.exit('Leaf count mismatch')
summary={'profile':a.profile,'cases':43,'number_cases':14,'url_cases':29,'matched':sum(e==o for e,o in zip(expected,observed)),'mismatches':[{'case':c,'node':e,'native':o} for c,e,o in zip(cases,expected,observed) if e!=o],'native_sha256':sha(private_exe),'status':'LEAF-MECHANISMS-NOT-NORMALIZER-OR-PRODUCT','environment_names':sorted(env)}
(out/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(json.dumps(summary))
