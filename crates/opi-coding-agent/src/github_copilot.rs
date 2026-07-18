//! Offline GitHub Copilot catalog pinned to pi 0.80.6.

use opi_ai::model_info::{AnthropicMessagesCompat, OpenAiCompletionsCompat, OpenAiResponsesCompat};
use opi_ai::{
    ModelCapabilities, ModelInfo, ModelPricing, Pricing, ThinkingLevel, ThinkingLevelMap,
    ThinkingLevelMapping, WireApi, WireCompat,
};

pub const GITHUB_COPILOT_DEFAULT_BASE_URL: &str = "https://api.individual.githubcopilot.com";

/// Headers pinned by the reviewed GitHub Copilot Chat profile.
pub fn github_copilot_static_headers() -> Vec<(String, String)> {
    [
        ("User-Agent", "GitHubCopilotChat/0.35.0"),
        ("Editor-Version", "vscode/1.107.0"),
        ("Editor-Plugin-Version", "copilot-chat/0.35.0"),
        ("Copilot-Integration-Id", "vscode-chat"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect()
}

#[derive(Clone, Copy)]
enum Compat {
    Anthropic {
        eager_tool_input: bool,
        adaptive_thinking: bool,
        temperature: bool,
    },
    Chat,
    Responses,
}

struct CatalogRecord {
    id: &'static str,
    name: &'static str,
    wire: WireApi,
    context_window: u64,
    max_tokens: u64,
    images: bool,
    reasoning: bool,
    thinking: &'static [(ThinkingLevel, Option<&'static str>)],
    compat: Compat,
    pricing: (f64, f64, f64, f64),
}

const NONE: &[(ThinkingLevel, Option<&str>)] = &[];
const MAX: &[(ThinkingLevel, Option<&str>)] = &[(ThinkingLevel::Max, Some("max"))];
const MINIMAL_LOW_MAX: &[(ThinkingLevel, Option<&str>)] = &[
    (ThinkingLevel::Minimal, Some("low")),
    (ThinkingLevel::Max, Some("max")),
];
const XHIGH_MAX: &[(ThinkingLevel, Option<&str>)] = &[
    (ThinkingLevel::XHigh, Some("xhigh")),
    (ThinkingLevel::Max, Some("max")),
];
const MINIMAL_LOW_XHIGH_MAX: &[(ThinkingLevel, Option<&str>)] = &[
    (ThinkingLevel::Minimal, Some("low")),
    (ThinkingLevel::XHigh, Some("xhigh")),
    (ThinkingLevel::Max, Some("max")),
];
const OFF_MINIMAL_LOW: &[(ThinkingLevel, Option<&str>)] = &[
    (ThinkingLevel::None, None),
    (ThinkingLevel::Minimal, Some("low")),
];
const OFF_MINIMAL_LOW_XHIGH: &[(ThinkingLevel, Option<&str>)] = &[
    (ThinkingLevel::None, None),
    (ThinkingLevel::Minimal, Some("low")),
    (ThinkingLevel::XHigh, Some("xhigh")),
];

const CHAT: Compat = Compat::Chat;
const RESPONSES: Compat = Compat::Responses;
const ANTHROPIC: Compat = Compat::Anthropic {
    eager_tool_input: true,
    adaptive_thinking: false,
    temperature: true,
};
const ANTHROPIC_NO_EAGER: Compat = Compat::Anthropic {
    eager_tool_input: false,
    adaptive_thinking: false,
    temperature: true,
};
const ANTHROPIC_ADAPTIVE: Compat = Compat::Anthropic {
    eager_tool_input: true,
    adaptive_thinking: true,
    temperature: true,
};
const ANTHROPIC_ADAPTIVE_NO_TEMPERATURE: Compat = Compat::Anthropic {
    eager_tool_input: true,
    adaptive_thinking: true,
    temperature: false,
};

