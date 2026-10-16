"""Reproduce isolated tracing and Pi catalog-helper mechanisms; never install dependencies."""
import argparse, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--pi', required=True)
    parser.add_argument('--node', required=True)
    parser.add_argument('--scratch', required=True)
    parser.add_argument('--target', required=True)
    parser.add_argument('--prepare-lock', action='store_true', help='author preparation only, offline Cargo-generated lock')
    args = parser.parse_args()
    assets = Path(__file__).resolve().parent
    pins = json.loads((assets/'pins.json').read_text())
    pi, scratch, target = (Path(v).resolve() for v in (args.pi,args.scratch,args.target))
    if sha(args.node) != pins['node_sha256']: sys.exit('Node pin mismatch')
    for name, digest in pins['pi_sources'].items():
        if sha(pi/name) != digest: sys.exit('Pi source pin mismatch: '+name)
    if not target.is_dir(): sys.exit('Pass an existing shared Cargo target')
    repo = assets.parents[4]
    if scratch == repo or repo in scratch.parents or scratch == pi or pi in scratch.parents:
        sys.exit('Scratch must be isolated from sources')
    scratch.mkdir(parents=True,exist_ok=True)
    crate = scratch/'crate'; (crate/'src').mkdir(parents=True,exist_ok=True)
    for source,dest in [('Cargo.toml.template',crate/'Cargo.toml'),('main.rs',crate/'src/main.rs')]:
        data=(assets/source).read_bytes()
        if dest.exists() and dest.read_bytes()!=data: sys.exit('Scratch contains different source: '+str(dest))
        dest.write_bytes(data)
    if not args.prepare_lock: shutil.copyfile(assets/'Cargo.lock',crate/'Cargo.lock')
    allow={'SYSTEMROOT','WINDIR','PATH','PATHEXT','USERPROFILE','HOMEDRIVE','HOMEPATH','APPDATA','LOCALAPPDATA','TEMP','TMP','CARGO_HOME','RUSTUP_HOME','INCLUDE','LIB','LIBPATH','VCINSTALLDIR','VCTOOLSINSTALLDIR','VSINSTALLDIR','WINDOWSSDKDIR','WINDOWSSDKVERSION','VSCMD_ARG_TGT_ARCH','VSCMD_ARG_HOST_ARCH','PROGRAMFILES','PROGRAMFILES(X86)','PROGRAMW6432','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS'}
    env={k:v for k,v in os.environ.items() if k.upper() in allow}
    env.update(CARGO_NET_OFFLINE='true',CARGO_INCREMENTAL='1',CARGO_TARGET_DIR=str(target))
    commands=[]
    def run(name,cmd,timeout=180):
        result=subprocess.run(cmd,cwd=crate,env=env,capture_output=True,timeout=timeout)
        (scratch/(name+'.stdout')).write_bytes(result.stdout)
        (scratch/(name+'.stderr')).write_bytes(result.stderr)
        commands.append({'name':name,'command':cmd,'exit':result.returncode,'stdout_sha256':hashlib.sha256(result.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(result.stderr).hexdigest()})
        (scratch/'commands.json').write_text(json.dumps(commands,indent=2)+'\n')
        print(json.dumps({'name':name,'exit':result.returncode}),flush=True)
        if result.returncode: sys.exit(name+' failed; see raw output')
        return result.stdout
    rust=run('rustc',['rustc','-Vv']); cargo=run('cargo',['cargo','-Vv'])
    for name,data in [('rustc',rust),('cargo',cargo)]:
        if hashlib.sha256(data).hexdigest()!=pins[name+'_stdout_sha256']: sys.exit(name+' pin mismatch')
    if args.prepare_lock:
        run('generate-lock',['cargo','generate-lockfile','--offline'])
        shutil.copyfile(crate/'Cargo.lock',assets/'Cargo.lock')
    elif sha(crate/'Cargo.lock')!=pins['lock_sha256']:sys.exit('Cargo lock pin mismatch')
    run('build',['cargo','build','--locked','--offline'])
    run('telemetry',[str(target/'debug/opi-pi130-readiness-runtime-probe.exe')])
    run('catalog',[args.node,str(assets/'catalog.mjs'),str(pi),str(scratch)])
    run('metadata',['cargo','metadata','--locked','--offline','--format-version','1'])
    print(json.dumps({'result':'pass','commands':len(commands),'scope':'mechanism only, not product conformance'}))
if __name__=='__main__': main()
