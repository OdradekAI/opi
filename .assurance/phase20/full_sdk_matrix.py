"""Execute actual packaged public SDK test sources from an ordinary consumer.

No product source is changed; this is an external evidence producer. The
per-family package runners separately prove private protocol/native details.
"""
import argparse,pathlib,json,subprocess,os,hashlib,time,sys,re,shutil,tomllib

sys.dont_write_bytecode = True

def local_peer_environment(environment):
    """Copy child inputs; loopback acceptance never inherits proxy credentials."""
    result = {name: value for name, value in environment.items()
              if name.upper() not in {"HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"}}
    result.update(NO_PROXY="127.0.0.1,localhost,::1", no_proxy="127.0.0.1,localhost,::1")
    return result


def local_peer_environment_receipt(environment):
    return {"scope": "test subprocess environment only; host settings unchanged",
            "removed_names": sorted(name for name in environment
                                    if name.upper() in {"HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"}),
            "NO_PROXY": "127.0.0.1,localhost,::1", "no_proxy": "127.0.0.1,localhost,::1"}


class ProcessGuard:
    """Do not declare cache quiescence after an uncontained command timeout."""
    def __init__(self):
        self.uncertain = False

    def run(self, command, **kwargs):
        is_release = ("lease" in command and "end" in command and
                      any(str(part).endswith("opi-cargo-cache.py") for part in command))
        if self.uncertain and is_release:
            message = "cache lease retained: timed-out command descendants are not proven stopped\n"
            text = kwargs.get("text") or kwargs.get("encoding")
            result = subprocess.CompletedProcess(command, 125, "" if text else b"",
                                                 message if text else message.encode())
        else:
            try:
                return subprocess.run(command, **kwargs)
            except subprocess.TimeoutExpired as error:
                self.uncertain = True
                def output(value):
                    value = value or b""
                    if kwargs.get("text") or kwargs.get("encoding"):
                        return value.decode(kwargs.get("encoding") or "utf-8", errors="replace") if isinstance(value, bytes) else value
                    return value.encode() if isinstance(value, str) else value
                result = subprocess.CompletedProcess(command, 124, output(error.stdout), output(error.stderr))
        if kwargs.get("check"):
            raise subprocess.CalledProcessError(result.returncode, command, result.stdout, result.stderr)
        return result

