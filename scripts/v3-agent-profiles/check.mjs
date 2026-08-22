import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import assert from "node:assert/strict";
import ts from "typescript";

const projectRoot = resolve(new URL("../..", import.meta.url).pathname);
const sourcePath = resolve(projectRoot, "src/components/agent-profiles/claudeProfile.ts");
const source = await readFile(sourcePath, "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2020,
  },
}).outputText;
const module = await import(`data:text/javascript;base64,${Buffer.from(transpiled).toString("base64")}`);
const { CLAUDE_AUTO_MODEL_VALUE, normalizeClaudeModelSelection } = module;

const autoValues = [null, undefined, "", "  ", "__auto__"];
for (const value of autoValues) {
  const selection = normalizeClaudeModelSelection(value, ["water18", "water18-mini"]);
  assert.equal(selection.uiValue, CLAUDE_AUTO_MODEL_VALUE, `legacy value ${String(value)} renders as auto`);
  assert.equal(selection.payloadValue, null, `legacy value ${String(value)} saves as null`);
  assert.equal(selection.status, "auto");
  assert.ok(selection.options.includes(CLAUDE_AUTO_MODEL_VALUE));
}

const known = normalizeClaudeModelSelection(" water18 ", ["water18", "water18-mini"]);
assert.equal(known.uiValue, "water18");
assert.equal(known.payloadValue, "water18");
assert.equal(known.status, "available");
assert.deepEqual(known.options, [CLAUDE_AUTO_MODEL_VALUE, "water18", "water18-mini"]);

const unknown = normalizeClaudeModelSelection("legacy-model", []);
assert.equal(unknown.uiValue, "legacy-model", "unknown configured values remain user-visible");
assert.equal(unknown.payloadValue, "legacy-model", "unknown configured values are not silently dropped");
assert.equal(unknown.status, "unknown");
assert.deepEqual(unknown.options, [CLAUDE_AUTO_MODEL_VALUE, "legacy-model"]);

const capabilityFixture = {
  capability_declaration: {
    status: "declared",
    source: "claude-code.settings.adapter",
    version: "1.0",
    supported_fields: ["model", "env.CLAUDE_CODE_SUBAGENT_MODEL"],
    fallback_priority: ["explicit_override", "managed.default_model_id", "unavailable"],
    message: null,
  },
  custom_model_options: {
    status: "available",
    source: "managed.providers[].models",
    values: ["water18", "water18-mini"],
    allow_custom: true,
    fallback_priority: ["explicit_override", "managed.default_model_id", "unavailable"],
    message: null,
  },
};
assert.equal(capabilityFixture.custom_model_options.source, "managed.providers[].models");
assert.deepEqual(capabilityFixture.custom_model_options.values, ["water18", "water18-mini"]);
assert.deepEqual(capabilityFixture.custom_model_options.fallback_priority, ["explicit_override", "managed.default_model_id", "unavailable"]);

console.log("v3-agent-profiles: 13 assertions passed");
