import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node observe_deferred_cancel_route.mjs --snapshot <path>");
  process.exit(2);
}
const load = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ createModels }, { fauxProvider, fauxAssistantMessage }] = await Promise.all([
  load("packages/ai/dist/models.js"),
  load("packages/ai/dist/providers/faux.js"),
]);
const apiKey = "in-memory-probe-key";
const changes = [
  ["provider-mismatch", { provider: "other-provider" }],
  ["model-mismatch", { modelId: "other-model" }],
  ["api-mismatch", { api: "other-api" }],
  ["unknown-job-id", { id: "deferred-missing" }],
  ["matching-handle", {}],
];

async function observe(name, change) {
  const faux = fauxProvider({
    provider: "lifecycle-differential", api: "scripted-deferred", models: [{ id: "mock-model" }],
    deferred: { pendingFetches: 0, pollAfterMs: 25 },
  });
  const base = faux.provider;
  const submissionModel = faux.getModel();
  const auth = base.auth.apiKey;
  const models = createModels();
  let authCalls = 0;
  models.setProvider({
    ...base,
    auth: { ...base.auth, apiKey: { ...auth, async resolve(input) {
      assert.equal(input.credential?.key, apiKey);
      authCalls++;
      return auth.resolve(input);
    } } },
  });

  faux.setResponses([fauxAssistantMessage("ready")]);
  const submitted = await models.completeSimple(submissionModel, { messages: [] }, { deferred: true, apiKey });
  assert.equal(submitted.stopReason, "deferred");
  const saved = JSON.parse(JSON.stringify({
    model: { provider: submissionModel.provider, modelId: submissionModel.id },
    handle: submitted.deferred,
  }));
  const original = saved.handle;
  const model = models.getModel(saved.model.provider, saved.model.modelId);
  assert.ok(model);
  assert.match(original.id, /^deferred:/);
  const input = { ...original, ...change };
  const normalized = (handle) => ({ ...handle, id: handle.id === original.id ? "deferred-1" : handle.id });
  const route = { provider: model.provider, modelId: model.id, api: model.api };
  const beforeCancel = authCalls;
  let dispatchedRoute;
  const result = await models.cancelDeferred(model, input, {
    apiKey,
    onResponse(_response, requestModel) {
      dispatchedRoute = { provider: requestModel.provider, modelId: requestModel.id, api: requestModel.api };
    },
  });
  assert.equal(result, undefined);
  assert.deepEqual(dispatchedRoute, route);
  assert.equal(authCalls - beforeCancel, 1);
  assert.deepEqual(faux.state.cancelledDeferred, [input]);
  assert.equal(faux.state.deferredFetchCount, 0);
  const cancel = {
    auth_calls: authCalls - beforeCancel,
    dispatch_count: faux.state.cancelledDeferred.length,
    dispatches: faux.state.cancelledDeferred.map(normalized),
    result_delivery: "fulfilled-void", typed_cause: null, error_message: null,
  };

  const beforeRefetch = authCalls;
  const message = await models.fetchDeferred(model, original, { wait: 0, apiKey });
  assert.equal(authCalls - beforeRefetch, 1);
  assert.equal(faux.state.deferredFetchCount, 1);
  assert.equal(faux.state.callCount, 1);
  assert.equal(faux.state.cancelledDeferred.length, 1);
  const text = message.content.filter(({ type }) => type === "text").map(({ text }) => text).join("");
  const missing = name === "unknown-job-id";
  assert.equal(message.stopReason, missing ? "stop" : "error");
  assert.equal(text, missing ? "ready" : "");
  if (!missing) assert.equal(message.errorMessage, `Faux deferred response was cancelled: ${original.id}`);
  const refetch_original = {
    auth_calls: authCalls - beforeRefetch, dispatch_count: faux.state.deferredFetchCount,
    result_delivery: "fulfilled-message", typed_cause: null, stop: message.stopReason, text,
    error_message: message.errorMessage?.replace(original.id, "deferred-1") ?? null,
  };
  return {
    name, route, original: normalized(original), request_handle: normalized(input),
    direct_validation: null, cancel, refetch_original,
  };
}

const rows = [];
for (const [name, change] of changes) rows.push(await observe(name, change));
console.log(JSON.stringify(rows));
