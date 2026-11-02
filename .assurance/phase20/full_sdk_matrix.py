"""Execute actual packaged public SDK test sources from an ordinary consumer.

No product source is changed; this is an external evidence producer. The
per-family package runners separately prove private protocol/native details.
"""
import argparse,pathlib,json,subprocess,os,hashlib,time,sys,re,shutil,tomllib

def main():
    p=argparse.ArgumentParser();p.add_argument('--package',required=True);p.add_argument('--target-dir',required=True);p.add_argument('--out',required=True);a=p.parse_args()
    package=pathlib.Path(a.package).resolve();out=pathlib.Path(a.out).resolve();target=pathlib.Path(a.target_dir).resolve()
    if out.exists() or out.is_relative_to(package) or out.is_relative_to(target):raise ValueError('fresh external output required')
    out.mkdir(parents=True);consumer=out/'consumer';consumer.mkdir();(consumer/'src').mkdir();(consumer/'src/lib.rs').write_text('',encoding='utf-8')
    tests=['external_sdk','provider_collection','oauth_flows','deferred_contract','classifier_contract']
    manifest='''[package]
name="phase20-actual-package-sdk-consumer"
version="0.0.0"
edition="2024"
[workspace]
[dependencies]
opi-ai={path=PACKAGE}
tokio={version="1",features=["rt-multi-thread","macros","net","io-util","time","test-util"]}
tokio-util="0.7"
serde={version="1",features=["derive"]}
serde_json="1"
futures-util="0.3"
secrecy="0.10"
wiremock="0.6"
tempfile="3"
base64="0.22"
time={version="0.3",features=["serde","formatting","parsing"]}
url="=2.5.8"
regress={version="=0.12.0",default-features=false,features=["std","utf16","prohibit-unsafe"]}
flate2={version="=1.1.10",default-features=false}
rustls={version="=0.23.40",default-features=false,features=["aws_lc_rs","std","tls12"]}
tokio-rustls="=0.26.4"
reqwest={version="=0.12.28",default-features=false,features=["rustls-tls","json","stream","http2"]}
hyper="=1.9.0"
hyper-util="=0.1.20"
http="=1.4.0"
[features]
caller-default=["regress/default","flate2/default"]
caller-c-zlib=["flate2/zlib"]
'''.replace('PACKAGE',json.dumps(package.as_posix()))
    # Bind ordinary helper versions to the current Cargo-generated package
    # lock rather than silently selecting unrelated newer cached revisions.
    locked=tomllib.loads((package/'Cargo.lock').read_text(encoding='utf-8'))['package']
    for name in ['tokio','tokio-util','serde','serde_json','futures-util','secrecy','wiremock','tempfile','base64','time']:
        versions={x['version'] for x in locked if x['name']==name}
        if len(versions)!=1:raise ValueError('ambiguous locked helper '+name)
        value='='+next(iter(versions))
        manifest=re.sub(r'(?m)^'+re.escape(name)+r'=\{version="[^"]+"',lambda m:m[0].split('version=')[0]+'version='+json.dumps(value),manifest)
        manifest=re.sub(r'(?m)^'+re.escape(name)+r'="[^"]+"',lambda _:name+'='+json.dumps(value),manifest)
    for test in tests:manifest+='\n[[test]]\nname='+json.dumps(test)+'\npath='+json.dumps((package/'tests'/f'{test}.rs').as_posix())+'\n'
    (consumer/'Cargo.toml').write_text(manifest,encoding='utf-8',newline='\n')
    env=dict(os.environ,CARGO_TARGET_DIR=str(target),CARGO_INCREMENTAL='1',AWS_LC_SYS_USE_SYSTEM='0',LIBZ_SYS_STATIC='1',VCPKGRS_DISABLE='1',CC_ENABLE_DEBUG_OUTPUT='1')
    record={'package':str(package),'tests':tests,'source_sha256':{str(package/'tests'/f'{t}.rs'):hashlib.sha256((package/'tests'/f'{t}.rs').read_bytes()).hexdigest() for t in tests},'steps':[],'verdict':'FAIL'}
    def run(name,cmd):
        start=time.monotonic();r=subprocess.run(cmd,cwd=consumer,env=env,capture_output=True,timeout=3600);(out/(name+'.stdout')).write_bytes(r.stdout);(out/(name+'.stderr')).write_bytes(r.stderr);record['steps'].append({'name':name,'command':cmd,'exit':r.returncode,'seconds':round(time.monotonic()-start,2)})
        if r.returncode:raise RuntimeError(f'{name} exit={r.returncode}')
        return r.stdout.decode('utf-8',errors='replace')
    try:
        run('lock',['cargo','generate-lockfile','--offline'])
        for profile,flags in [('ordinary',[]),('default',['--features','caller-default']),('c-zlib',['--features','caller-c-zlib'])]:
            run(profile+'-metadata',['cargo','metadata','--offline','--locked','--format-version=1']+flags)
            log=run(profile+'-tests',['cargo','test','--offline','--locked','--tests','--message-format=json']+flags+['--','--test-threads=1'])
            messages=[]
            for line in log.splitlines():
                if line.startswith('{'):
                    try: messages.append(json.loads(line))
                    except ValueError: pass
            binaries=[]
            for item in messages:
                if item.get('reason')=='compiler-artifact' and item.get('executable'):
                    binary=pathlib.Path(item['executable']);saved=out/(profile+'-'+binary.name);shutil.copyfile(binary,saved);binaries.append({'file':str(saved),'sha256':hashlib.sha256(saved.read_bytes()).hexdigest()})
            summaries=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',log)
            if len(summaries)!=len(tests)+1 or any(int(f) or int(i) for _,f,i in summaries) or sum(int(n) for n,_,_ in summaries)<100:raise ValueError('incomplete executed consumer matrix')
            record.setdefault('profiles',[]).append({'name':profile,'executed':sum(int(n) for n,_,_ in summaries),'failed':0,'ignored':0,'executables':binaries})
        record['verdict']='PASS'
    except Exception as e:record['error']=str(e)
    finally:(out/'evidence.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print(json.dumps(record,indent=2));return 0 if record['verdict']=='PASS' else 1
if __name__=='__main__':raise SystemExit(main())
