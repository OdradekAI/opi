import argparse,pathlib,shutil,urllib.request,hashlib,tarfile,json,subprocess,os,sys
HERE=pathlib.Path(__file__).resolve().parent
p=argparse.ArgumentParser(description='Prepare isolated pinned WS research crate; offline by default; never edits product files.')
p.add_argument('--workspace',required=True,type=pathlib.Path);p.add_argument('--allow-download',action='store_true');p.add_argument('--expected-before');p.add_argument('--build',action='store_true');args=p.parse_args()
pins=json.loads((HERE/'patch-pins.json').read_text(encoding='utf-8'))
if args.expected_before is not None and args.expected_before!=pins['before']:raise SystemExit('source pin mismatch before workspace creation')
work=args.workspace.resolve()
if work.exists():raise SystemExit('workspace must not exist')
work.mkdir(parents=True)
for name in ['run.py','client.mjs','outbound.txt','drop.py','deflate-distance.py','summarize.py']:
 shutil.copyfile(HERE/name,work/name)
shutil.copytree(HERE/'rust',work/'rust')
cache=pathlib.Path.home()/'.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f'
retrieval=[]
for row in json.loads((HERE/'retrieval.json').read_text(encoding='utf-8')):
 name=row['name'];ver=row['version'];filename=f'{name}-{ver}.crate';cached=cache/filename
 if cached.exists():b=cached.read_bytes();method='existing-cache'
 elif args.allow_download:b=urllib.request.urlopen(row['source'],timeout=30).read();method='official-download'
 else:raise SystemExit('missing cached '+filename+'; prepare separately or explicitly --allow-download')
 if hashlib.sha256(b).hexdigest()!=row['sha256']:raise SystemExit('archive checksum mismatch: '+filename)
 archive=work/filename;archive.write_bytes(b)
 with tarfile.open(archive) as tar:tar.extractall(work/'source',filter='data')
 retrieval.append(dict(file=filename,method=method,sha256=row['sha256']))
f=work/'source/ratchet_core-1.2.1/src/protocol/frame.rs';b=f.read_bytes()
if hashlib.sha256(b).hexdigest()!=pins['before']:raise SystemExit('source pin mismatch before patch')
needle=b'        let second = source[1];';replacement=b'''        // This private research patch is scoped to the selected PMD-only codec.
        // RFC 7692 section 6.1: RSV1 is only on a message's first data frame.
        if (opcode.is_control() || (first & 0x0f) == 0) && (first & 0x40) != 0 {
            return Err(ProtocolError::UnknownExtension);
        }

        let second = source[1];'''
if b.count(needle)!=1:raise SystemExit('patch anchor mismatch')
b=b.replace(needle,replacement)
if hashlib.sha256(b).hexdigest()!=pins['after']:raise SystemExit('source pin mismatch after patch')
f.write_bytes(b);(work/'preparation.json').write_text(json.dumps(retrieval,indent=2)+'\n',encoding='utf-8')
if args.build:
 command=['cargo','build','--locked','--offline','--manifest-path',str(work/'rust/Cargo.toml')]
 result=subprocess.run(command,capture_output=True,text=True);(work/'cargo-build.txt').write_text(result.stdout+result.stderr,encoding='utf-8')
 if result.returncode:raise SystemExit(result.returncode)
print(json.dumps(dict(workspace=str(work),patchedSourceSha256=pins['after'],buildRequested=args.build)))
