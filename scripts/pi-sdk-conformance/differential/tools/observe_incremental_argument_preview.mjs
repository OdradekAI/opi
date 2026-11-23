import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) {
  console.error("node observe_incremental_argument_preview.mjs <frozen-pi> <fixture>");
  process.exit(2);
}
const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ ToolCallArgumentsStream, StreamingJsonParser }, { repairJson }] = await Promise.all([
  from("packages/ai/dist/utils/streaming-json.js"), from("packages/ai/dist/utils/json-parse.js"),
]);
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));

function describe(value) {
  const key_order = [], non_finite = [], negative_zero_paths = [];
  function visit(value, path) {
    if (typeof value === "number" && !Number.isFinite(value)) {
      non_finite.push({ path, kind: String(value) });
    } else if (Object.is(value, -0)) {
      negative_zero_paths.push(path);
    } else if (Array.isArray(value)) {
      value.forEach((child, index) => visit(child, [...path, index]));
    } else if (value !== null && typeof value === "object") {
      key_order.push({ path, keys: Object.keys(value) });
      for (const key of Object.keys(value)) visit(value[key], [...path, key]);
    }
  }
  visit(value, []);
  return { key_order, non_finite, negative_zero_paths };
}

function operations(test) {
  const tools = new Map();
  const rows = [];
  const retained = [];
  for (const action of test.operations) {
    const tool = action.tool ?? 0;
    if (!tools.has(tool)) tools.set(tool, { parser: new ToolCallArgumentsStream(), preview: {} });
    const state = tools.get(tool);
    let suffix = null;
    switch (action.op) {
      case "append": state.preview = state.parser.append(action.text); break;
      case "replace": {
        const result = state.parser.replace(action.text);
        suffix = result.delta ?? null;
        state.preview = result.arguments;
        break;
      }
      case "snapshot": case "finish": break;
      default: assert.fail("unknown operation");
    }
    retained.push(state.preview);
    const row = { op: action.op, tool, raw: state.parser.text,
      preview: structuredClone(state.preview), suffix, ...describe(state.preview) };
    if (action.op === "finish") {
      const final = state.parser.finish();
      const valid = final.argumentsError === undefined;
      let raw = state.parser.text.trim() === "" ? "{}" : state.parser.text;
      if (valid) {
        try { JSON.parse(raw); } catch { raw = repairJson(raw); }
      }
      row.final = { valid, raw, value: valid ? final.arguments : null,
        non_finite: valid ? describe(final.arguments).non_finite : [] };
    }
    rows.push(row);
  }
  assert.deepEqual(retained, rows.map((row) => row.preview));
  return { name: test.name, rows, retained };
}

function largeArray(items) {
  const parser = new ToolCallArgumentsStream();
  // The lower-level parser controls repeated snapshot reads explicitly.
  const reference = new StreamingJsonParser();
  let preview = parser.append('{"a":[');
  reference.append('{"a":['); reference.snapshot();
  let credit = null;
  const lengths = [];
  for (let index = 0; index < items; index++) {
    const piece = index === 0 ? '"x"' : ',"x"';
    preview = parser.append(piece);
    reference.append(piece);
    assert.deepEqual(reference.snapshot(), preview);
    lengths.push(preview.a.length);
    if (credit === null && preview.a.length !== index + 1) {
      const published = preview.a.length;
      const repeated = reference.snapshot().a.length;
      const same = parser.replace(parser.text);
      assert.equal(same.delta, "");
      const identical = same.arguments.a.length;
      const refreshed = parser.append("");
      reference.append("");
      assert.deepEqual(reference.snapshot(), refreshed);
      credit = { received: index + 1, published, repeated,
        identical_replace: identical, empty_append: refreshed.a.length };
    }
  }
  assert.equal(parser.append("]}").a.length, items);
  assert.equal(parser.finish().argumentsError, undefined);
  assert.notEqual(credit, null);
  return { lengths, credit, final_length: items };
}

function costs() {
  const rows = [];
  for (const { length, chunk } of fixture.long_strings) {
    for (const snapshots of [false, true]) {
      const parser = new StreamingJsonParser();
      parser.append('{"s":"');
      const text = "x".repeat(length);
      const start = process.hrtime.bigint();
      let visible = 0;
      let calls = 0;
      let retained;
      for (let index = 0; index < text.length; index += chunk) {
        parser.append(text.slice(index, index + chunk));
        if (snapshots) {
          const value = parser.snapshot();
          visible += value.s.length;
          calls++;
          retained ??= value;
        }
      }
      const elapsed = process.hrtime.bigint() - start;
      parser.append('"}');
      assert.equal(parser.snapshot().s, text);
      if (retained) assert.equal(retained.s, "x".repeat(chunk));
      rows.push({ length, chunk, snapshots, snapshot_calls: calls,
        returned_string_characters: visible, elapsed_ns: Number(elapsed) });
    }
  }
  return rows;
}

console.log(JSON.stringify({ cases: fixture.cases.map(operations),
  large_array: largeArray(fixture.large_array_items), cost_observations: costs(),
  cost_scope: "Pi snapshots structurally share data; Rust explicitly materializes owned JSON. Timings are observations, not equal-work performance acceptance." }));
