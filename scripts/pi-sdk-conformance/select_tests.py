"""Plan offline opi-ai tests from reviewed production paths or actual Rust test consumers."""

import argparse
import json
from pathlib import Path
import re
import subprocess

import check_test_inventory as inventory
from rust_source import module_bindings, module_children, tokens


CARGO_TEST = ["cargo", "test", "--locked", "--offline", "-p", "opi-ai"]
CROSS_CLASSIFIER = (
    "cross_protocol::four_classifier_protocols_share_one_models_dispatch",
    "cross_protocol::http_retry_headers_and_business_validation_keep_distinct_boundaries",
)
PROTECTED_CLASSIFIER_HEADERS = "cross_protocol::classifiers_protected_headers_never_reach_transport"
EXTERNAL_CLASSIFIER = ("classifier_and_image_apis_answer_typed_through_one_instance",)
BUILTIN_CLASSIFIER = ("classifier_and_image_builtins_register", "builtin_classifier_profiles_send")

# The two sides of each binding are reviewed together. An empty tuple runs the whole target.
PRODUCTION = {
    "src/api/openai_completions.rs": {
        "chat_argument_preview": (), "openai_chat_finish_timing": (),
        "openai_chat_fixtures": (), "openai_chat_interleaved": (),
        "openai_chat_output_limits": (), "openai_chat_response_observer": (),
        "openai_choice_usage": (), "openai_compat_lifecycle": (),
        "provider_openai_chat_profile": (),
        "external_sdk": ("assembled_matrix_drives",),
    },
    "src/api/openai_decisions.rs": {
        "classifier_contract": (
            "openai_decisions::", *CROSS_CLASSIFIER, PROTECTED_CLASSIFIER_HEADERS,
            "cross_protocol::classifiers_reused_callbacks_keep_each_calls_payload",
            "cross_protocol::classifiers_response_callback_is_outside_attempt_timeout",
            "cross_protocol::classifiers_response_callbacks_require_successful_json_acquisition",
        ),
        "external_sdk": EXTERNAL_CLASSIFIER,
        "provider_collection": (*BUILTIN_CLASSIFIER, "builtin_openai_oauth_excludes_decisions",
                                "classifier_model_modality_is_rejected"),
        "builtin_auth_ownership": ("custom_classifier_survives_store",),
        "catalog_operations": ("custom_classifier_availability",),
    },
    "src/api/cloudflare_workers_ai_system_one.rs": {
        "classifier_contract": ("cloudflare_workers_ai_system_one::", *CROSS_CLASSIFIER),
        "external_sdk": EXTERNAL_CLASSIFIER,
        "provider_collection": BUILTIN_CLASSIFIER,
    },
    "src/api/llama_cpp_classify.rs": {
        "classifier_contract": (
            "llama_cpp_classify::", PROTECTED_CLASSIFIER_HEADERS, *CROSS_CLASSIFIER,
        ),
        "external_sdk": EXTERNAL_CLASSIFIER,
    },
    # TypeSafe, Cloudflare, Decisions and llama.cpp share validation or attempt helpers here.
    "src/api/system_one.rs": {
        "classifier_contract": (), "external_sdk": EXTERNAL_CLASSIFIER,
        "catalog_operations": (
            "mixed_operation_catalog_reopens", "complete_nonchat_record_protected_header",
            "management_lists_actual_operation_owners", "removal_preserves_captured_image_and_classifier",
            "custom_classifier_availability", "dynamic_unknown_prices", "source_unknown_base",
            "finite_fractional_thresholds",
        ),
        "builtin_auth_ownership": ("custom_classifier_survives_store",),
        "provider_collection": (*BUILTIN_CLASSIFIER, "classifier_same_api_keeps",
                                "builtin_nonstream_operations_reuse",
                                "builtin_openrouter_classifier_sends_images_before_state"),
        "telemetry_operations": ("classifier_calls_freeze_context",),
    },
    "src/providers/mapped.rs": {
        "api_mapped_provider": (), "provider_collection": ("mapped_",),
        "external_sdk": ("assembled_matrix_drives",),
    },
    "src/providers/openai.rs": {
        "provider_openai_chat_profile": (), "openai_compat_lifecycle": (),
        "openai_chat_fixtures": ("openai_chat_provider_", "profile_extra_headers",
                                 "source_compat_chat_frozen_model_controls"),
        "api_mapped_provider": ("provider_headers_", "mapped_provider_dispatches"),
        "provider_collection": ("builtin_mapped_calls",),
        "external_sdk": ("assembled_matrix_drives",),
    },
    "src/providers/openai_responses.rs": {
        "provider_openai_responses_profile": (), "openai_responses_lifecycle": (),
        "openai_responses_fixtures": ("responses_provider_id", "responses_source_defaults",
                                      "models_direct_adapter_rejects_wrong_wire_records"),
        "api_mapped_provider": ("provider_headers_", "mapped_provider_dispatches"),
        "provider_collection": ("builtin_mapped_calls",),
        "external_sdk": ("assembled_matrix_drives",),
    },
    "src/providers/pi_messages.rs": {
        "radius_builtin_catalog": (), "chat_pi_messages": (),
        "provider_pi_messages_profile": (),
        "provider_collection": ("builtin_source_profiles_reach_their_registered_wires",),
        "external_sdk": ("assembled_matrix_drives",),
    },
}
ARCHIVE = "scripts/pi-sdk-conformance/run_protocol_adapters_package.py"
PACKAGE_SLICE = "archive@scripts/pi-sdk-conformance/package_slice.py"
PACKAGE_SLICE_FAMILIES = {
    "http_acceptance": "http",
    "retry_backoff": "http",
    "tls_contract": "crypto",
    "whole_call_retry": "http",
    "ws_upgrade": "ws",
}
# Some callers lie outside Cargo integration binaries.
EXTRA_LIB = {
    "tests/support/ws_peer.rs": ("websocket-compression", "ws::tests::upgrade::"),
}
EXTRA_RUNNERS = {
    "tests/support/ws_peer.rs": f"{PACKAGE_SLICE} --family ws",
    "tests/support/tls_fixtures.rs": f"{PACKAGE_SLICE} --family ws",
}
TEST_FN = re.compile(
    r"#\[(?:tokio::)?test(?:\([^]]*\))?\]\s*(?:#\[[^]]*\]\s*)*(?:async\s+)?fn\s+(\w+)"
)
# Most test files contain no external Rust modules. This prefilter only skips tokenization.
HAS_CHILDREN = re.compile(r"\b(?:mod\b|include\s*!)")