const RECORDS: &[CatalogRecord] = &[
    CatalogRecord {
        id: "claude-fable-5",
        name: "Claude Fable 5",
        wire: WireApi::OpenAiCompletions,
        context_window: 1_000_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (10.0, 50.0, 1.0, 12.5),
    },
    CatalogRecord {
        id: "claude-haiku-4.5",
        name: "Claude Haiku 4.5 (latest)",
        wire: WireApi::AnthropicMessages,
        context_window: 200_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: ANTHROPIC_NO_EAGER,
        pricing: (1.0, 5.0, 0.1, 1.25),
    },
    CatalogRecord {
        id: "claude-opus-4.5",
        name: "Claude Opus 4.5 (latest)",
        wire: WireApi::AnthropicMessages,
        context_window: 200_000,
        max_tokens: 32_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: ANTHROPIC,
        pricing: (5.0, 25.0, 0.5, 6.25),
    },
    CatalogRecord {
        id: "claude-opus-4.6",
        name: "Claude Opus 4.6",
        wire: WireApi::AnthropicMessages,
        context_window: 1_000_000,
        max_tokens: 32_000,
        images: true,
        reasoning: true,
        thinking: MAX,
        compat: ANTHROPIC_ADAPTIVE,
        pricing: (5.0, 25.0, 0.5, 6.25),
    },
    CatalogRecord {
        id: "claude-opus-4.7",
        name: "Claude Opus 4.7",
        wire: WireApi::AnthropicMessages,
        context_window: 1_000_000,
        max_tokens: 32_000,
        images: true,
        reasoning: true,
        thinking: MINIMAL_LOW_XHIGH_MAX,
        compat: ANTHROPIC_ADAPTIVE_NO_TEMPERATURE,
        pricing: (5.0, 25.0, 0.5, 6.25),
    },
    CatalogRecord {
        id: "claude-opus-4.8",
        name: "Claude Opus 4.8",
        wire: WireApi::AnthropicMessages,
        context_window: 1_000_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: MINIMAL_LOW_XHIGH_MAX,
        compat: ANTHROPIC_ADAPTIVE_NO_TEMPERATURE,
        pricing: (5.0, 25.0, 0.5, 6.25),
    },
    CatalogRecord {
        id: "claude-sonnet-4",
        name: "Claude Sonnet 4 (latest)",
        wire: WireApi::AnthropicMessages,
        context_window: 216_000,
        max_tokens: 16_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: ANTHROPIC_NO_EAGER,
        pricing: (3.0, 15.0, 0.3, 3.75),
    },
    CatalogRecord {
        id: "claude-sonnet-4.5",
        name: "Claude Sonnet 4.5 (latest)",
        wire: WireApi::AnthropicMessages,
        context_window: 200_000,
        max_tokens: 32_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: ANTHROPIC_NO_EAGER,
        pricing: (3.0, 15.0, 0.3, 3.75),
    },
    CatalogRecord {
        id: "claude-sonnet-4.6",
        name: "Claude Sonnet 4.6",
        wire: WireApi::AnthropicMessages,
        context_window: 1_000_000,
        max_tokens: 32_000,
        images: true,
        reasoning: true,
        thinking: MINIMAL_LOW_MAX,
        compat: ANTHROPIC_ADAPTIVE,
        pricing: (3.0, 15.0, 0.3, 3.75),
    },
    CatalogRecord {
        id: "claude-sonnet-5",
        name: "Claude Sonnet 5",
        wire: WireApi::AnthropicMessages,
        context_window: 1_000_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: XHIGH_MAX,
        compat: ANTHROPIC_ADAPTIVE,
        pricing: (2.0, 10.0, 0.2, 2.5),
    },
    CatalogRecord {
        id: "gemini-2.5-pro",
        name: "Gemini 2.5 Pro",
        wire: WireApi::OpenAiCompletions,
        context_window: 128_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (1.25, 10.0, 0.125, 0.0),
    },
    CatalogRecord {
        id: "gemini-3-flash-preview",
        name: "Gemini 3 Flash Preview",
        wire: WireApi::OpenAiCompletions,
        context_window: 128_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (0.5, 3.0, 0.05, 0.0),
    },
    CatalogRecord {
        id: "gemini-3.1-pro-preview",
        name: "Gemini 3.1 Pro Preview",
        wire: WireApi::OpenAiCompletions,
        context_window: 200_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (2.0, 12.0, 0.2, 0.0),
    },
    CatalogRecord {
        id: "gemini-3.5-flash",
        name: "Gemini 3.5 Flash",
        wire: WireApi::OpenAiCompletions,
        context_window: 200_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (1.5, 9.0, 0.15, 0.0),
    },
    CatalogRecord {
        id: "gpt-4.1",
        name: "GPT-4.1",
        wire: WireApi::OpenAiCompletions,
        context_window: 128_000,
        max_tokens: 16_384,
        images: true,
        reasoning: false,
        thinking: NONE,
        compat: CHAT,
        pricing: (2.0, 8.0, 0.5, 0.0),
    },
    CatalogRecord {
        id: "gpt-5-mini",
        name: "GPT-5 Mini",
        wire: WireApi::OpenAiResponses,
        context_window: 264_000,
        max_tokens: 64_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW,
        compat: RESPONSES,
        pricing: (0.25, 2.0, 0.025, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.2",
        name: "GPT-5.2",
        wire: WireApi::OpenAiResponses,
        context_window: 400_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (1.75, 14.0, 0.175, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.2-codex",
        name: "GPT-5.2 Codex",
        wire: WireApi::OpenAiResponses,
        context_window: 400_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (1.75, 14.0, 0.175, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.3-codex",
        name: "GPT-5.3 Codex",
        wire: WireApi::OpenAiResponses,
        context_window: 1_000_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (1.75, 14.0, 0.175, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.4",
        name: "GPT-5.4",
        wire: WireApi::OpenAiResponses,
        context_window: 1_000_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (2.5, 15.0, 0.25, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.4-mini",
        name: "GPT-5.4 mini",
        wire: WireApi::OpenAiResponses,
        context_window: 400_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (0.75, 4.5, 0.075, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.4-nano",
        name: "GPT-5.4 nano",
        wire: WireApi::OpenAiResponses,
        context_window: 400_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (0.2, 1.25, 0.02, 0.0),
    },
    CatalogRecord {
        id: "gpt-5.5",
        name: "GPT-5.5",
        wire: WireApi::OpenAiResponses,
        context_window: 1_000_000,
        max_tokens: 128_000,
        images: true,
        reasoning: true,
        thinking: OFF_MINIMAL_LOW_XHIGH,
        compat: RESPONSES,
        pricing: (5.0, 30.0, 0.5, 0.0),
    },
    CatalogRecord {
        id: "kimi-k2.7-code",
        name: "Kimi K2.7 Code",
        wire: WireApi::OpenAiCompletions,
        context_window: 256_000,
        max_tokens: 32_000,
        images: true,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (0.95, 4.0, 0.19, 0.0),
    },
    CatalogRecord {
        id: "mai-code-1-flash-picker",
        name: "MAI-Code-1-Flash",
        wire: WireApi::OpenAiCompletions,
        context_window: 256_000,
        max_tokens: 128_000,
        images: false,
        reasoning: true,
        thinking: NONE,
        compat: CHAT,
        pricing: (0.75, 4.5, 0.075, 0.0),
    },
];

/// Exact static 25-model catalog from pi 0.80.6.
pub fn github_copilot_catalog() -> Vec<ModelInfo> {
    RECORDS.iter().map(catalog_model).collect()
}

fn catalog_model(record: &CatalogRecord) -> ModelInfo {
    let capabilities = ModelCapabilities::new(record.context_window, record.max_tokens)
        .with_images(record.images)
        .with_streaming(true)
        .with_thinking(record.reasoning);
    let mut thinking = if record.reasoning {
        ThinkingLevelMap::reasoning_default()
    } else {
        ThinkingLevelMap::disabled()
    };
    for (level, mapped) in record.thinking {
        thinking = thinking.with_mapping(
            *level,
            mapped.map_or(ThinkingLevelMapping::Unsupported, |value| {
                ThinkingLevelMapping::Mapped(value.to_owned())
            }),
        );
    }
    let compat = match record.compat {
        Compat::Anthropic {
            eager_tool_input,
            adaptive_thinking,
            temperature,
        } => WireCompat::AnthropicMessages(AnthropicMessagesCompat {
            supports_eager_tool_input_streaming: eager_tool_input,
            force_adaptive_thinking: adaptive_thinking,
            supports_temperature: temperature,
        }),
        Compat::Chat => WireCompat::OpenAiCompletions(OpenAiCompletionsCompat {
            chat_completions_path: "/chat/completions".into(),
            ..Default::default()
        }),
        Compat::Responses => WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            responses_path: "/responses".into(),
            ..Default::default()
        }),
    };
    let (input, output, cache_read, cache_write) = record.pricing;
    ModelInfo::new(record.id, record.name, record.wire, capabilities)
        .with_base_url(GITHUB_COPILOT_DEFAULT_BASE_URL)
        .with_thinking_level_map(thinking)
        .with_compat(compat)
        .expect("catalog wire and compatibility match")
        .with_pricing(
            ModelPricing::try_new(
                Pricing {
                    input_cost_per_mtok: input,
                    output_cost_per_mtok: output,
                    cache_read_cost_per_mtok: cache_read,
                    cache_write_cost_per_mtok: cache_write,
                },
                vec![],
            )
            .expect("catalog pricing is non-negative"),
        )
        .expect("catalog pricing is valid")
}
