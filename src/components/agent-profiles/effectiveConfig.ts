import type { AgentProfileDocument, RuntimeTarget } from '@/v3/contracts/generated/agent-profile';

export type PreviewRow = { key: string; value: unknown; source: 'file' | 'model' | 'variant' | 'inherited'; variant?: string; overridden?: unknown };
const record = (value: unknown): Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};

export function knownConfiguration(profile: AgentProfileDocument): PreviewRow[] {
    const managed = profile.managed;
    const selected = managed.default_model_id;
    const provider = profile.agent === 'opencode'
        ? managed.providers.find(p => p.provider_id === selected?.split('/')[0])
        : managed.providers.find(p => p.provider_id === managed.default_provider_id);
    const model = provider?.models.find(m => selected === m.model_id || selected === `${provider.provider_id}/${m.model_id}`);
    const rows: PreviewRow[] = [];
    const add = (key: string, value: unknown, source: PreviewRow['source'] = 'model') => rows.push({ key, value: value ?? null, source: value == null ? 'inherited' : source });
    add('provider', provider?.provider_id, 'file');
    add('model', selected, 'file');
    add('protocol', provider?.protocol.native_protocol, 'file');
    add('smallModel', managed.small_model_id, 'file');
    add('variant', profile.agent === 'opencode' ? model?.thinking.selected : null, 'file');
    add('reasoningEffort', profile.agent === 'opencode' ? model?.thinking.reasoning_effort : model?.thinking.selected);
    add('thinkingMode', model?.thinking.thinking_mode);
    add('thinkingBudget', model?.thinking.thinking_budget);
    add('inputModalities', model?.modalities?.input);
    add('outputModalities', model?.modalities?.output);
    add('context', model?.limits?.context);
    add('inputLimit', model?.limits?.input);
    add('outputLimit', model?.limits?.output);
    // Preview only managed, non-credential values. Never spread provider options,
    // credentials or arbitrary variant keys into the preview.
    const variantName = profile.agent === 'opencode' ? model?.thinking.selected : null;
    const variant = record(variantName ? model?.thinking.variant_values?.[variantName] : null);
    if (variantName && variant.disabled !== true) {
        const thinking = record(variant.thinking);
        const effortKey = provider?.protocol.native_protocol === 'anthropic_messages' ? 'effort' : 'reasoningEffort';
        for (const [key, value] of [['reasoningEffort', variant[effortKey]], ['thinkingMode', thinking.type], ['thinkingBudget', thinking.budgetTokens]] as const) {
            if (key === 'thinkingBudget' ? typeof value !== 'number' : typeof value !== 'string') continue;
            const row = rows.find(row => row.key === key)!;
            row.overridden = row.value; row.value = value; row.source = 'variant'; row.variant = variantName;
        }
    }
    return rows;
}

export function launchCommand(profile: AgentProfileDocument, target: RuntimeTarget | null): { command: string | null; shell: 'powershell' | 'posix'; error?: string } {
    const wsl = target?.kind === 'wsl';
    const windows = wsl || (target?.platform ?? profile.source.path.platform) === 'windows';
    const shell = windows ? 'powershell' : 'posix';
    const quote = (value: string) => windows ? `'${value.replace(/'/g, "''")}'` : `'${value.replace(/'/g, "'\\''")}'`;
    let path = profile.source.path.native;
    if (wsl) {
        const unc = path.replace(/^\\\\\?\\UNC\\/i, '\\\\').match(/^\\\\(?:wsl\$|wsl\.localhost)\\([^\\]+)(\\.*)?$/i);
        if (unc && unc[1].toLowerCase() === target.distribution?.toLowerCase()) path = (unc[2] || '/').replace(/\\/g, '/');
        else if (!path.startsWith('/') || path.startsWith('//')) return { command: null, shell, error: 'runtimePathMismatch' };
        if (!target.distribution) return { command: null, shell, error: 'runtimePathMismatch' };
    }
    const executable = profile.agent === 'opencode' ? 'opencode' : profile.agent === 'claude_code' ? 'claude' : 'codex';
    const args = profile.agent === 'claude_code' && profile.source.scope !== 'user' ? ['--setting-sources', '', '--settings', path]
        : profile.agent === 'codex' && !profile.default_state.is_default && profile.source.scope !== 'user' ? ['--profile-v2', profile.source.profile_name || profile.display_name] : [];
    if (wsl) {
        const command = ['wsl.exe', '--distribution', target.distribution!, '--exec', ...(profile.agent === 'opencode' ? ['env', `OPENCODE_CONFIG=${path}`] : []), executable, ...args];
        return { command: '& ' + command.map(quote).join(' '), shell };
    }
    if (profile.agent === 'opencode') {
        return { command: windows
            ? `& { $previousOpenCodeConfig = $env:OPENCODE_CONFIG; try { $env:OPENCODE_CONFIG = ${quote(path)}; & 'opencode' } finally { $env:OPENCODE_CONFIG = $previousOpenCodeConfig } }`
            : `env ${quote(`OPENCODE_CONFIG=${path}`)} opencode`, shell };
    }
    return { command: (windows ? '& ' : '') + [executable, ...args].map(quote).join(' '), shell };
}
