"""Single editable operator profile -> sanitized resolution -> Companion TOML."""
from __future__ import annotations
import copy
import json
import math
from pathlib import Path, PurePosixPath
import os
import re
import tomllib
from urllib.parse import urlsplit

from operator_process import OperatorError, write_bytes

BENCHMARKS = {"tb21": ("terminal-bench", "2.1", "terminal-bench-2.1"),
              "tb30": ("terminal-bench", "3.0", "terminal-bench-3.0"),
              "deepswe": ("deepswe", "v1.1", "deepswe-v1.1")}
TEMPLATE = Path(__file__).with_name("config.example.toml")
PRIVATE_ENV = "OPI_EVAL_OPERATOR_KEY"
MAX_LOGICAL_TRIALS = 10000


def init_profile(path):
    write_bytes(path, TEMPLATE.read_bytes())


def load(path):
    try:
        with Path(path).open("rb") as stream:
            return tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise OperatorError("cannot read a valid operator TOML profile") from exc


def table(value, keys, name, required=()):
    if not isinstance(value, dict) or set(value) - set(keys) or set(required) - set(value):
        raise OperatorError(f"invalid or missing fields in {name}")


def positive(value, name, zero=False):
    if type(value) is not int or value < (0 if zero else 1) or value > 2**63 - 1:
        raise OperatorError(f"{name} must be {'nonnegative' if zero else 'positive'} integer")


def text(value, name):
    if not isinstance(value, str) or not value.strip() or any(ord(c) < 32 for c in value):
        raise OperatorError(f"{name} must be a nonempty single-line string")


def overrides(profile, values):
    result = copy.deepcopy(profile)
    for expression in values:
        try:
            name, raw = expression.split("=", 1)
            components = name.split(".")
            if any(component in {"api_key", "credential_env", "credential_file"} for component in components):
                raise OperatorError("credential overrides belong only in the private profile")
            value = tomllib.loads("value=" + raw)["value"]
            target = result
            for component in components[:-1]:
                target = target[component]
            if components[-1] not in target:
                raise OperatorError("override names an unknown configuration field")
            if isinstance(target[components[-1]], dict) or isinstance(value, dict):
                raise OperatorError("overrides select scalar or list leaves; table replacement is refused")
            target[components[-1]] = value
        except (KeyError, TypeError, ValueError, tomllib.TOMLDecodeError) as exc:
            raise OperatorError("invalid override; use an existing dotted key=TOML_VALUE") from exc
    return result


