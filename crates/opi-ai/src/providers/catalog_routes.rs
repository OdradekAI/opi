//! Operation routes installed by the builtin collection and checked during catalog authoring.

pub(super) const CLASSIFIERS: &[(&str, &str)] = &[
    ("typesafe", "typesafe-system-one"),
    ("openrouter", "typesafe-system-one"),
    ("opencode", "typesafe-system-one"),
    ("vercel-ai-gateway", "typesafe-system-one"),
    ("openai", "openai-decisions"),
    ("cloudflare-workers-ai", "cloudflare-workers-ai-system-one"),
];

pub(super) const IMAGE: (&str, &str) = ("openrouter", "openrouter-images");
