"""Execute nested Anthropic usage contract from actual SDK/telemetry archives."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import run_early_package_proof as shared
from package_slice import check_cargo_configs, controlled_environment
from run_readme_examples import check_opi_feature_profile

REPO=Path(__file__).resolve().parents[2]


EXPECTED_TESTS = {
    "nested_cache_and_reasoning_usage_reaches_the_canonical_message",
    "reasoning_delta_subset_validation_is_atomic",
    "merged_delta_validation_is_atomic_and_cost_zero_has_authority",
    "anthropic_usage_trace",
    "anthropic_usage_contract",
}


def check_execution(stdout):
    observed = set(re.findall(r"^test ([A-Za-z0-9_]+) \.\.\. ok$", stdout, re.MULTILINE))
    missing = EXPECTED_TESTS - observed
    if missing or not re.search(r"test result: ok\. \d+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;", stdout):
        raise ValueError(f"named Anthropic contract tests did not all execute: {sorted(missing)}")


def check_telemetry_feature(metadata, telemetry_id, defaults):
    node=next(n for n in metadata["resolve"]["nodes"] if n["id"]==telemetry_id)
    if ("tracing" in node["features"]) != defaults:
        raise ValueError("companion telemetry tracing profile was not exercised")
    return sorted(node["features"])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ("scratch","target-dir","evidence-output"):parser.add_argument("--"+name,type=Path,required=True)
    args=parser.parse_args();scratch=args.scratch.resolve();target=args.target_dir.resolve();output=args.evidence_output.resolve()
    if output.exists():parser.error("refusing to overwrite evidence")
    shared.validate_paths(REPO,scratch,target)
    env=controlled_environment(dict(os.environ),target);env.setdefault("CARGO_BUILD_JOBS","4")
    configs=check_cargo_configs(env,[REPO,scratch/"consumer",scratch/"package"])
    scratch.mkdir(parents=True)
    evidence={"verdict":"FAIL","scope":"Nested Anthropic usage archive consumer with builtins/no-tracing and defaults", "steps":[],"profiles":[],"cargo_configs":configs}
    candidate=None
    def run(name,command,cwd):
        if candidate is not None:command=candidate.command(command)
        start=time.monotonic();r=subprocess.run(command,cwd=cwd,env=env,capture_output=True,timeout=3600)
        row={"name":name,"command":command,"exit":r.returncode,"seconds":round(time.monotonic()-start,2)}
        for label in ("stdout","stderr"):
            p=scratch/(name+"."+label);p.write_bytes(getattr(r,label));row[label]={"path":str(p),"sha256":hashlib.sha256(p.read_bytes()).hexdigest()}
        evidence["steps"].append(row)
        if r.returncode:raise ValueError(f"{name} failed; see {scratch}")
        return r.stdout.decode("utf-8")
    lease=[sys.executable,str(REPO/"scripts/opi-cargo-cache.py"),"lease"]
    leased=False
    try:
        run("lease-start",lease+["start","--target",str(target),"--pid",str(os.getpid())],REPO);leased=True
        package,evidence["package"],candidate=shared.package_sdk(run,scratch,target)
        consumer=scratch/"consumer";(consumer/"tests/data").mkdir(parents=True)
        files=["tests/anthropic_usage_updates.rs","tests/anthropic_nested_usage.rs","tests/data/anthropic-usage-updates.json"]
        evidence["executed_sources"]={}
        for name in files:
            src=package/name; require=REPO/"crates/opi-ai"/name
            if src.read_bytes()!=require.read_bytes():raise ValueError("archive source mismatch")
            shutil.copyfile(src,consumer/name);evidence["executed_sources"][name]=hashlib.sha256(src.read_bytes()).hexdigest()
        manifest=('[package]\nname="opi-anthropic-consumer"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\n'
                  'opi-ai={path='+json.dumps(package.as_posix())+',default-features=false,features=["builtins"]}\n'
                  'opi-telemetry={version="=0.8.3",default-features=false}\n'
                  'serde_json="=1.0.149"\nsecrecy="=0.10.3"\nfutures-util="=0.3.32"\nwiremock="=0.6.5"\n'
                  'tokio={version="=1.52.3",features=["macros","rt-multi-thread","sync","time"]}\n'
                  '[features]\ncaller-defaults=["opi-ai/default"]\n')
        (consumer/"Cargo.toml").write_text(manifest,encoding="utf-8",newline="\n")
        run("lock",["cargo","generate-lockfile","--offline"],consumer)
        for defaults in (False,True):
            name="defaults" if defaults else "builtins-no-tracing";flags=["--features","caller-defaults"] if defaults else []
            metadata=json.loads(run(name+"-metadata",["cargo","metadata","--offline","--locked","--format-version","1",*flags],consumer))
            telemetry_id=candidate.verify_graph(metadata)
            telemetry_features=check_telemetry_feature(metadata,telemetry_id,defaults)
            evidence["profiles"].append({"name":name,**check_opi_feature_profile(metadata,package,defaults),"telemetry_features":telemetry_features})
            local=[p for p in metadata["packages"] if p.get("source") is None]
            if {p["name"] for p in local}!={"opi-ai","opi-telemetry","opi-anthropic-consumer"}:raise ValueError("unexpected local dependency")
            run(name+"-clippy",["cargo","clippy","--offline","--locked","--tests",*flags,"--","-D","warnings"],consumer)
            stdout=run(name+"-updates-tests",["cargo","test","--offline","--locked","--test","anthropic_usage_updates",*flags,"--","--nocapture"],consumer)
            check_execution(stdout)
            nested=run(name+"-nested-tests",["cargo","test","--offline","--locked","--test","anthropic_nested_usage",*flags,"--","--nocapture"],consumer)
            if not re.search(r"test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",nested):
                raise ValueError("nested Anthropic usage contract test did not execute")
        evidence["verdict"]="PASS"
    finally:
        if leased:run("lease-end",lease+["end","--target",str(target),"--pid",str(os.getpid())],REPO)
        evidence["runner_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        output.parent.mkdir(parents=True,exist_ok=True)
        with output.open("x",encoding="utf-8") as f:json.dump(evidence,f,indent=2)
    print(json.dumps({"verdict":evidence["verdict"],"profiles":len(evidence["profiles"])}))

if __name__=="__main__":main()