def resolve(profile, explicit=()):
    p = overrides(profile, explicit)
    keys = {"schema", "model", "execution", "benchmarks", "baseline", "storage"}
    version = p.get("schema")
    if version == "opi-eval-operator/2":
        keys.add("retention")
    table(p, keys, "profile", keys)
    if not isinstance(version, str) or version not in {"opi-eval-operator/1", "opi-eval-operator/2"}:
        raise OperatorError("unsupported operator profile schema")
    model = p["model"]
    table(model, {"provider", "model", "api", "base_url", "credential_env", "credential_file", "api_key", "controls", "thinking"},
          "model", {"provider", "model", "api", "base_url", "controls", "thinking"})
    for name in ("provider", "model", "api", "base_url"):
        text(model[name], "model." + name)
    if model["api"] not in {"anthropic-messages", "openai-completions", "openai-responses"}:
        raise OperatorError("unsupported model API")
    try:
        url = urlsplit(model["base_url"])
        valid = url.scheme in {"http", "https"} and url.hostname and not url.username and not url.password and not url.query and not url.fragment
        _ = url.port
    except ValueError:
        valid = False
    if not valid:
        raise OperatorError("model endpoint must be an HTTP(S) URL without credentials, query or fragment")
    credentials = [name for name in ("credential_env", "credential_file", "api_key") if name in model]
    if len(credentials) != 1:
        raise OperatorError("model requires exactly one credential source")
    credential = credentials[0]
    text(model[credential], "credential reference")
    if credential == "credential_env" and not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", model[credential]):
        raise OperatorError("credential_env must be an environment variable name")
    if credential == "credential_file" and not (Path(model[credential]).is_absolute() or PurePosixPath(model[credential]).is_absolute()):
        raise OperatorError("credential_file must be absolute")
    controls = model["controls"]
    table(controls, {"context_window", "max_output_tokens", "temperature", "top_p"}, "model.controls",
          {"context_window", "max_output_tokens", "temperature", "top_p"})
    for name in ("context_window", "max_output_tokens"):
        positive(controls[name], "model.controls." + name)
    for name, ceiling in (("temperature", 2), ("top_p", 1)):
        value = controls[name]
        if value not in ("omitted", "unknown") and (type(value) not in (int, float) or not math.isfinite(value) or not 0 <= value <= ceiling):
            raise OperatorError("invalid sampling control")
        if type(value) in (int, float):
            # Companion sampling controls require TOML floats, including whole-number values.
            controls[name] = float(value)
    table(model["thinking"], {"level", "required"}, "model.thinking", {"level", "required"})
    if not isinstance(model["thinking"]["level"], str) or model["thinking"]["level"] not in {"off", "minimal", "low", "medium", "high", "xhigh", "max", "omitted"}:
        raise OperatorError("unknown thinking level")
    if type(model["thinking"]["required"]) is not bool or (model["thinking"]["required"] and model["thinking"]["level"] == "omitted"):
        raise OperatorError("required thinking must have an explicit supported level")
    execution = p["execution"]
    required = {"trial_retries", "batch_timeout_seconds", "concurrency", "capture_policy"}
    table(execution, required, "execution", required)
    positive(execution["trial_retries"], "execution.trial_retries", zero=True)
    for name in ("batch_timeout_seconds", "concurrency"):
        positive(execution[name], "execution." + name)
    if execution["concurrency"] > 2**32 - 1:
        raise OperatorError("execution.concurrency exceeds the Companion u32 contract")
    if not isinstance(execution["capture_policy"], str) or execution["capture_policy"] not in {"full", "terminal-only"}:
        raise OperatorError("unsupported capture policy")
    table(p["benchmarks"], BENCHMARKS, "benchmarks")
    if not p["benchmarks"]:
        raise OperatorError("at least one benchmark configuration is required")
    for bench, selected in p["benchmarks"].items():
        keys = {"scope", "repetitions", "task_ids", "seeds"}
        table(selected, keys, "benchmarks." + bench, keys)
        positive(selected["repetitions"], "benchmark repetitions")
        if selected["repetitions"] > 2**32 - 1:
            raise OperatorError("benchmark repetitions exceeds the Companion u32 contract")
        ids = selected["task_ids"]
        if not isinstance(ids, list) or any(not isinstance(item, str) or not item for item in ids) or len(ids) != len(set(ids)):
            raise OperatorError("task_ids must be unique nonempty strings")
        if not isinstance(selected["scope"], str) or selected["scope"] not in {"full", "subset"} or (selected["scope"] == "full" and ids) or (selected["scope"] == "subset" and not ids):
            raise OperatorError("full scope has no task_ids; subset requires task_ids")
        if selected["scope"] == "subset" and len(ids) * selected["repetitions"] > MAX_LOGICAL_TRIALS:
            raise OperatorError("operator plan exceeds its 10000 logical-trial admission bound")
        if not isinstance(selected["seeds"], dict):
            raise OperatorError("seeds must be a task-to-integer table")
        for task, seed in selected["seeds"].items():
            if selected["scope"] == "subset" and task not in ids:
                raise OperatorError("seed task outside selected subset")
            positive(seed, "seed", zero=True)
    baseline_keys = {"pi_version", "lookup"}
    if version == "opi-eval-operator/2":
        baseline_keys.add("max_age_days")
    table(p["baseline"], baseline_keys, "baseline", baseline_keys)
    text(p["baseline"]["pi_version"], "baseline.pi_version")
    if not isinstance(p["baseline"]["lookup"], str) or p["baseline"]["lookup"] not in {"auto", "none"}:
        raise OperatorError("baseline.lookup must be auto or none")
    if version == "opi-eval-operator/2":
        positive(p["baseline"]["max_age_days"], "baseline.max_age_days", zero=True)
        if p["baseline"]["max_age_days"] > 36500:
            raise OperatorError("baseline.max_age_days exceeds the 100-year bound")
        retention = p["retention"]
        fields = {"keep_latest_per_group"}
        table(retention, fields, "retention", fields)
        positive(retention["keep_latest_per_group"], "retention.keep_latest_per_group")
    table(p["storage"], {"workspace", "cache"}, "storage", {"workspace", "cache"})
    for name in ("workspace", "cache"):
        text(p["storage"][name], "storage." + name)
        value = p["storage"][name]
        path = PurePosixPath(value) if os.name == "nt" and value.startswith("/") else Path(value)
        if not path.is_absolute():
            raise OperatorError("storage paths must be absolute")
        p["storage"][name] = str(path) if isinstance(path, PurePosixPath) and not isinstance(path, Path) else str(path.resolve())
    workspace, cache = (PurePosixPath(p["storage"][name]) if p["storage"][name].startswith("/") else Path(p["storage"][name]) for name in ("workspace", "cache"))
    if workspace == cache or workspace in cache.parents or cache in workspace.parents:
        raise OperatorError("storage workspace and cache must be disjoint directories")
    if credential == "api_key":
        del model["api_key"]
        p["credential"] = {"kind": "private-profile"}
    else:
        p["credential"] = {"kind": credential, "reference": model.pop(credential)}
    p["overrides"] = list(explicit)
    return p


