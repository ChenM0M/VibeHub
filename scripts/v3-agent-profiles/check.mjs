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

// Check the shipped UI and declarations, not a hand-written ideal fixture.
for (const locale of ['en', 'zh', 'zh-TW']) {
  const strings = JSON.parse(await readFile(resolve(projectRoot, `src/locales/${locale}.json`), 'utf8')).agentProfiles.claudeCompatibility;
  for (const key of ['autoOption', 'description', 'aliasGroupHint', 'haikuModelHint']) {
    assert.doesNotMatch(strings[key], /follow.*main model|project.*main model|沿用.*主模型|投影.*主模型|上面的|上方的|above/i);
  }
}
const panel = await readFile(resolve(projectRoot, 'src/components/agent-profiles/AgentProfilesPanel.tsx'), 'utf8');
assert.ok(!panel.includes('id="claude-small-model"'), 'one editor for the single Haiku/background key');
assert.ok(panel.includes('claude_native_resolution'));
const backend = await readFile(resolve(projectRoot, 'src-tauri/src/agent_profiles.rs'), 'utf8');
const capability = backend.slice(backend.indexOf('fn claude_schema_capability('), backend.indexOf('fn default_state_value('));
assert.ok(capability.includes('claude_native_resolution'));
assert.ok(!capability.includes('managed.default_model_id'));
console.log('v3-agent-profiles: selection normalization, unknown models, canonical Haiku editor, three locales and capability declarations passed');

const defaultSource = await readFile(resolve(projectRoot, 'src/components/agent-profiles/modelDefaults.ts'), 'utf8');
const defaultCode = ts.transpileModule(defaultSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { modelIsDefault, updateDefaultAfterModelEdit } = await import(`data:text/javascript;base64,${Buffer.from(defaultCode).toString('base64')}`);
const profile = { agent: 'codex', managed: { default_provider_id: 'primary', default_model_id: 'same', providers: [
    { provider_id: 'primary', models: [{ model_id: 'same', enabled: true }] },
    { provider_id: 'secondary', models: [{ model_id: 'same', enabled: true }] },
] } };
assert.equal(modelIsDefault(profile, 'secondary', 'same'), false, 'same model ID in another provider is not default');
for (const enabled of [true, false]) {
    const draft = structuredClone(profile);
    updateDefaultAfterModelEdit(draft, 'secondary', 'same', enabled, false);
    assert.equal(draft.managed.default_provider_id, 'primary');
    assert.equal(draft.managed.default_model_id, 'same', 'editing or disabling non-default model preserves choice');
}
const renamed = structuredClone(profile);
updateDefaultAfterModelEdit(renamed, 'primary', 'renamed', true, true);
assert.equal(renamed.managed.default_model_id, 'renamed');
const disabled = structuredClone(profile);
disabled.managed.providers[0].models[0].enabled = false;
updateDefaultAfterModelEdit(disabled, 'primary', 'same', false, true);
assert.equal(disabled.managed.default_provider_id, 'secondary');
assert.equal(disabled.managed.default_model_id, 'same', 'Codex replacement remains a bare model ID');
console.log('model edit default preservation, duplicate provider IDs, rename and disable checks passed');
