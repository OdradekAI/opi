"""Replay a bound Anthropic candidate without overwriting historical evidence."""
import argparse
import json
import subprocess
from pathlib import Path
from common import REPO_ROOT, file_hash, run_json, runtime_environment
from run_lifecycle_differential import material_tools
from compare_anthropic_usage import compare, projection, require

REVISION = "f9a26365568127461c47a07a37dcb42c140a3ac5"
ITERATION = Path("docs/iterations/opi-ai-1-5-alignment")
DRIVER = ITERATION / "tools/observe_anthropic_usage.mjs"
FIXTURE = Path("crates/opi-ai/tests/data/anthropic-usage-updates.json")
SOURCES = ("packages/ai/src/models.ts", "packages/ai/src/api/anthropic-messages.ts",
           "packages/ai/src/api/lazy.ts", "packages/ai/src/telemetry.ts",
           "packages/agent/src/agent-loop.ts", "packages/durable/src/harness/generation.ts")


def run_rust(build):
    require(file_hash(Path(build["executable"])) == build["sha256"], "sealed executable changed")
    command = [build["executable"],"--exact","anthropic_usage_trace","--nocapture"]
    result = subprocess.run(command,cwd=REPO_ROOT,env=runtime_environment(),capture_output=True,timeout=60)
    require(result.returncode == 0,"Rust trace failed")
    text = result.stdout.decode("utf-8")
    lines = [line for line in text.splitlines() if line.startswith('{"cases":')]
    require(len(lines) == 1, "expected one Rust trace")
    return {"command":command,"exit":result.returncode,"stdout":text,"stderr":result.stderr.decode("utf-8"),"trace":json.loads(lines[0])}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ("materials","pi-repo","before-build","after-build","output"):
        parser.add_argument("--"+name,type=Path,required=True)
    args=parser.parse_args()
    if args.output.exists(): parser.error("refusing to overwrite evidence")
    freezer=material_tools(REPO_ROOT)
    material=json.loads(args.materials.read_bytes())
    require(not material["blocked_gates"],"materials blocked")
    snapshot=Path(material["snapshot"])
    source_hashes={}
    for name in SOURCES:
        original=subprocess.check_output(["git","-C",str(args.pi_repo),"show",f"{REVISION}:{name}"])
        require((snapshot/name).read_bytes()==original,f"frozen source drift: {name}")
        source_hashes[name]=file_hash(snapshot/name)
    before=json.loads(args.before_build.read_bytes()); after=json.loads(args.after_build.read_bytes())
    for name,digest in after["sources"].items(): require(file_hash(REPO_ROOT/name)==digest,f"candidate changed: {name}")
    inputs=[args.materials,args.before_build,args.after_build,REPO_ROOT/DRIVER,REPO_ROOT/FIXTURE,Path(__file__),Path(__file__).with_name("compare_anthropic_usage.py")]
    bound={str(p):file_hash(p) for p in inputs}
    def verify():
        require(not freezer.verify_evidence(REPO_ROOT,material),"material drift")
        require(all(file_hash(Path(p))==h for p,h in bound.items()),"input drift")
    verify()
    pi=[run_json(["node",str(REPO_ROOT/DRIVER),str(snapshot),str(REPO_ROOT/FIXTURE)],REPO_ROOT,60) for _ in range(3)]
    old=[run_rust(before) for _ in range(3)]; current=[run_rust(after) for _ in range(3)]
    for label,runs in (("Pi",pi),("before",old),("after",current)):
        require(all(projection(r["trace"])==projection(runs[0]["trace"]) for r in runs),f"unstable {label} semantics")
    fixtures=json.loads((REPO_ROOT/FIXTURE).read_bytes())
    comparison=compare(fixtures,pi[0]["trace"],current[0]["trace"])
    mismatches=[]
    cases={c["id"]:c for c in fixtures}
    for row in old[0]["trace"]["cases"]:
        expected=cases[row["id"]]["expected"]
        cost=sum(n*(i+1)/1e6 for i,n in enumerate(expected))
        if row["result"]["usage"]!=expected or abs(row["result"]["cost"]-cost)>1e-12 or (row["ending"]=="success" and row["delivery"]!="ok"):
            mismatches.append([row["id"],row["ending"],row["route"]])
    require(mismatches,"before repair must reproduce the defect")
    verify()
    freezer.write_new_json(args.output,{"schemaVersion":1,"revision":REVISION,"pi_sources":source_hashes,"inputs":bound,
        "before_build":before,"after_build":after,"before_mismatches":mismatches,"comparison":comparison,
        "pi_runs":pi,"before_runs":old,"after_runs":current,
        "scope":"Anthropic accounting/presence/observer slice, not full adapter equality; raw duration samples retained"})
    print(json.dumps({"status":comparison["status"],"before_mismatches":len(mismatches),"rust_cases":comparison["rust_routes"]}))

if __name__=="__main__":main()
