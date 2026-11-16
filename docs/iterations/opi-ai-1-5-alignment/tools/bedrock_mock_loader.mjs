const mock = new URL("./bedrock_mock_runtime.mjs", import.meta.url);

export async function resolve(specifier, context, nextResolve) {
  if (specifier === "@aws-sdk/client-bedrock-runtime") {
    return { url: mock.href, shortCircuit: true };
  }
  return nextResolve(specifier, context);
}