def retention_policy(profile):
    """Legacy profiles use the maintained template without rewriting their bytes."""
    return copy.deepcopy(profile["retention"] if "retention" in profile else load(TEMPLATE)["retention"])


def experiment(p, bench, product, version, identity, cache, root):
    if bench not in p["benchmarks"]:
        raise OperatorError("benchmark missing from operator profile")
    family, revision, adapter = BENCHMARKS[bench]
    credential = p["credential"]
    connection = {"id": "selected-model", **{key: p["model"][key] for key in ("provider", "model", "api", "base_url")}}
    connection[credential["kind"] if credential["kind"] != "private-profile" else "credential_env"] = credential.get("reference", PRIVATE_ENV)
    controls = {**p["model"]["controls"], "reasoning": p["model"]["thinking"]["level"],
                "required": ["context_window", "max_output_tokens"]}
    if p["model"]["thinking"]["required"]:
        controls["required"].append("reasoning")
    return {"schema": "opi-eval-experiment/2", "experiment_id": identity, "connection": "selected-model",
            "benchmark": {"name": family, "revision": revision, "dataset": adapter},
            "subjects": [{"id": product + "-subject", "product": product, "version": version}],
            "connections": [connection], "model_controls": controls,
            "selection": {**p["benchmarks"][bench], "reason": "operator-profile selection"},
            "execution": {"mode": "live", "concurrency": p["execution"]["concurrency"],
                          "max_output_tokens": controls["max_output_tokens"]},
            "storage": {"cache": str(cache), "results": str(Path(root) / "runs"), "capture_policy": p["execution"]["capture_policy"]},
            "environment": {"platform": "linux", "architecture": "x86_64", "cwd_policy": "isolated"}}


def dumps_toml(document):
    lines = []
    def value(item):
        if isinstance(item, bool):
            return "true" if item else "false"
        if isinstance(item, str):
            return json.dumps(item, ensure_ascii=False)
        if isinstance(item, (int, float)):
            return str(item)
        if isinstance(item, list):
            return "[" + ", ".join(value(entry) for entry in item) + "]"
        raise OperatorError("unsupported TOML field type")
    def emit(table_value, path=(), array=False):
        if path:
            title = ".".join(json.dumps(item) for item in path)
            lines.extend(["", ("[[" if array else "[") + title + ("]]" if array else "]")])
        for key, item in table_value.items():
            if not isinstance(item, dict) and not (isinstance(item, list) and item and isinstance(item[0], dict)):
                lines.append(json.dumps(key) + " = " + value(item))
        for key, item in table_value.items():
            if isinstance(item, dict):
                emit(item, path + (key,))
            elif isinstance(item, list) and item and isinstance(item[0], dict):
                for entry in item:
                    emit(entry, path + (key,), True)
    emit(document)
    return "\n".join(lines) + "\n"
