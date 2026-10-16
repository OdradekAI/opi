"""Regenerate regress tables from pinned Unicode16 with its original generator.

Network/downloads are a separate explicit preparation step. Inputs must exist.
Only the generator's UCD_PATH constant and build-only path manifest change.
The algorithm sources and regress runtime algorithm remain unchanged.
"""
import argparse,hashlib,json,os,shutil,subprocess,sys,tarfile,time,zipfile
from pathlib import Path
p=argparse.ArgumentParser()
for n in ['regress_archive','codegen_archive','ucd_parse_archive','regex_lite_archive','ucd_zip','emoji_sequences','emoji_zwj_sequences','scratch','target']:p.add_argument('--'+n.replace('_','-'),required=True)
p.add_argument('--assets',default=str(Path(__file__).resolve().parent))
a=p.parse_args();assets=Path(a.assets).resolve();root=Path(a.scratch).resolve();root.mkdir(parents=True,exist_ok=True)
pins=json.loads((assets/'generator-inputs.json').read_text(encoding='utf-8'));out=root/'generator-results';out.mkdir(exist_ok=True)
def sha(f):return hashlib.sha256(Path(f).read_bytes()).hexdigest()
for n in ['regress_archive','codegen_archive','ucd_parse_archive','regex_lite_archive','ucd_zip','emoji_sequences','emoji_zwj_sequences']:
    if sha(getattr(a,n))!=pins[n]['sha256']:sys.exit('Generator input pin mismatch: '+n)
for n,h in pins['source_files'].items():
    if sha(assets/n)!=h:sys.exit('Generator source pin mismatch: '+n)
vendor=root/'vendor'
for n in ['regress_archive','codegen_archive','ucd_parse_archive','regex_lite_archive']:
    with tarfile.open(getattr(a,n)) as t:
        for m in t.getmembers():
            if not (vendor/m.name).resolve().is_relative_to(vendor.resolve()):sys.exit('Archive traversal')
        t.extractall(vendor,filter='data')
ucd=root/'ucd-16.0.0';ucd.mkdir(exist_ok=True)
with zipfile.ZipFile(a.ucd_zip) as z:
    for n in z.namelist():
        if not (ucd/n).resolve().is_relative_to(ucd.resolve()):sys.exit('Zip traversal')
    z.extractall(ucd)
shutil.copyfile(a.emoji_sequences,ucd/'emoji-sequences.txt');shutil.copyfile(a.emoji_zwj_sequences,ucd/'emoji-zwj-sequences.txt')
gen=vendor/'regress-0.12.0/gen-unicode';(gen/'src').mkdir(parents=True,exist_ok=True)
for f in (assets/'upstream-generator').glob('*.rs'):shutil.copyfile(f,gen/'src'/f.name)
main=gen/'src/main.rs';original=main.read_text(encoding='utf-8');assert original.count('"/tmp/ucd-17.0.0"')==1
main.write_text(original.replace('"/tmp/ucd-17.0.0"',json.dumps(ucd.as_posix())),encoding='utf-8')
shutil.copyfile(assets/'upstream-generator/Cargo.toml.template',gen/'Cargo.toml')
shutil.copyfile(assets/'upstream-generator/Cargo.lock',gen/'Cargo.lock')
allow={'SYSTEMROOT','WINDIR','COMSPEC','PATHEXT','PATH','USERPROFILE','HOMEDRIVE','HOMEPATH','APPDATA','LOCALAPPDATA','TEMP','TMP','CARGO_HOME','RUSTUP_HOME','INCLUDE','LIB','LIBPATH','VCINSTALLDIR','VCTOOLSINSTALLDIR','VSINSTALLDIR','WINDOWSSDKDIR','WINDOWSSDKVERSION','VSCMD_ARG_TGT_ARCH','VSCMD_ARG_HOST_ARCH','PROGRAMFILES','PROGRAMFILES(X86)','PROGRAMW6432','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS'}
env={k:v for k,v in os.environ.items() if k.upper() in allow};env.update(CARGO_NET_OFFLINE='true',CARGO_TARGET_DIR=str(Path(a.target).resolve()),CARGO_INCREMENTAL='1')
commands=[]
for i in [1,2]:
    cmd=['cargo','run','--release','--offline','--locked','--manifest-path',str(gen/'Cargo.toml')]
    start=time.time();r=subprocess.run(cmd,cwd=gen,env=env,capture_output=True,timeout=300)
    (out/f'generate-{i}.stdout').write_bytes(r.stdout);(out/f'generate-{i}.stderr').write_bytes(r.stderr)
    row={'command':cmd,'exit':r.returncode,'elapsed_seconds':round(time.time()-start,3),'stdout_sha256':hashlib.sha256(r.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(r.stderr).hexdigest()}
    if r.returncode==0:
        row['table_sha256']=sha(vendor/'regress-0.12.0/src/unicodetables.rs');row['tests_sha256']=sha(vendor/'regress-0.12.0/tests/unicode_property_escapes.rs')
    commands.append(row);(out/'commands.json').write_text(json.dumps(commands,indent=2)+'\n',encoding='utf-8');print(json.dumps(row),flush=True)
    if r.returncode!=0:sys.exit('Generator failed; inspect raw streams')
if commands[0]['table_sha256']!=commands[1]['table_sha256'] or commands[0]['tests_sha256']!=commands[1]['tests_sha256']:sys.exit('Nondeterministic generated output')
if commands[0]['table_sha256']!=pins['expected_table_sha256']:sys.exit('Generated table pin mismatch')
shutil.copyfile(vendor/'regress-0.12.0/src/unicodetables.rs',root/'unicodetables-16.rs')
shutil.copyfile(vendor/'regress-0.12.0/tests/unicode_property_escapes.rs',root/'unicode_property_escapes-16.rs')
print(json.dumps({'status':'GENERATOR-REPEATED-IDENTICALLY','table_sha256':commands[0]['table_sha256'],'tests_sha256':commands[0]['tests_sha256']}))