def test_inputs(crate, source):
    """Follow integration roots, their #[path] topics and inline support modules."""
    tests = (crate / "tests").resolve()
    root = (crate / source).resolve()
    pending = [root]
    found = {}
    while pending:
        path = pending.pop()
        if path in found or not path.is_relative_to(tests):
            continue
        # A deleted topic/helper may still be named by a consuming root. Preserve that edge.
        source_text = path.read_text(encoding="utf-8") if path.is_file() else ""
        found[path] = source_text
        if HAS_CHILDREN.search(source_text):
            words, _ = tokens(source_text)
            pending.extend(module_children(path, words, crate_root=path == root))
            for i, word in enumerate(words):
                if (words[i - 3:i] == ["include", "!", "("]
                        and word.startswith("<path>") and word.endswith(".rs")):
                    pending.append((path.parent / word.removeprefix("<path>")).resolve())
    return found


def source_modules(root, files):
    """Return every path bound below a test target and its qualified module path."""
    found = []
    pending = [(root, ())]
    seen = set()
    while pending:
        path, prefix = pending.pop()
        if path in seen:
            continue
        seen.add(path)
        words, _ = tokens(files.get(path, ""))
        for child, module in module_bindings(path, words, crate_root=path == root):
            qualified = (*prefix, *module)
            found.append((child, qualified))
            if child in files:
                pending.append((child, qualified))
    return found


