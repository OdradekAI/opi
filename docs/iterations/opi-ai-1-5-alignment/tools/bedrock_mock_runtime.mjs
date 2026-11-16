export class BedrockRuntimeServiceException extends Error {}

export const StopReason = {
  END_TURN: "end_turn",
  STOP_SEQUENCE: "stop_sequence",
  MAX_TOKENS: "max_tokens",
  MODEL_CONTEXT_WINDOW_EXCEEDED: "model_context_window_exceeded",
  TOOL_USE: "tool_use",
};
export const CachePointType = { DEFAULT: "default" };
export const CacheTTL = { ONE_HOUR: "one_hour" };
export const ConversationRole = { ASSISTANT: "assistant", USER: "user" };
export const ImageFormat = { JPEG: "jpeg", PNG: "png", GIF: "gif", WEBP: "webp" };
export const ToolResultStatus = { ERROR: "error", SUCCESS: "success" };

export class ConverseStreamCommand {
  constructor(input) {
    this.input = input;
  }
}

export class BedrockRuntimeClient {
  constructor(config) {
    this.config = config;
    this.middlewareStack = { add() {} };
  }

  async send() {
    const fixture = globalThis.__opiBedrockFixture;
    const events = fixture.events.map(({ type, payload }) => ({ [type]: payload }));
    return {
      $metadata: { httpStatusCode: 200, requestId: "fixture-request" },
      stream: (async function* () {
        for (const event of events) {
          yield event;
        }
      })(),
    };
  }
}
