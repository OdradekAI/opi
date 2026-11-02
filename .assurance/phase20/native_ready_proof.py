"""Verification-branch entry point; consumes an immutable production commit."""
import argparse,pathlib,subprocess,sys,os,json,tarfile,shutil,hashlib,tomllib

def main():
 p=argparse.ArgumentParser();p.add_argument('--repository',required=True);p.add_argument('--out',required=True);p.add_argument('--head',default='e433ac833a28ece771ec8dbb00f947efbd592055');p.add_argument('--prime-registry',action='store_true');a=p.parse_args()
 repository=pathlib.Path(a.repository).resolve();out=pathlib.Path(a.out).resolve();out.mkdir(parents=True,exist_ok=False);source=out/'source';source.mkdir();archive=out/'source.tar'
 def run(cmd,cwd):subprocess.run(cmd,cwd=cwd,check=True,timeout=3600)
 run(['git','archive','--format=tar','-o',str(archive),a.head],repository)
 with tarfile.open(archive) as t:t.extractall(source,filter='data')
 # Workspace/toolchain-scoped cache, never a per-run target directory.
 target=pathlib.Path(subprocess.check_output([sys.executable,str(repository/'scripts/opi-cargo-cache.py'),'resolve','--workspace',str(repository)],cwd=repository,text=True).strip())
 os.environ.update(CARGO_TARGET_DIR=str(target),CARGO_INCREMENTAL='1',CARGO_BUILD_JOBS='4',AWS_LC_SYS_USE_SYSTEM='0',LIBZ_SYS_STATIC='1',VCPKGRS_DISABLE='1',CC_ENABLE_DEBUG_OUTPUT='1')
 sys.path.insert(0,str(source/'scripts/pi-sdk-conformance'))
 import package_slice as family
 import run_early_package_proof as schema
 old_validate=schema.validate_paths
 def archive_paths(repo,scratch,cache):
  old_validate(repository,scratch,cache)
  if repo.resolve()!=source or scratch.is_relative_to(source):raise ValueError('sealed source/scratch boundary')
 schema.validate_paths=archive_paths;schema.REPO=source;family.REPO=source
 if a.prime_registry:
  run(['cargo','fetch','--locked'],source)
  # Registry acquisition only: no provider, process, build script or model call.
  # Freeze the exact candidate lock before the offline evidence consumers.
  prime=out/'registry-prime';prime.mkdir();(prime/'src').mkdir();(prime/'src/lib.rs').write_text('')
  manifest='[package]\nname="phase20-proof-registry-prime"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\nflate2={version="=1.1.10",features=["zlib"]}\nregress={version="=0.12.0",features=["prohibit-unsafe"]}\nreqwest={version="=0.12.28",features=["gzip"],default-features=false}\nrustls={version="=0.23.40",features=["ring","aws_lc_rs"],default-features=false}\nlibz-sys="=1.1.30"\n'
  frozen={'aws-lc-rs','aws-lc-sys','cc','url','idna','idna_adapter','tokio','tokio-util','hyper','hyper-util','http','h2','serde','serde_core','serde_derive','serde_json','time','time-core','time-macros','proc-macro2','quote','syn'}
  packages=tomllib.loads((source/'Cargo.lock').read_text(encoding='utf-8'))['package']
  frozen.update(x['name'] for x in packages if x['name'].startswith(('icu_','futures-')))
  for name in sorted(frozen):
   versions={x['version'] for x in packages if x['name']==name and x.get('source','').startswith('registry+')}
   if len(versions)==1:manifest+=name+'="='+next(iter(versions))+'"\n'
  (prime/'Cargo.toml').write_text(manifest,encoding='utf-8')
  run(['cargo','generate-lockfile'],prime);run(['cargo','fetch','--locked'],prime)
 report={'head':a.head,'host':sys.platform,'target_cache':str(target),'families':[]}
 for name in ['ws','http','crypto']:
  scratch=out/('package-'+name);sys.argv=['package_slice.py','--family',name,'--offline','--scratch',str(scratch),'--target-dir',str(target)]
  if family.main():return 1
  evidence=json.loads((scratch/'evidence.json').read_bytes());saved=scratch/'retained-artifacts';saved.mkdir()
  for profile in evidence['profiles']:
   files=[profile['executable']['file']]
   for native in profile['native_provenance'].values():files+=native['static_archives']
   for i,item in enumerate(files):
    f=pathlib.Path(item);shutil.copyfile(f,saved/(profile['name']+'-'+str(i)+'-'+f.name))
  report['families'].append({'family':name,'evidence':str(scratch/'evidence.json'),'sha256':hashlib.sha256((scratch/'evidence.json').read_bytes()).hexdigest()})
 package=out/'package-ws/package/opi-ai-0.8.3'
 run([sys.executable,str(pathlib.Path(__file__).with_name('full_sdk_matrix.py')),'--package',str(package),'--target-dir',str(target),'--out',str(out/'full-sdk')],repository)
 sys.argv=['run_early_package_proof.py','--scratch',str(out/'schema-package'),'--target-dir',str(target)]
 if schema.main():return 1
 report['verdict']='PASS';(out/'native-proof.json').write_text(json.dumps(report,indent=2),encoding='utf-8');print(json.dumps(report,indent=2));return 0
if __name__=='__main__':raise SystemExit(main())
