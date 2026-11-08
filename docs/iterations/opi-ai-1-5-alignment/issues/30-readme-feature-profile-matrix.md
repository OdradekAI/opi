# Publish the feature profile matrix

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Index the advanced README references](29-readme-reference-map.md)

## Scope

Independent consumers need to know what `default-features = false` changes. The
current crate has one public `opi-ai` feature, so the README must not imply that
the core profile removes providers, network dependencies, or runtime behavior.

## Implementation

Both localized READMEs now publish a small feature profile matrix. It records
the default `telemetry-tracing` profile, the `default-features = false` profile,
and the separate `opi-telemetry` `tracing` feature. It also states that feature
selection changes the optional tracing carrier only; provider registration and
host-owned exporter policy remain unchanged.

## Acceptance

`test_readmes_describe_feature_profiles_without_overclaiming_minimal_builds`
requires the same profile semantics in English and Chinese. The packaged proof
continues to exercise both profiles as real external consumers.

## Limits

This documents the current feature surface. It does not introduce provider
feature gates or claim a no-network, no-Tokio, or zero-dependency build.