def main():
    p=argparse.ArgumentParser();p.add_argument('--package',required=True);p.add_argument('--target-dir',required=True);p.add_argument('--out',required=True);p.add_argument('--source',required=True);p.add_argument('--repository',required=True);a=p.parse_args()
    source=pathlib.Path(a.source).resolve();repository=pathlib.Path(a.repository).resolve();sys.path.insert(0,str(source/'scripts/pi-sdk-conformance'));import package_slice as family
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
sha2="0.10"
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
    for name in ['tokio','tokio-util','serde','serde_json','futures-util','secrecy','wiremock','tempfile','base64','time','sha2']:
        versions={x['version'] for x in locked if x['name']==name}
        if len(versions)!=1:raise ValueError('ambiguous locked helper '+name)
        value='='+next(iter(versions))
        manifest=re.sub(r'(?m)^'+re.escape(name)+r'=\{version="[^"]+"',lambda m:m[0].split('version=')[0]+'version='+json.dumps(value),manifest)
        manifest=re.sub(r'(?m)^'+re.escape(name)+r'="[^"]+"',lambda _:name+'='+json.dumps(value),manifest)
    for test in tests:manifest+='\n[[test]]\nname='+json.dumps(test)+'\npath='+json.dumps((package/'tests'/f'{test}.rs').as_posix())+'\n'
    (consumer/'Cargo.toml').write_text(manifest,encoding='utf-8',newline='\n')
    env=local_peer_environment(family.controlled_environment(dict(os.environ),target))
    test_sources = [f'{test}.rs' for test in tests] + ['support/catalog_store.rs', 'support/tls_fixtures.rs', 'support/pending_auth.rs']
    for test_source in test_sources:
        if (package/'tests'/test_source).read_bytes() != (source/'crates/opi-ai/tests'/test_source).read_bytes():
            raise ValueError('packaged public test differs from sealed source: '+test_source)
    guard=ProcessGuard()
    record={'package':str(package),'tests':tests,'source_sha256':{str(package/'tests'/t):hashlib.sha256((package/'tests'/t).read_bytes()).hexdigest() for t in test_sources},'steps':[],'verdict':'FAIL','test_environment':local_peer_environment_receipt(os.environ)}
    def run(name,cmd,allow_failure=False):
        print('RUN '+name+' '+subprocess.list2cmdline(cmd),flush=True);start=time.monotonic();r=guard.run(cmd,cwd=consumer,env=env,capture_output=True,timeout=3600);(out/(name+'.stdout')).write_bytes(r.stdout);(out/(name+'.stderr')).write_bytes(r.stderr);record['steps'].append({'name':name,'command':cmd,'exit':r.returncode,'seconds':round(time.monotonic()-start,2)})
        if r.returncode and not allow_failure:raise RuntimeError(f'{name} exit={r.returncode}')
        return r.stdout.decode('utf-8',errors='replace')
    lease=[sys.executable,str(repository/'scripts/opi-cargo-cache.py'),'lease']
    leased=False
    try:
        run('lease-start',lease+['start','--target',str(target),'--pid',str(os.getpid())]);leased=True
        run('lock',['cargo','generate-lockfile','--offline'])
        lock=tomllib.loads((consumer/'Cargo.lock').read_text(encoding='utf-8'))
        record['consumer_manifest_sha256']=hashlib.sha256((consumer/'Cargo.toml').read_bytes()).hexdigest()
        record['consumer_lock_sha256']=hashlib.sha256((consumer/'Cargo.lock').read_bytes()).hexdigest()
        record['rustc']=run('rustc-version',['rustc','-Vv'])
        record['cargo']=run('cargo-version',['cargo','-Vv'])
        record['cargo_configs']=family.check_cargo_configs(env,[consumer,package])
        for profile,flags,graph_profile in [('ordinary',[],'ordinary'),('default',['--features','caller-default'],'union-default'),('c-zlib',['--features','caller-c-zlib'],'union-c-zlib')]:
            metadata=json.loads(run(profile+'-metadata',['cargo','metadata','--offline','--locked','--format-version=1']+flags))
            requirement=next(x['graph_requirements'] for x in family.FAMILIES['ws']['profiles'] if x['name']==graph_profile)
            graph=family.check_family_graph(metadata,package,requirement,lock=lock,profile_name=graph_profile,ws_profile=True)
            profile_record={'name':profile,'graph':graph,'test_targets':[],'executed':0,'failed':0,'ignored':0,'failures':[]}
            for test in tests:
                base=['cargo','test','--offline','--locked','--test',test,'--message-format=json']+flags
                run(profile+'-'+test+'-list',base+['--','--list'])
                run(profile+'-'+test+'-ignored',base+['--','--ignored','--list'])
                log=run(profile+'-'+test+'-tests',base+['--','--test-threads=1'],allow_failure=True)
                test_exit=record['steps'][-1]['exit']
                run(profile+'-'+test+'-execution',[sys.executable,str(source/'.agents/skills/opi-audit/scripts/check_test_execution.py'),'--list-output',str(out/(profile+'-'+test+'-list.stdout')),'--ignored-list-output',str(out/(profile+'-'+test+'-ignored.stdout')),'--run-output',str(out/(profile+'-'+test+'-tests.stdout')),'--selection','normal'],allow_failure=True)
                checker_exit=record['steps'][-1]['exit']
                execution=json.loads((out/(profile+'-'+test+'-execution.stdout')).read_text(encoding='utf-8'))
                messages=[]
                for line in log.splitlines():
                    if line.startswith('{'):
                        try:messages.append(json.loads(line))
                        except ValueError:pass
                binaries=[]
                for item in messages:
                    if item.get('reason')=='compiler-artifact' and item.get('executable') and item.get('target',{}).get('name')==test:
                        binary=pathlib.Path(item['executable']);saved=out/(profile+'-'+binary.name);shutil.copyfile(binary,saved);binaries.append({'file':str(saved),'sha256':hashlib.sha256(saved.read_bytes()).hexdigest()})
                if len(binaries)!=1:raise ValueError('missing exact consumer test executable '+test)
                if test==tests[0]:
                    provenance=family.collect_native_provenance(target,messages,metadata,require_zlib=profile=='c-zlib')
                    for native_name,native in provenance.items():
                        output=pathlib.Path(native['out_dir']).parent/'output';saved=out/(profile+'-'+native_name+'-native-output.txt');shutil.copyfile(output,saved);native['output_snapshot']=str(saved)
                        for i,item in enumerate(native['static_archives']):
                            f=pathlib.Path(item);shutil.copyfile(f,out/(profile+'-'+native_name+'-'+str(i)+'-'+f.name))
                    profile_record['native_provenance']=provenance
                observed=execution['observed'];profile_record['executed']+=observed['executed'];profile_record['failed']+=observed['failed'];profile_record['ignored']+=observed['ignored']
                failed_names=re.findall(r'^test (.+) \.\.\. FAILED$',log,re.MULTILINE)
                if execution['status']!='PASS' or test_exit!=0 or checker_exit!=0:
                    profile_record['failures'].append({'target':test,'test_exit':test_exit,'checker_exit':checker_exit,'test_names':failed_names,'checker_errors':execution.get('errors',[])})
                profile_record['test_targets'].append({'target':test,'execution':execution,'test_exit':test_exit,'checker_exit':checker_exit,'executables':binaries})
            if profile_record['executed']<100 or profile_record['ignored']:raise ValueError('incomplete executed consumer matrix')
            record.setdefault('profiles',[]).append(profile_record)
        record['evidence_status']='COMPLETE'
        record['verdict']='FAIL' if any(p['failures'] for p in record['profiles']) else 'PASS'
    except Exception as e:record['error']=str(e)
    finally:
        if guard.uncertain:record['lease_retained_reason']='Timed-out descendants are not proven stopped; explicit owner reconciliation is required.'
        if leased:
            try:run('lease-end',lease+['end','--target',str(target),'--pid',str(os.getpid())])
            except Exception as e:record['error']=str(e);record['verdict']='FAIL'
        (out/'evidence.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print(json.dumps(record,indent=2));return 0 if record['verdict']=='PASS' else 1
if __name__=='__main__':raise SystemExit(main())
