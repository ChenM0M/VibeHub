// Real Claude CLI, disposable config/cwd, fake credentials and loopback API only.
// No live provider requests, user settings, keychain, hooks or plugins are needed.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createServer } from 'node:http';
import { spawn, execFileSync } from 'node:child_process';

const cli = process.env.CLAUDE_TEST_BINARY || 'claude';
const root = await mkdtemp(join(tmpdir(), 'vibehub-claude-runtime-'));
const config = join(root, 'config');
const cwd = join(root, 'project');
await mkdir(config);
await mkdir(join(cwd, '.claude'), { recursive: true });
const observations = [];
const server = createServer(async (req, res) => {
  let body = '';
  for await (const chunk of req) body += chunk;
  const input = body ? JSON.parse(body) : {};
  if (req.url.startsWith('/v1/messages') && !req.url.includes('count_tokens')) {
    observations.push({ model: input.model });
    const message = { id: 'msg_fixture', type: 'message', role: 'assistant', model: input.model,
      content: [], stop_reason: null, stop_sequence: null, usage: { input_tokens: 1, output_tokens: 0 } };
    res.writeHead(200, { 'Content-Type': 'text/event-stream' });
    const emit = (type, data) => res.write(`event: ${type}\ndata: ${JSON.stringify({ type, ...data })}\n\n`);
    emit('message_start', { message });
    emit('content_block_start', { index: 0, content_block: { type: 'text', text: '' } });
    emit('content_block_delta', { index: 0, delta: { type: 'text_delta', text: 'fixture-ok' } });
    emit('content_block_stop', { index: 0 });
    emit('message_delta', { delta: { stop_reason: 'end_turn', stop_sequence: null }, usage: { output_tokens: 1 } });
    emit('message_stop', {});
    res.end();
  } else {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(req.url.includes('count_tokens') ? { input_tokens: 1 } : { data: [] }));
  }
});
await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
const base = `http://127.0.0.1:${server.address().port}`;
const env = Object.fromEntries(['PATH', 'HOME', 'TMPDIR', 'SystemRoot'].filter(k => process.env[k]).map(k => [k, process.env[k]]));
Object.assign(env, { CLAUDE_CONFIG_DIR: config, ANTHROPIC_API_KEY: 'fixture-not-a-secret', ANTHROPIC_BASE_URL: base,
  DISABLE_TELEMETRY: '1', DISABLE_ERROR_REPORTING: '1', DISABLE_AUTOUPDATER: '1', CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: '1' });
const a = join(root, 'profile a.settings.json');
const b = join(root, 'profile b.settings.json');
const native = join(config, 'settings.json');
const project = join(cwd, '.claude/settings.json');
await writeFile(a, JSON.stringify({ model: 'claude-sonnet-4-6', env: { ANTHROPIC_DEFAULT_HAIKU_MODEL: 'claude-haiku-4-5' } }));
await writeFile(b, JSON.stringify({ model: 'claude-haiku-4-5' }));
await writeFile(native, JSON.stringify({ model: 'claude-opus-4-6' }));
await writeFile(project, JSON.stringify({ model: 'claude-opus-4-6' }));
const files = [a, b, native, project];
const before = await Promise.all(files.map(f => readFile(f, 'utf8')));
try {
  console.log(`Claude ${execFileSync(cli, ['--version'], { env, encoding: 'utf8' }).trim()}`);
  for (const [file, expected] of [[a, 'claude-sonnet-4-6'], [b, 'claude-haiku-4-5'], [a, 'claude-sonnet-4-6']]) {
    const start = observations.length;
    const args = ['--setting-sources', '', '--settings', file, '--bare', '-p', 'Reply fixture-ok only.',
      '--tools', '', '--max-turns', '1', '--no-session-persistence', '--strict-mcp-config', '--mcp-config', '{"mcpServers":{}}'];
    const result = await new Promise((resolve, reject) => {
      const child = spawn(cli, args, { env, cwd, stdio: ['ignore', 'pipe', 'pipe'] });
      let stdout = '', stderr = '';
      child.stdout.on('data', x => stdout += x);
      child.stderr.on('data', x => stderr += x);
      const timer = setTimeout(() => { child.kill('SIGKILL'); reject(new Error('Claude fixture timed out')); }, 30000);
      child.once('error', e => { clearTimeout(timer); reject(e); });
      child.once('close', code => { clearTimeout(timer); resolve({ code, stdout, stderr }); });
    });
    assert.equal(result.code, 0, `CLI exited ${result.code}: ${result.stderr.slice(0, 500)}`);
    assert.match(result.stdout, /fixture-ok/);
    const models = observations.slice(start).map(x => x.model);
    assert.ok(models.includes(expected), `Expected ${expected}, observed ${models}`);
    assert.ok(!models.includes('claude-opus-4-6'), 'native/project settings leaked into isolated launch');
    assert.deepEqual(await Promise.all(files.map(f => readFile(f, 'utf8'))), before, 'settings changed after startup');
    console.log(JSON.stringify({ profile: file, expected, observedModels: models, filesUnchanged: true }));
  }
  console.log('PASS: real Claude CLI isolated A/B/A restarts, path with spaces, model selection and unchanged settings; --bare, loopback mock API, not native GUI/Windows acceptance.');
} finally {
  server.closeAllConnections();
  server.close();
  console.log(`Disposable evidence: ${root}`);
}
