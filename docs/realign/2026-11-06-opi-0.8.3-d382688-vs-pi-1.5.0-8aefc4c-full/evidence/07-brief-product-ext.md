# Batch EXT/PKG — extensions, packages, model config, MCP integration, diagnostics (Dims 7,12,3,15,17)

pi side: `packages/coding-agent/src/{core/extensions/ (runtime + API),
package-manager.ts, pi-manifest.ts, model-config.ts, model-registry.ts,
model-resolver.ts, model-runtime.ts, provider-composer.ts, virtual-models.ts,
mcp-servers.ts, remote-catalog-provider.ts, radius.ts, diagnostics.ts,
bug-report.ts, bug-report-upload.ts, crash-log.ts, telemetry.ts,
client-identity.ts, runtime-credentials.ts, auth-storage.ts,
auth-guidance.ts, cache-stats.ts, timings.ts, footer-data-provider.ts,
experimental/source-resolver.ts, extensions/ (codemode/, llama/, mcp/,
tool-search/), package-manager-cli.ts}` + docs `extensions.md`,
`custom-provider.md`, `mcp.md`, `packages.md`, `models-configuration.md`,
`providers.md`.

Opi side: `crates/opi-coding-agent/src/{adapter_extension.rs, adapter_host.rs,
adapter_protocol.rs, package_activation.rs, package_cli.rs,
package_discovery.rs, package_resolver.rs, package_store.rs,
runtime_packages.rs, diagnostics.rs, diagnostic_bridge.rs, doctor.rs,
evidence.rs, provider_factory.rs, github_copilot.rs, openai_codex.rs,
native_keyring.rs, credential_store.rs, pricing.rs, theme_discovery.rs}`.

Measure:
1. Extension model: pi extension JS API surface (tools, custom providers,
   renderers, event hooks, themes, keybindings — docs/extensions.md) vs
   `adapter_extension.rs`; every capability mapped or absent.
2. Packages: add/remove/list/doctor flows, manifest format, install locks,
   trust, lifecycle scripts vs opi `package_*` + CLI subcommands.
3. Model config: user models.json (custom models, overrides, API mappings),
   provider composer, virtual models, remote catalog vs opi
   `provider_factory.rs`/TOML model config.
4. MCP product integration: `mcp-servers.ts` (config discovery, server
   lifecycle, tool exposure) vs opi MCP (run absence search:
   `rg -li 'mcp' crates/*/src` first; earlier probe found no obvious module).
5. Built-in extensions: codemode tool, tool-search, llama.cpp vs opi
   equivalents (absence searches).
6. Diagnostics: bug-report, crash-log, telemetry wiring, client-identity vs
   `diagnostics.rs`/`doctor.rs`/`evidence.rs`.

Output `evidence/product-ext.md`, prefixes `EXT`, `PKG`. Tallies + coverage
honesty; exact absence-search commands.
