"""Inert Companion process fixture using current native CLI/report field shapes."""
from contextlib import ExitStack
import copy
import json
import os
from pathlib import Path
import shutil
import sys
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from operator_config import dumps_toml
from operator_process import pin, sha, write_json, Commands

COMPANION = r'''
import json, hashlib, os, sys, tomllib
from pathlib import Path
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def read(p): return json.loads(Path(p).read_text())
def write(p, v):
    p = Path(p); p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(v, sort_keys=True) + '\n')
def flag(name): return sys.argv[sys.argv.index(name)+1]
def config(): return tomllib.loads(Path(flag('--config')).read_text())
def manifest(doc): return hashlib.sha256(json.dumps(doc, sort_keys=True).encode()).hexdigest()
def trials(doc):
    if 'trials' in doc: return doc['trials']
    subject=doc['subjects'][0]['id']
    return [{'id':f'trial-{subject}-{task}-r{rep}', 'subject':subject, 'task':task, 'group':f'{task}-r{rep}'}
            for task in doc['selection']['task_ids'] for rep in range(1,doc['selection']['repetitions']+1)]
OBS={'engine_id':'synthetic-engine','engine_cpu':{'status':'known','value':8},
     'engine_memory':{'status':'known','value':16000000000},'engine_gpus':{'status':'known','value':0},'gpu_types':[]}
def corpus(doc):
    return [{'id':task,'selected':True,'status':'ok','requirements':{'cpus':1},'observations':OBS,
             'image_material':{'status':'ok','images':[{'status':'known','image':'synthetic@sha256:'+'1'*64}]}}
            for task in doc['selection']['task_ids']]
command=sys.argv[1]
if command == '--help': print('synthetic Companion: no provider or Docker effects'); raise SystemExit(0)
if command in ('doctor','prepare','validate','plan','verify','report') and os.environ.get('CUSTOM_AUTH'):
    raise SystemExit('synthetic privacy failure: credential inherited by offline step')
if command=='doctor': print('{}')
elif command=='prepare':
    doc=config(); cache=Path(doc['storage']['cache']); out=Path(flag('--out')); out.mkdir()
    agent=doc['subjects'][0]['product']; runtime=cache/'agents'/agent
    write(out/'material.json', {'schema':'opi-eval-native-material/7','origin':'local',
        'static_lock':{'path':str(cache/'pins/static/linux-x86_64.json'),'sha256':sha(cache/'pins/static/linux-x86_64.json')},
        'agents':{agent:{'executable':{'path':str(runtime/agent),'sha256':sha(runtime/agent)},
                         'runtime_package':{'path':str(runtime),'entrypoint':agent,'manifest_sha256':'r'*64}}},
        'benchmarks':{doc['benchmark']['dataset']:{'corpus':{'root':str(cache/'corpora'),'source_commit':'commit','tasks_tree':'tree'},
             'tasks':{task:{'task_package':str(cache/task),'profile':{'path':str(cache/task),'sha256':sha(cache/task)},
                             'task_package_manifest_sha256':'3'*64} for task in doc['selection']['task_ids']},
             'verifier_executable':{'path':str(cache/'uv'),'sha256':sha(cache/'uv')},
             'runner_project':{'path':str(cache/'runner'),'source_manifest_sha256':'5'*64}, 'verifier_env':{}}}})
    print('{}')
elif command=='validate': print('experiment synthetic resolved: digest='+manifest(config())+' native_integrity='+'a'*64)
elif command=='plan':
    doc=config(); source=Path(flag('--config')).read_text()
    write(flag('--out'),{'schema':'opi-eval-run-plan/2','experiment':{'source':source,'manifest_digest':manifest(doc)},
         'material':{'path':flag('--native-material'),'manifest_sha256':sha(flag('--native-material'))},
         'selection':doc['selection'],'trials':trials(doc),'corpus':corpus(doc),'plan_digest':'b'*64})
    print('{}')
elif command=='run':
    if '--plan' in sys.argv:
        plan=read(flag('--plan'));doc=tomllib.loads(plan['experiment']['source']); planned=plan['trials']; plan_digest=plan['plan_digest']
    else: doc=config();planned=trials(doc);plan_digest=None
    root=Path(flag('--root')); root.mkdir(parents=True)
    if not os.environ.get('CUSTOM_AUTH'): raise SystemExit('synthetic live selected credential absent')
    contract=dict(doc,manifest_digest=manifest(doc),trials=planned)
    write(root/'run-control.json',{'schema':'opi-eval-run-control/2','experiment_contract':contract,
          'planned_trials':planned,'plan_digest':plan_digest})
    rows=[]
    for trial in planned:
        invalid = doc['subjects'][0]['product']=='opi' and trial['task']=='two' and trial['group']=='two-r2' and '--plan' in sys.argv
        row=dict(trial, outcome={'class':'invalid','reason':'provider-access','source':'settlement'} if invalid else {'class':'graded','source':'settlement'},
                 status='sealed',native_facts={'usage':{'state':'unknown','reason':'synthetic-no-usage'}})
        if not invalid: row['headline']={'reward':'known:0.0(native-trial-result)',
                                       'native_source':{'artifact':'native/trial-result.json','digest':'c'*64}}
        rows.append(row)
        write(root/'trials'/trial['id']/'receipt.json',{'trial':trial['id']})
    report={'schema':'opi-eval-normalized-report/2','outcome':'published','manifest_digest':manifest(doc),
            'integrity_digest':'a'*64,'trials':rows, 'coverage_counts':{'planned':len(planned)}}
    write(root/'native-report.json', report); (root/'native-report.sha256').write_text(sha(root/'native-report.json'))
    print('{}')
elif command=='verify':
    root=Path(flag('--root'))
    if sha(root/'native-report.json')!=(root/'native-report.sha256').read_text(): raise SystemExit('synthetic seal drift')
    print('{}')
elif command=='report':
    root=Path(flag('--root'));out=Path(flag('--out'))
    if (Path.cwd()/'fail-report').exists(): raise SystemExit('synthetic report failure')
    if not out.parent.is_dir(): raise SystemExit('native report requires preexisting destination parent')
    out.write_bytes((root/'native-report.json').read_bytes()); print('{}')
else: raise SystemExit('unsupported synthetic command')
'''

