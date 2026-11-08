# Add task-oriented README navigation

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Exercise README onboarding in both public feature profiles](26-readme-package-profiles.md)

## Scope

The independent-consumer README must expose the supported integration path
before the long provider and protocol reference. Navigation is part of the
public contract because a complete API manual is not useful if a new consumer
cannot find the first working path.

## Implementation

Both localized READMEs now begin with a task-oriented guide linking installation,
ordinary calls, streaming, context/tools, model/auth setup, providers,
telemetry, and the test/package proof. The guide explicitly separates the
consumer path from later capability and task reference sections.

## Acceptance

`test_readmes_expose_task_oriented_integration_guide` requires every guide entry
in both localized manuals. The existing packaged proof remains the executable
validation for the linked onboarding examples.

## Limits

This slice improves discoverability and section boundaries; it does not claim
that every provider reference has been rewritten or that all Pi surface area is
already implemented.
