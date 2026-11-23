import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
assert.ok(snapshot && fixturePath && process.argv.length === 4, "snapshot and fixture required");
const load = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ createModels, createProvider }, chat] = await Promise.all([
	load("packages/ai/dist/models.js"),
	load("packages/ai/dist/api/openai-completions.js"),
]);
const fixture = JSON.parse(await readFile(resolve(fixturePath), "utf8"));

const server = createServer(async (_request, response) => {
	response.writeHead(200, { "content-type": "text/event-stream" });
	response.end(fixture.sse);
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
try {
	const model = {
		id: "gpt-4o",
		name: "OpenAI Chat response observer fixture",
		provider: "openai",
		api: "openai-completions",
		baseUrl: `http://127.0.0.1:${server.address().port}/v1`,
		reasoning: false,
		input: ["text"],
		contextWindow: 128000,
		maxTokens: 4096,
		cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
	};
	const models = createModels();
	models.setProvider(createProvider({
		id: "openai",
		models: [model],
		api: { stream: chat.stream, streamSimple: chat.streamSimple },
		auth: { apiKey: { name: "fixture", resolve: async () => ({ auth: { apiKey: "fixture-key" } }) } },
	}));
	let callbackCalls = 0;
	const observerStatuses = [];
	const onResponse = async (response) => {
		observerStatuses.push(response.status);
		if (callbackCalls++ === 0) throw new Error("response observer fixture failure");
	};
	const context = { messages: [{ role: "user", content: "observer fixture", timestamp: 1 }] };
	const first = models.streamSimple(model, context, { apiKey: "fixture-key", maxRetries: 0, onResponse });
	const firstEvents = [];
	for await (const event of first) firstEvents.push(event.type);
	const firstResult = await first.result();
	const second = models.streamSimple(model, context, { apiKey: "fixture-key", maxRetries: 0, onResponse });
	const secondEvents = [];
	for await (const event of second) secondEvents.push(event.type);
	const secondResult = await second.result();
	const trace = {
		case: fixture.id,
		observer_statuses: observerStatuses,
		first: { stop: firstResult.stopReason, event_types: firstEvents },
		second: {
			stop: secondResult.stopReason,
			event_types: secondEvents,
			usage: { input: secondResult.usage.input, output: secondResult.usage.output, total: secondResult.usage.totalTokens ?? null },
		},
		requests: 2,
	};
	assert.deepEqual(trace.observer_statuses, fixture.expected.observer_statuses);
	assert.equal(trace.first.stop, fixture.expected.first_stop);
	assert.deepEqual(trace.first.event_types, fixture.expected.first_event_types);
	assert.equal(trace.second.stop, fixture.expected.second_stop);
	assert.deepEqual(trace.second.usage, fixture.expected.second_usage);
	console.log(JSON.stringify(trace));
} finally {
	server.closeAllConnections();
	await new Promise((done) => server.close(done));
}