PROOF = r'''
import json,sys,hashlib
from pathlib import Path
def flag(n):return Path(sys.argv[sys.argv.index(n)+1])
out=flag('--out');out.mkdir(parents=True)
material=flag('--material');doc=json.loads(material.read_text())
for product, agent in doc['agents'].items():
    p=out/(product+'.json');p.write_text('{}')
    agent['output_control_evidence']={'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
material.write_text(json.dumps(doc))
'''


class Fixture:
    def __init__(self, root, profile):
        self.root = root
        self.repo = root / "repo"
        self.repo.mkdir()
        self.workspace = root / "workspace"
        self.cache = root / "cache"
        self.index = root / "evidence-roots.jsonl"
        self.index.write_text("")
        self.profile = copy.deepcopy(profile)
        self.profile["benchmarks"]["tb21"]["seeds"] = {}
        self.profile["model"]["credential_env"] = "CUSTOM_AUTH"
        self.profile["storage"] = {"workspace": str(self.workspace), "cache": str(self.cache)}
        self.config = root / "eval.toml"
        self.config.write_text(dumps_toml(self.profile))
        eval_root = self.repo / "crates/opi-eval"
        eval_root.mkdir(parents=True)
        self.fake = self.repo / "companion.py"
        self.fake.write_text(COMPANION)
        self.proof = eval_root / "scripts/native-control-proof.py"
        self.proof.parent.mkdir()
        self.proof.write_text(PROOF)
        (self.repo / "Cargo.toml").write_text('[workspace.package]\nversion="0.8.2"\n')
        (self.repo / "Cargo.lock").write_text("synthetic lock")
        pi = self.cache / "agents/pi"
        (pi / "dist/bundle").mkdir(parents=True)
        for name in ("one", "two", "uv"):
            (self.cache / name).write_bytes(name.encode())
        for name in ("runner", "corpora"):
            (self.cache / name).mkdir()
        (pi / "package.json").write_text('{"version":"1.0.0"}')
        (pi / "node").write_text("synthetic node")
        (pi / "dist/bundle/cli.js").write_text("synthetic bundle")
        (pi / "pi").write_bytes(b'#!/bin/sh\nagent_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$agent_root/node" "$agent_root/dist/bundle/cli.js" "$@"\n')
        write_json(pi / "build-identity.json", {"schema":"opi-eval-agent-identity/1","agent":"pi", "checkout_commit":"commit", "checkout_dirty":False,
                   "package_lock_sha256":"lock", "shrinkwrap_sha256":"shrink", "node_version":"v22.19.0",
                   "node_sha256":sha(pi/"node"), "bundle_sha256":sha(pi/"dist/bundle/cli.js")})
        lock = {"schema":"opi-eval-external-lock/static/1", "authority":{"producers":[]},"tools":[{"id":"node","version":"22.19.0"}],
                "subjects":[{"id":"pi","commit":"commit","model_data_archive":{"version":"1.0.0"},
                             "blobs":[{"path":"packages/coding-agent/package.json","sha256":sha(pi/"package.json")},
                                      {"path":"package-lock.json","sha256":"lock"},
                                      {"path":"packages/coding-agent/npm-shrinkwrap.json","sha256":"shrink"}]}]}
        lock_file = eval_root / "external-locks/static/linux-x86_64.json"
        write_json(lock_file, lock)
        (eval_root / "scripts/scripted-provider.py").write_text("# inert provider")
        for family in ("harbor", "pier"):
            folder = eval_root / "integrations" / family
            folder.mkdir(parents=True)
            (folder / "integration.py").write_text("# inert integration")
        (eval_root / "profiles/benchmarks").mkdir(parents=True)

    def build(self, commands, root, product, companion, repo):
        source = root / "builds/source-before.json"
        write_json(source, {"head":"synthetic-head","files":[],"manifest_sha256":"source"})
        frozen = root / "builds/companion.py"
        shutil.copy2(self.fake, frozen)
        if product == "opi":
            opi = root / "cache/agents/opi/opi"
            opi.parent.mkdir(parents=True)
            opi.write_text("synthetic latest client")
        else:
            opi = None
        identity = {"source":pin(source),"source_manifest_sha256":"source", "companion":pin(frozen), "opi":pin(opi) if opi else None}
        write_json(root / "builds/identity.json", identity)
        return identity

    def patches(self):
        stack = ExitStack()
        original = Commands.run
        def command(instance, argv, cwd, **kwargs):
            argv = list(argv)
            if str(argv[0]).endswith("companion.py"):
                argv = [sys.executable, *argv]
            return original(instance, argv, cwd, **kwargs)
        stack.enter_context(patch("operator_flow.platform_check"))
        stack.enter_context(patch("operator_flow.build", side_effect=self.build))
        stack.enter_context(patch.object(Commands, "run", command))
        stack.enter_context(patch.dict(os.environ, {"CUSTOM_AUTH":"synthetic-secret"}))
        return stack
