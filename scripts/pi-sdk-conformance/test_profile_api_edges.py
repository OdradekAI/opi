import tempfile
import unittest
from pathlib import Path

from profile_api_edges import PROFILES, profile_api_edges, scan_profile


class ProfileApiBoundaryTests(unittest.TestCase):
    def test_multiline_trees_raw_identifiers_aliases_and_globs(self):
        rows = scan_profile("""use crate::{
            provider::operation::SimpleOptions,
            r#api::{openai_completions::{self as wire, OpenAiToolChoice as Choice}, shared::*},
        };""", "mapped.rs")
        self.assertEqual({row["path"] for row in rows}, {
            "crate::api::openai_completions", "crate::api::openai_completions::OpenAiToolChoice", "crate::api::shared::*",
        })
        self.assertTrue(all(row["kind"] == "use" and row["line"] == 3 and not row["permission"]
                            for row in rows))

    def test_relative_paths_and_unreviewable_crate_alias(self):
        source = """use crate as sdk;
            mod nested { type A = super::super::super::api::openai_completions::Native; }
            use super::super::*;"""
        self.assertEqual([r["path"] for r in scan_profile(source, "openai.rs")],
                         ["crate", "crate::api::openai_completions::Native", "crate::*"])

    def test_grouped_accounting_export_has_a_closed_symbol_list(self):
        source = """pub use crate::api::shared::openrouter_usage::{
            OpenRouterCostDetails, OpenRouterUsageAccounting,
            apply_openrouter_reported_cost, Extra, OpenRouterCostDetails as Renamed
        };
        use crate::api::shared::openrouter_usage::OpenRouterCostDetails;
        pub(crate) use crate::api::shared::openrouter_usage::OpenRouterCostDetails;
        fn f() { let _ = crate::api::shared::openrouter_usage::apply_openrouter_reported_cost; }
        mod nested { pub use crate::api::shared::openrouter_usage::OpenRouterCostDetails; }"""
        rows = scan_profile(source, "openrouter.rs")
        self.assertEqual(len(rows), 9)
        self.assertEqual(sum(bool(r["permission"]) for r in rows), 3)
        for file in ("other.rs", "nested/openrouter.rs"):
            self.assertFalse(any(r["permission"] for r in scan_profile(source, file)))

    def test_constructor_and_dyn_registration_are_the_only_collection_uses(self):
        source = """fn register() {
            let _ = crate::api::openai_decisions::OpenAiDecisionsApi::new(args);
            let _: Box<dyn crate::api::ClassifierApi<O>>;
            let _ = crate::api::openai_decisions::OpenAiDecisionsApi::other();
            let _ = crate::api::openai_decisions::OpenAiDecisionsApi::new;
            let _ = crate::api::openai_completions::OpenAiCompletionsOptions::default();
            type Other = crate::api::ClassifierApi<O>;
        }
        use crate::api::openai_decisions::OpenAiDecisionsApi as Api;"""
        rows = scan_profile(source, "collection.rs")
        self.assertEqual([(r["line"], r["permission"]) for r in rows if r["permission"]],
                         [(2, "construct builtin adapter"), (3, "register typed classifier")])
        self.assertEqual(len(rows), 7)
        self.assertFalse(any(r["permission"] for r in scan_profile(source, "mapped.rs")))

    def test_comments_literals_and_cfg_test_leave_production_lines(self):
        src = '''//! crate::api::Fake
            /* before /* inner */ crate::api::Fake */
            #[doc = r###"quote "#, crate::api::Fake"###]
            const BYTES: &[u8] = br#"crate::api::Fake"#;
            const S: &str = "escaped \\" // crate::api::Fake";
            #[cfg(all(feature = "adapter", test))]
            #[allow(dead_code)]
            mod only_test { use crate::api::Fake; }
            #[cfg(test)] use crate::{api::{One, Two}};
            #[cfg(any(test, feature = "live"))]
            fn real() { crate::api::Real::new(); }
            #[cfg(not(test))] fn other() { crate::api::Other(); }'''
        rows = scan_profile(src, "p.rs")
        self.assertEqual([(r["line"], r["path"]) for r in rows],
                         [(11, "crate::api::Real::new"), (12, "crate::api::Other")])
        self.assertEqual(scan_profile("#![cfg(test)]\nuse crate::api::Fake;", "p.rs"), [])

    def test_inner_cfg_stops_only_its_enclosing_module(self):
        src = """mod inner { #![cfg(test)] fn f() { use crate::api::NotSeen; }
                   use crate::api::NotSeenEither; } use crate::api::Seen;"""
        self.assertEqual([r["path"] for r in scan_profile(src, "p.rs")], ["crate::api::Seen"])

    def test_test_attributes_and_hidden_crate_alias(self):
        src = """#[test] fn one() { use crate::api::Hidden; }
            #[tokio::test(flavor = "current_thread")] async fn two() { crate::api::Hidden(); }
            #[cfg(test)] extern crate self as fake;
            extern crate self as sdk;
            fn production() { crate::api::Real(); }"""
        rows = scan_profile(src, "mapped.rs")
        self.assertEqual([(r["kind"], r["path"]) for r in rows],
                         [("extern crate", "crate"), ("qualified", "crate::api::Real")])
        self.assertTrue(all(r["permission"] is None for r in rows))

    def test_physical_test_module_reachability(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            root = repo / PROFILES
            sources = {
                "mod.rs": "mod main; mod inline { #[cfg(test)] mod cases; } mod live;",
                "main.rs": "#[cfg(test)] pub(crate) mod cases; mod shared;",
                "main/cases.rs": "use crate::api::Unused; mod nested;",
                "main/cases/nested/mod.rs": "mod deeper; use crate::api::Unused;",
                "main/cases/nested/deeper.rs": "use crate::api::Unused;",
                "inline/cases.rs": "use crate::api::Unused;",
                "main/shared.rs": "#[cfg(test)] mod cases; #[cfg(not(test))] mod cases;",
                "main/shared/cases.rs": "use crate::api::StillLive;",
                "live/mod.rs": "mod cases;",
                "live/cases.rs": "use crate::api::SameNameIsLive;",
                "not_yet_declared.rs": "use crate::api::Orphan;",
            }
            for name, source in sources.items():
                p = root / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text(source, encoding="utf-8")
            edges = profile_api_edges(repo)
            self.assertEqual([r["path"] for r in edges],
                             ["crate::api::SameNameIsLive", "crate::api::StillLive", "crate::api::Orphan"])

    def test_alternative_live_path_binding_keeps_physical_file(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            root = repo / PROFILES
            root.mkdir(parents=True)
            (root / "mod.rs").write_text(
                '#[cfg(test)] mod test_only; #[path = "test_only.rs"] pub mod live;', encoding="utf-8")
            (root / "test_only.rs").write_text("use crate::api::Shared;", encoding="utf-8")
            self.assertEqual([r["path"] for r in profile_api_edges(repo)], ["crate::api::Shared"])

    def test_repo_lookup_includes_nested_production_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            root = repo / PROFILES
            (root / "nested").mkdir(parents=True)
            (root / "nested/profile.rs").write_text("use crate::api::Options;", encoding="utf-8")
            [edge] = profile_api_edges(repo)
            self.assertEqual(edge["file"], "crates/opi-ai/src/providers/nested/profile.rs")


if __name__ == "__main__":
    unittest.main()