def make_plan(rows, paths, targets, *, full=False, repo=inventory.REPO):
    crate = repo / "crates/opi-ai"
    by_target = {row["target"]: row for row in rows}
    tests = {}
    inputs = {}
    expand = set()
    related = set()
    library = set()

    def sources(row):
        source = row["source"]
        if source not in inputs:
            inputs[source] = test_inputs(crate, source)
            for path in inputs[source]:
                if not path.is_file():
                    expand.add(f"{path.relative_to(crate)}: missing test source referenced by {source}")
        return inputs[source]

    def source_filters(row, path):
        """Narrow a changed topic module without narrowing shared fixtures."""
        root = (crate / row["source"]).resolve()
        if path == root:
            return ()
        files = sources(row)
        for child, module in source_modules(root, files):
            if child != path:
                continue
            if TEST_FN.findall(files.get(path, "")):
                return ("::".join(module) + "::",)
            # A helper fixture has no own assertions. Its change affects every
            # topic that consumes it, so retain the whole target.
            return ()
        return ()

    def select(name, filters, why):
        if name not in by_target:
            raise ValueError(f"unknown inventory target: {name}")
        if filters:
            row = by_target[name]
            files = sources(row)
            names = {case for text in files.values() for case in TEST_FN.findall(text)}
            if any("::" in query for query in filters):
                root = (crate / row["source"]).resolve()
                for path, module in source_modules(root, files):
                    for case in TEST_FN.findall(files.get(path, "")):
                        names.add("::".join((*module, case)))
            for query in filters:
                if not any(query in case for case in names):
                    raise ValueError(f"stale test filter {name}: {query}")
        record = tests.setdefault(name, {"all": False, "filters": set(), "reasons": set()})
        record["all"] |= not filters
        record["filters"].update(filters)
        record["reasons"].add(why)
        entry = by_target[name]["runners"]
        if entry != "-":
            for binding in entry.split():
                if binding == PACKAGE_SLICE and name in PACKAGE_SLICE_FAMILIES:
                    binding += f" --family {PACKAGE_SLICE_FAMILIES[name]}"
                related.add(binding)

    for target in targets:
        select(target, (), "explicit target")
    for changed in paths:
        path = (repo / Path(changed.replace("\\", "/"))).resolve()
        if not path.is_relative_to(crate):
            expand.add(f"{changed}: outside the crate's reviewed test mapping")
            continue
        relative = path.relative_to(crate).as_posix()
        if not path.is_file():
            expand.add(f"{relative}: missing or moved path; verify the replacement and remaining consumers")
        if relative in PRODUCTION:
            for target, filters in PRODUCTION[relative].items():
                select(target, filters, relative)
            related.add("archive@" + ARCHIVE)
            if relative.startswith("src/providers/"):
                related.add("structure@scripts/pi-sdk-conformance/check_architecture_boundaries.py --enforce")
        elif relative == "Cargo.toml" or relative == "src/providers/collection.rs":
            expand.add(f"{relative}: feature selection or builtin composition affects multiple owners")
        elif relative == "tests/targets.tsv":
            related.add("inventory@scripts/pi-sdk-conformance/check_test_inventory.py")
        elif relative == "tests/support/OWNERS.tsv":
            related.add("inventory@scripts/pi-sdk-conformance/check_support_inventory.py")
        elif relative.startswith("tests/") and relative.endswith(".rs"):
            consumers = [row for row in rows if path in sources(row)]
            for row in consumers:
                select(row["target"], source_filters(row, path), relative)
            if relative in EXTRA_LIB:
                library.add(EXTRA_LIB[relative])
            if relative in EXTRA_RUNNERS:
                related.add(EXTRA_RUNNERS[relative])
            if not consumers:
                expand.add(f"{relative}: no resolved Cargo test consumer")
        else:
            expand.add(f"{relative}: reviewed production or test mapping unavailable")

    commands = []
    for target, record in sorted(tests.items()):
        row = by_target[target]
        gates = row["required-features"]
        args = [*CARGO_TEST, "--no-default-features"]
        if gates != "-":
            args.extend(["--features", gates])
        args.extend(["--test", target])
        if not record["all"]:
            args.extend(["--", *sorted(record["filters"])])
        commands.append({"target": target, "args": args, "cases": row["cases"],
                         "reasons": sorted(record["reasons"])})
    for feature, query in sorted(library):
        commands.append({"target": "lib", "args": [*CARGO_TEST, "--no-default-features",
                         "--features", feature, "--lib", "--", query],
                         "reasons": ["tests/support/ws_peer.rs"], "cases": "Private WebSocket upgrade"})
    if full:
        expand.add("explicit broad check")
    return {"scope": "broader checks required" if expand else "focused",
            "expansion": sorted(expand), "tests": commands, "runners": sorted(related)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="changed paths relative to the repository root")
    parser.add_argument("--target", action="append", default=[], help="include an exact Cargo test target")
    parser.add_argument("--full", action="store_true", help="show broad regression commands")
    parser.add_argument("--json", action="store_true", help="emit a machine-readable plan")
    args = parser.parse_args()
    if not (args.paths or args.target or args.full):
        parser.error("pass a changed path, --target NAME or --full")
    try:
        plan = make_plan(inventory.load_inventory(), args.paths, args.target, full=args.full)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        parser.exit(2, f"{exc}\nCannot produce a verified selection. Reconcile the inventory or "
                       "renamed sources, then rerun with --full.\n")
    plan["broad_commands"] = [
        [*CARGO_TEST, "--all-targets"], [*CARGO_TEST, "--no-default-features", "--tests"],
    ] if plan["expansion"] else []
    plan["additional_checks"] = [
        "For feature changes also verify each affected owner in isolation and its package consumer.",
    ] if plan["expansion"] else []
    if args.json:
        print(json.dumps(plan, indent=2))
        return
    print(plan["scope"])
    for entry in plan["tests"]:
        print(f"\n{', '.join(entry['reasons'])} -> {entry['target']} ({entry['cases']})")
        print(" ".join(entry["args"]))
    for reason in plan["expansion"]:
        print(reason)
    for command in plan["broad_commands"]:
        print(" ".join(command))
    for check in plan["additional_checks"]:
        print(check)
    for runner in plan["runners"]:
        print(f"Related runner: {runner}")


if __name__ == "__main__":
    main()
