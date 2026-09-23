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

const upstreamSource = await readFile(resolve(projectRoot, 'src/components/agent-profiles/upstreamModels.ts'), 'utf8');
const upstreamCode = ts.transpileModule(upstreamSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { modelFromUpstream } = await import(`data:text/javascript;base64,${Buffer.from(upstreamCode).toString('base64')}`);
for (const agent of ['codex', 'opencode', 'claude_code']) {
  const model = modelFromUpstream(agent, { model_id: 'thinking-ultra', display_name: '' });
  assert.equal(model.thinking.supports_reasoning, null);
  assert.equal(model.thinking.supports_effort, null);
  assert.equal(model.thinking.selected, null);
  assert.deepEqual(model.thinking.options, []);
}
const advertised = { model_id: 'model', display_name: 'Model', supports_reasoning: false, supports_effort: true, effort_options: ['low', 'high'] };
assert.equal(modelFromUpstream('codex', advertised).thinking.supports_reasoning, false);
assert.deepEqual(modelFromUpstream('codex', advertised).thinking.options, ['low', 'high']);
assert.equal(modelFromUpstream('codex', advertised).thinking.selected, null);
assert.deepEqual(modelFromUpstream('opencode', advertised).thinking.options, [], 'effort levels must not turn into empty OpenCode variants');
console.log('upstream capability unknown/false states, advertised options and no automatic selection checks passed');

const variantsSource = await readFile(resolve(projectRoot, 'src/components/agent-profiles/variantValues.ts'), 'utf8');
const variantsCode = ts.transpileModule(variantsSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { parseVariantDrafts } = await import(`data:text/javascript;base64,${Buffer.from(variantsCode).toString('base64')}`);
const previousVariants = { high: { reasoningEffort: 'high', extra: { keep: true } } };
assert.deepEqual(parseVariantDrafts(['high'], {}, previousVariants), previousVariants);
assert.equal(parseVariantDrafts(['high'], { high: '{"reasoningEffort":"medium","extra":{"keep":true}}' }, previousVariants).high.reasoningEffort, 'medium');
for (const invalid of ['[]', 'null', '"high"', '{', '{"thinking":{"type":"enabled","budgetTokens":1023}}', '{"thinking":{"type":"enabled"}}']) {
  assert.throws(() => parseVariantDrafts(['high'], { high: invalid }, previousVariants));
}
assert.deepEqual(parseVariantDrafts([], {}, previousVariants), {});
console.log('Variant parameters: same-name edits, unknown fields, deletion and invalid object/budget checks passed');

const limitsSource = await readFile(resolve(projectRoot, 'src/components/agent-profiles/modelLimits.ts'), 'utf8');
const limitsCode = ts.transpileModule(limitsSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { parseModelLimits } = await import(`data:text/javascript;base64,${Buffer.from(limitsCode).toString('base64')}`);
assert.equal(parseModelLimits({ context: '', input: ' ', output: '' }), null);
assert.deepEqual(parseModelLimits({ context: '128000', input: '', output: '8192' }), { context: 128000, input: null, output: 8192 });
for (const invalid of ['', '0', '-1', '1.5', 'Infinity', '9007199254740992']) {
  assert.throws(() => parseModelLimits({ context: invalid, input: '', output: '8192' }));
}
console.log('Token limit inheritance, optional input, required context/output and safe integer checks passed');

const previewSource = await readFile(resolve(projectRoot, 'src/components/agent-profiles/effectiveConfig.ts'), 'utf8');
const previewCode = ts.transpileModule(previewSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { knownConfiguration, launchCommand } = await import(`data:text/javascript;base64,${Buffer.from(previewCode).toString('base64')}`);
const previewProfile = {
  agent: 'opencode', display_name: 'Demo', source: { path: { native: "/tmp/A B/quote'$(printf injected).jsonc", platform: 'linux' }, scope: 'user' }, default_state: {},
  managed: { default_provider_id: 'wrong', default_model_id: 'p/org/m', providers: [
    { provider_id: 'wrong', models: [], protocol: { native_protocol: 'unknown' } },
    { provider_id: 'p', credential: { secret: 'SECRET_SENTINEL' }, protocol: { native_protocol: 'anthropic_messages' }, models: [{ model_id: 'org/m',
      thinking: { reasoning_effort: 'low', thinking_mode: null, thinking_budget: null, selected: 'high', variant_values: { high: { effort: 'high', apiKey: 'SECRET_SENTINEL', thinking: { type: 'adaptive' } } } },
      modalities: { input: ['text', 'image'], output: ['text'] }, limits: { context: 128000, input: null, output: 8192 }
    }] }
  ] }
};
const preview = knownConfiguration(previewProfile);
assert.equal(preview.find(r => r.key === 'provider').value, 'p');
assert.deepEqual(preview.find(r => r.key === 'reasoningEffort'), { key: 'reasoningEffort', value: 'high', source: 'variant', variant: 'high', overridden: 'low' });
assert.equal(preview.find(r => r.key === 'thinkingMode').source, 'variant');
assert.equal(preview.find(r => r.key === 'inputLimit').source, 'inherited');
assert.ok(!JSON.stringify(preview).includes('SECRET_SENTINEL'));
previewProfile.managed.providers[1].models[0].thinking.variant_values.high.disabled = true;
assert.equal(knownConfiguration(previewProfile).find(r => r.key === 'reasoningEffort').value, 'low');
const { mkdtempSync, writeFileSync, chmodSync, rmSync } = await import('node:fs');
const { tmpdir } = await import('node:os');
const { execFileSync } = await import('node:child_process');
const probeDir = mkdtempSync(resolve(tmpdir(), 'vibehub-command-'));
try {
  const probe = resolve(probeDir, 'opencode');
  writeFileSync(probe, '#!/bin/sh\nprintf \'%s\' "$OPENCODE_CONFIG"\n'); chmodSync(probe, 0o755);
  const command = launchCommand(previewProfile, null).command;
  const output = execFileSync('/bin/sh', ['-c', command], { env: { ...process.env, PATH: probeDir + ':' + process.env.PATH }, encoding: 'utf8' });
  assert.equal(output, previewProfile.source.path.native, 'quoted POSIX path is passed literally to the child');
} finally { rmSync(probeDir, { recursive: true }); }
const windowsProfile = structuredClone(previewProfile);
windowsProfile.source.path = { native: "C:\\Users\\A B\\it's.jsonc", platform: 'windows' };
const windowsCommand = launchCommand(windowsProfile, { kind: 'host', platform: 'windows' });
assert.equal(windowsCommand.shell, 'powershell');
assert.ok(windowsCommand.command.includes("it''s.jsonc"));
assert.ok(windowsCommand.command.includes('finally { $env:OPENCODE_CONFIG = $previousOpenCodeConfig }'));
windowsProfile.source.path.native = '\\\\wsl.localhost\\Ubuntu\\home\\Case User\\opencode.jsonc';
const wslCommand = launchCommand(windowsProfile, { kind: 'wsl', platform: 'linux', distribution: 'Ubuntu' });
assert.ok(wslCommand.command.includes("'OPENCODE_CONFIG=/home/Case User/opencode.jsonc'"));
assert.equal(launchCommand(windowsProfile, { kind: 'wsl', platform: 'linux', distribution: 'Debian' }).command, null);
console.log('Preview source/override/credential exclusion and Linux child execution, Windows quoting and WSL path checks passed');
const actionSource = await readFile(resolve(projectRoot, 'src/components/agent-profiles/profileActions.ts'), 'utf8');
const actionModule = ts.transpileModule(actionSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { profileForAction } = await import(`data:text/javascript;base64,${Buffer.from(actionModule).toString('base64')}`);
const stored = { revision: 1 }, saved = { revision: 2 };
let writes = 0;
assert.equal(await profileForAction(stored, false, async () => { writes++; return saved; }), stored);
assert.equal(writes, 0, 'clean launch does not rewrite config');
assert.equal(await profileForAction(stored, true, async () => { writes++; return saved; }), saved, 'dirty action receives the saved snapshot');
assert.equal(await profileForAction(stored, true, async () => null), null, 'failed save cannot authorize an action');
assert.equal(await profileForAction(null, true, async () => { throw Error('should not save'); }), null);
let finishSave;
const delayed = profileForAction(stored, true, () => new Promise(resolve => { finishSave = resolve; }));
let resolved = false;
delayed.then(() => { resolved = true; });
await Promise.resolve();
assert.equal(resolved, false, 'action waits for persistence');
finishSave(saved);
assert.equal(await delayed, saved);
console.log('agent profile save-before-action ordering, failure and clean-state checks passed');
