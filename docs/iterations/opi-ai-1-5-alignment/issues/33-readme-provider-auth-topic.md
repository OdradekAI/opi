# Split the Provider, Catalog and Auth reference from the onboarding README

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Split the cost and pricing reference](32-readme-cost-pricing-topic.md)

## Scope

Provider selection, catalog publication, credential contracts and refresh rules
were spread across the main README and mixed with provider-specific protocol
notes. The split keeps the task-oriented entry points and Core API inventory in
the main manual while giving downstream Rust consumers one bilingual reference
for the shared Provider/Auth boundary.

## Implementation

`crates/opi-ai/docs/provider-auth.md` and `provider-auth.zh.md` now collect the
factory and Provider entry points, credential/OAuth/request-enrichment/refresh
lifecycle, provider behavior and OpenAI-compatible matrices, offline and
dynamic Catalog contracts, and the canonical built-in factory layer. The main
README variants keep stable section anchors and short links to the package-local
topic. Core API and protocol-specific adapter sections remain in the main manual
where they are useful as the broader API index.

## Acceptance

`test_readmes_split_provider_catalog_auth_into_package_local_topic_references`
checks both localized links and representative Provider, Auth, Catalog, OAuth
and refresh symbols in both topic documents. `opi-doc-check.py` validates the
new relative links.

## Limits

This is a documentation and contract-test split. It does not alter Provider
dispatch, credential resolution, catalog persistence, refresh behavior or
feature configuration.
