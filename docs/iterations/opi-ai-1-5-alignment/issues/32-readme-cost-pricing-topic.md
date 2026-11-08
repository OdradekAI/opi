# Split the cost and pricing reference from the onboarding README

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Index advanced README references](29-readme-reference-map.md)

## Scope

The README's cost summary was useful for onboarding, but its estimator,
sampling, pricing-tier, cache-pricing, and service-tier tables were long enough
to compete with the first-call path. The split must preserve the old section
anchors and exact semantics while giving external consumers a package-local
reference they can read independently.

## Implementation

`crates/opi-ai/docs/cost-pricing.md` and `cost-pricing.zh.md` now contain the
detailed estimator and pricing metadata reference. Both README variants keep
the short unknown-cost/no-blocking summary, retain the stable estimator heading,
and link the full reference from the advanced capability index and the summary.
The topic documents also identify the public values and calculation entry point
without requiring a workspace-only path.

## Acceptance

`test_readmes_split_cost_pricing_into_package_local_topic_references` checks
both localized README links, the retained summary, the package-local topic
files, and representative estimator/pricing rules including
`calculate_cost_tiered` and `PricingTier::input_tokens_above`.

## Limits

This is a documentation and contract-test split. It does not change pricing
implementation, usage accumulation, provider billing rules, or the release
acceptance policy.
